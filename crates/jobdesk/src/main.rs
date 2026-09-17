use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use render_tex::jobdesk_build_to_dir;
use server::{AppState, run as run_server};
use resolve::default_composition;
use store::{
    export_catalog_to_content, import_legacy_content, CreateApplicationRequest,
    ListApplicationsQuery, Store, UpdateApplicationRequest, ImportOptions, STAGING_CONTENT_DIR,
};

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
    },
    /// Application tracker (list, add, show, set status)
    Application {
        #[command(subcommand)]
        command: ApplicationCommands,
    },
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
enum ApplicationCommands {
    /// List applications (optional filters)
    List {
        #[arg(long)]
        q: Option<String>,
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
        Commands::Build { config, pdf } => {
            let store = Store::open(&db_path).context(
                "catalog database missing; run `jobdesk import` first (seed from content/)",
            )?;
            let config_path = resolve_path(&root, &config);
            let legacy: domain::LegacyConfig =
                serde_yaml::from_str(&std::fs::read_to_string(&config_path)?)?;
            let out_dir = root.join(&legacy.output.directory);

            let result = jobdesk_build_to_dir(&store, &root, &config_path, &out_dir, pdf)?;
            println!("Wrote {}", result.tex_path.display());
            if let Some(pdf_path) = result.pdf_path {
                println!("Wrote {}", pdf_path.display());
            }
            println!("{}", result.content_hash);
        }
        Commands::Application { command } => {
            let store = Store::open(&db_path).context(
                "catalog database missing; run `jobdesk import` first",
            )?;
            match command {
                ApplicationCommands::List { q, status, tag } => {
                    let apps = store.list_applications(&ListApplicationsQuery {
                        q,
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
                            "{}\t{}\t{}\t{}{}{}",
                            app.id, app.status, app.company, app.title, tags, url
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
                            source: None,
                            status,
                            tags: tag,
                            clone_from_application_id: None,
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
                    store.update_application(
                        &id,
                        UpdateApplicationRequest {
                            status: Some(status),
                            ..Default::default()
                        },
                    )?;
                    println!("ok");
                }
            }
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
                AppState {
                    root: root.clone(),
                    config_path,
                    preview_dir,
                    db_path: db_path.clone(),
                    workspace_dir: workspace.clone(),
                },
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
