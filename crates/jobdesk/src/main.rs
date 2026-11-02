use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use resolve::{default_composition, resolve_composition_with_config, ResolveConfig};
use theme::load_theme_info;
use server::{AppState, run as run_server};
use store::{
    default_ledger_download_path, export_catalog_to_content, import_legacy_content,
    CreateApplicationRequest, CreateElementRequest, ImportOptions, ListApplicationsQuery, Store,
    UpdateApplicationRequest, STAGING_CONTENT_DIR, DEFAULT_THEME_ID,
};

use render_tex::jobdesk_build_aligned;

mod mcp;

#[derive(Parser)]
#[command(name = "jobdesk", about = "Local job-search workspace (resume + tracker)")]
struct Cli {
    /// Project / repo root (defaults to current directory)
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Workspace directory for SQLite and artifacts
    #[arg(long, default_value = "workspace")]
    workspace: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import repo content/*.yaml into SQLite (skips existing elements unless --force)
    Import {
        /// Append YAML as new revisions for elements that already exist
        #[arg(long)]
        force: bool,
    },
    /// Export latest catalog revisions to workspace/staging/content/
    Export {
        /// Also copy exported YAML into repo content/ (overwrites seed files)
        #[arg(long)]
        mirror_repo_content: bool,
    },
    /// Build resume from catalog + settings/jobdesk.yaml
    Build {
        #[arg(long, default_value = "settings/jobdesk.yaml")]
        config: PathBuf,
        #[arg(long)]
        pdf: bool,
        /// Build using an application's draft or canonical submission (same as serve preview)
        #[arg(long)]
        application: Option<String>,
        /// With `--application`: `draft` (default) or `submitted`
        #[arg(long, default_value = "draft")]
        preview_mode: String,
        #[arg(long, default_value = "classic")]
        theme: String,
    },
    /// Portfolio catalog (create elements)
    Catalog {
        #[command(subcommand)]
        command: CatalogCommands,
    },
    /// Skill taxonomy (catalog nodes)
    Skills {
        #[command(subcommand)]
        command: SkillsCommands,
    },
    /// Application tracker (list, add, show, set status)
    Application {
        #[command(subcommand)]
        command: ApplicationCommands,
    },
    /// Model Context Protocol server (stdio) over workspace DB
    Mcp,
    /// Local web UI with live PDF preview
    Serve {
        #[arg(long, default_value = "settings/jobdesk.yaml")]
        config: PathBuf,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 3847)]
        port: u16,
        #[arg(long)]
        open: bool,
    },
}

#[derive(Subcommand)]
enum CatalogCommands {
    /// Create a portfolio element (revision 1) from a JSON payload file
    Add {
        kind: String,
        id: String,
        /// Element payload object (JSON), e.g. `{"company":"Acme","position":"Engineer",...}`
        #[arg(long)]
        payload: PathBuf,
    },
}

#[derive(Subcommand)]
enum SkillsCommands {
    /// Upsert skill nodes from a JSON array (`data/catalog_seed.json`)
    Import {
        #[arg(long, default_value = "data/catalog_seed.json")]
        file: PathBuf,
    },
    /// Merge framework_refs onto existing skill nodes (`data/framework_refs_seed.json`)
    ImportFrameworks {
        #[arg(long, default_value = "data/framework_refs_seed.json")]
        file: PathBuf,
    },
    /// Write docs/LEGACY_SKILLS_INVENTORY.md from skills.yaml and element technologies (no DB writes)
    LegacyInventory {
        #[arg(long, default_value = "docs/LEGACY_SKILLS_INVENTORY.md")]
        out: PathBuf,
    },
    /// Delete all asserted element_skill_links (portfolio evidence tags)
    ClearLinks {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        yes: bool,
    },
    /// Import O*NET text database (Technology/Software Skills + Skills.txt)
    ImportCatalog {
        #[arg(long)]
        onet_dir: PathBuf,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        replace: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        yes: bool,
    },
    /// Count skill_nodes by kind
    CatalogStats,
}

#[derive(Subcommand)]
enum ApplicationCommands {
    /// List applications (optional filters)
    List {
        #[arg(long)]
        q: Option<String>,
        #[arg(long)]
        phase: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Create a new application
    Add {
        company: String,
        title: String,
        #[arg(long)]
        posting_url: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        tag: Vec<String>,
        #[arg(long)]
        clone_from: Option<String>,
    },
    /// Show one application (JSON)
    Show { id: String },
    /// Update application status
    SetStatus { id: String, status: String },
    /// Update tracker fields (only flags you pass are changed)
    Update {
        id: String,
        #[arg(long)]
        company: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        posting_url: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        tag: Vec<String>,
    },
    /// Freeze draft composition to manifest (same as UI Submit)
    Submit { id: String },
    /// Delete an application and its submissions
    Delete { id: String },
    /// Export all applications to a ledger PDF (default: ~/Downloads/jobdesk-applications.pdf)
    ExportLedger {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Deterministic posting vs catalog/composition report
    Analyze {
        id: String,
        /// Print JSON (default: human summary)
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.root.canonicalize().unwrap_or(cli.root.clone());
    let workspace = root.join(&cli.workspace);
    let db_path = workspace.join("jobdesk.db");

    match cli.command {
        Commands::Import { force } => {
            let store = Store::open(&db_path)?;
            let stats = import_legacy_content(
                &root,
                &store,
                ImportOptions { force },
            )?;
            println!(
                "Catalog at {} — created {}, skipped {}, appended {}",
                db_path.display(),
                stats.created,
                stats.skipped,
                stats.appended
            );
        }
        Commands::Export {
            mirror_repo_content,
        } => {
            let store = Store::open(&db_path)?;
            let staging = workspace.join(STAGING_CONTENT_DIR);
            export_catalog_to_content(&store, &staging)?;
            println!("Exported catalog to {}", staging.display());
            if mirror_repo_content {
                export_catalog_to_content(&store, &root.join("content"))?;
                println!("Mirrored export to {}", root.join("content").display());
            }
        }
        Commands::Build {
            config,
            pdf,
            application,
            preview_mode,
            theme,
        } => {
            let store = Store::open(&db_path).context(
                "catalog database missing; run `jobdesk import` first (seed from content/)",
            )?;
            let config_path = resolve_path(&root, &config);
            let legacy: domain::LegacyConfig =
                serde_yaml::from_str(&std::fs::read_to_string(&config_path)?)?;
            let out_dir = root.join(&legacy.output.directory);

            let result = jobdesk_build_aligned(
                &store,
                &workspace,
                &root,
                &config_path,
                &out_dir,
                pdf,
                application.as_deref(),
                &preview_mode,
                &theme,
            )?;
            println!("Wrote {}", result.tex_path.display());
            if let Some(pdf_path) = result.pdf_path {
                println!("Wrote {}", pdf_path.display());
            }
            println!("{}", result.content_hash);
        }
        Commands::Skills { command } => {
            let store = Store::open(&db_path).context(
                "catalog database missing; run `jobdesk import` first (seed from content/)",
            )?;
            match command {
                SkillsCommands::Import { file } => {
                    let path = resolve_path(&root, &file);
                    let raw = std::fs::read_to_string(&path)
                        .with_context(|| format!("read {}", path.display()))?;
                    let nodes: Vec<domain::SkillNode> =
                        serde_json::from_str(&raw).context("parse skills JSON array")?;
                    let n = store.import_skill_nodes(&nodes)?;
                    println!("Imported {n} skill node(s)");
                }
                SkillsCommands::ImportFrameworks { file } => {
                    let path = resolve_path(&root, &file);
                    let raw = std::fs::read_to_string(&path)
                        .with_context(|| format!("read {}", path.display()))?;
                    let rows: Vec<domain::FrameworkRefsImportRow> =
                        serde_json::from_str(&raw).context("parse framework refs JSON array")?;
                    let n = store.import_framework_refs(&rows)?;
                    println!("Merged framework refs on {n} skill node(s)");
                }
                SkillsCommands::LegacyInventory { out } => {
                    let out_path = resolve_path(&root, &out);
                    store::write_legacy_skills_inventory(&store, &root, &out_path)?;
                    println!("Wrote {}", out_path.display());
                }
                SkillsCommands::ClearLinks { yes } => {
                    if !yes {
                        anyhow::bail!("pass --yes to delete all asserted portfolio evidence links");
                    }
                    let n = store.clear_asserted_element_skill_links()?;
                    println!("Deleted {n} asserted element skill link(s)");
                }
                SkillsCommands::ImportCatalog {
                    onet_dir,
                    replace,
                    yes,
                } => {
                    if replace && !yes {
                        anyhow::bail!("pass --yes with --replace to wipe the catalog before import");
                    }
                    let dir = resolve_path(&root, &onet_dir);
                    let started = std::time::Instant::now();
                    if replace {
                        store.reset_skill_catalog()?;
                        println!("Cleared skill catalog and all evidence links");
                    }
                    let result = onet_import::import_from_onet_dir(&dir)?;
                    let tool_n = store.import_skill_nodes_batch(&result.tools)?;
                    let comp_n = store.import_skill_nodes_batch(&result.competencies)?;
                    let edge_pairs: Vec<(String, String)> = result
                        .edges
                        .iter()
                        .map(|e| (e.tool_id.clone(), e.competency_id.clone()))
                        .collect();
                    let edge_n = store.replace_tool_competency_edges(&edge_pairs)?;
                    let elapsed = started.elapsed();
                    println!(
                        "Imported {tool_n} tools, {comp_n} competencies, {edge_n} tool→competency edges in {:.1}s",
                        elapsed.as_secs_f64()
                    );
                }
                SkillsCommands::CatalogStats => {
                    let stats = store.skill_catalog_stats()?;
                    if stats.is_empty() {
                        println!("Catalog empty — run `jobdesk skills import-catalog --onet-dir …`");
                    } else {
                        for (kind, n) in stats {
                            println!("{kind}: {n}");
                        }
                    }
                }
            }
        }
        Commands::Catalog { command } => {
            let store = Store::open(&db_path).context(
                "catalog database missing; run `jobdesk import` first (seed from content/)",
            )?;
            match command {
                CatalogCommands::Add { kind, id, payload } => {
                    let payload_path = resolve_path(&root, &payload);
                    let raw = std::fs::read_to_string(&payload_path)
                        .with_context(|| format!("read payload {}", payload_path.display()))?;
                    let payload_value: serde_json::Value =
                        serde_json::from_str(&raw).context("parse payload JSON")?;
                    let detail = store.create_element(&CreateElementRequest {
                        kind,
                        id,
                        payload: payload_value,
                        parent_element_id: None,
                    })?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&detail).context("serialize element")?
                    );
                }
            }
        }
        Commands::Application { command } => {
            let store = Store::open(&db_path).context(
                "catalog database missing; run `jobdesk import` first",
            )?;
            match command {
                ApplicationCommands::List { q, phase, status, tag } => {
                    let apps = store.list_applications(&ListApplicationsQuery {
                        q,
                        phase,
                        status,
                        tag,
                    })?;
                    for app in apps {
                        let tags = if app.tags.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", app.tags.join(", "))
                        };
                        let url = app
                            .posting_url
                            .as_deref()
                            .map(|u| format!("\t{u}"))
                            .unwrap_or_default();
                        println!(
                            "{}\t{}\t{}\t{}\t{}{}{}",
                            app.id, app.phase, app.status, app.company, app.title, tags, url
                        );
                    }
                }
                ApplicationCommands::Add {
                    company,
                    title,
                    posting_url,
                    status,
                    tag,
                    clone_from,
                } => {
                    let draft = if let Some(ref clone_id) = clone_from {
                        store
                            .get_application(clone_id)?
                            .map(|a| a.draft_composition)
                            .with_context(|| format!("unknown application `{clone_id}`"))?
                    } else {
                        default_composition(&store)?
                    };
                    let app = store.create_application(
                        CreateApplicationRequest {
                            company,
                            title,
                            posting_url,
                            posting_text: None,
                            source: None,
                            status,
                            tags: tag,
                            clone_from_application_id: None,
                            clone_from_preset_id: None,
                            employment_type: None,
                            work_mode: None,
                            location: None,
                            compensation: None,
                        },
                        draft,
                    )?;
                    println!("{}", app.id);
                }
                ApplicationCommands::Show { id } => {
                    let app = store
                        .get_application(&id)?
                        .with_context(|| format!("unknown application `{id}`"))?;
                    println!("{}", serde_json::to_string_pretty(&app)?);
                }
                ApplicationCommands::SetStatus { id, status } => {
                    use domain::{phase_and_workflow_from_status, DRAFT_WORKFLOW};
                    let (phase, workflow) = if status == DRAFT_WORKFLOW {
                        ("draft", DRAFT_WORKFLOW)
                    } else {
                        phase_and_workflow_from_status(&status)
                    };
                    store.update_application(
                        &id,
                        UpdateApplicationRequest {
                            phase: Some(phase.to_string()),
                            status: Some(workflow.to_string()),
                            ..Default::default()
                        },
                    )?;
                    println!("ok");
                }
                ApplicationCommands::Update {
                    id,
                    company,
                    title,
                    source,
                    posting_url,
                    status,
                    tag,
                } => {
                    let req = UpdateApplicationRequest {
                        company,
                        title,
                        source: source.map(Some),
                        posting_url: posting_url.map(Some),
                        status,
                        tags: if tag.is_empty() { None } else { Some(tag) },
                        ..Default::default()
                    };
                    store.update_application(&id, req)?;
                    println!("ok");
                }
                ApplicationCommands::Submit { id } => {
                    let app = store
                        .get_application(&id)?
                        .with_context(|| format!("unknown application `{id}`"))?;
                    let config = load_theme_info(&root, DEFAULT_THEME_ID)
                        .map(|t| ResolveConfig::from_theme(&t))
                        .unwrap_or_default();
                    let resolved_map =
                        resolve_composition_with_config(&store, &app.draft_composition, &config)?;
                    let mut resolved = BTreeMap::new();
                    for (k, v) in resolved_map {
                        resolved.insert(k, v);
                    }
                    let snap_dir = workspace.join("artifacts/theme_snapshots");
                    let theme_files_hash = theme::ensure_theme_snapshot(
                        &root,
                        DEFAULT_THEME_ID,
                        &snap_dir,
                    )?;
                    let sub = store.submit_application(
                        &id,
                        &workspace,
                        DEFAULT_THEME_ID,
                        Some(theme_files_hash),
                        &app.draft_composition,
                        resolved,
                    )?;
                    println!("{}", sub.content_hash);
                }
                ApplicationCommands::Delete { id } => {
                    if store.delete_application(&id)? {
                        println!("deleted");
                    } else {
                        anyhow::bail!("unknown application `{id}`");
                    }
                }
                ApplicationCommands::ExportLedger { output } => {
                    let dest = output
                        .map(Ok)
                        .unwrap_or_else(default_ledger_download_path)?;
                    let build_dir = workspace.join("artifacts/ledger-build");
                    let path = store.export_application_ledger_pdf(&build_dir, &dest)?;
                    println!("Wrote {}", path.display());
                }
                ApplicationCommands::Analyze { id, json } => {
                    let report = store.application_analyze(&id)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        println!(
                            "{} — {} @ {}",
                            report.application_id, report.title, report.company
                        );
                        println!(
                            "Posting tokens: {} · Objective: {:?}",
                            report.posting_token_count, report.objective_alignment.status
                        );
                        if !report.suggestions.is_empty() {
                            println!("\nSuggestions:");
                            for s in &report.suggestions {
                                println!("  • {s}");
                            }
                        }
                        if !report.skill_gaps.is_empty() {
                            println!("\nSkill gaps ({}):", report.skill_gaps.len());
                            for g in report.skill_gaps.iter().take(15) {
                                println!(
                                    "  • {} ({:?})",
                                    g.label,
                                    g.status
                                );
                            }
                        }
                    }
                }
            }
        }
        Commands::Mcp => {
            mcp::run_mcp(db_path)?;
        }
        Commands::Serve {
            config,
            host,
            port,
            open,
        } => {
            let config_path = resolve_path(&root, &config);
            let preview_dir = workspace.join("preview");
            let url = format!("http://{host}:{port}/");

            if open {
                let url_copy = url.clone();
                tokio::spawn(async move {
                    let _ = open_browser(&url_copy);
                });
            }

            run_server(
                AppState::new(
                    root.clone(),
                    config_path,
                    preview_dir,
                    db_path.clone(),
                    workspace.clone(),
                ),
                &host,
                port,
            )
            .await?;
        }
    }

    Ok(())
}

fn resolve_path(root: &PathBuf, path: &PathBuf) -> PathBuf {
    if path.is_relative() {
        root.join(path)
    } else {
        path.clone()
    }
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        if Command::new("xdg-open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
    #[cfg(target_os = "macos")]
    {
        if Command::new("open")
            .arg(url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
    #[cfg(target_os = "windows")]
    {
        if Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }
    }
    eprintln!("Could not open browser for {url}");
    Ok(())
}
