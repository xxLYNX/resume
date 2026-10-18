mod filters;
mod legacy_build;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use domain::{LegacyConfig, MonthDisplay};
use minijinja::{Environment, Value};
use resolve::ResolvedSections;
use theme::{load_theme_info, theme_dir_for_id};

pub use legacy_build::{
    hash_tex, jobdesk_build_aligned, jobdesk_build_composition_to_dir,
    jobdesk_build_resolved_to_dir, jobdesk_build_to_dir, write_build_to_dir, LegacyBuildResult,
};

/// Apply `themes/{theme_id}/` template paths and read date display policy from `theme.toml`.
pub fn load_themed_build_config(
    project_root: &Path,
    config_path: &Path,
    theme_id: &str,
) -> Result<(LegacyConfig, MonthDisplay)> {
    let config_text = fs::read_to_string(config_path)
        .with_context(|| format!("read config {}", config_path.display()))?;
    let mut config: LegacyConfig =
        serde_yaml::from_str(&config_text).context("parse build config YAML")?;
    let info = load_theme_info(project_root, theme_id)
        .with_context(|| format!("load theme `{theme_id}`"))?;
    config.preamble = format!("themes/{theme_id}/preamble.tex");
    for section in &mut config.sections {
        if let Some(sec) = info.sections.get(&section.name) {
            section.template = format!("themes/{theme_id}/{}", sec.template);
            section.section_title = sec.title.clone();
        }
    }
    Ok((config, MonthDisplay::parse(&info.month_display)))
}

fn theme_id_from_preamble(preamble_rel: &str) -> Option<String> {
    let path = Path::new(preamble_rel);
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(String::from)
}

fn register_theme_macros(
    env: &mut Environment,
    project_root: &Path,
    preamble_rel: &str,
) -> Result<()> {
    let Some(theme_id) = theme_id_from_preamble(preamble_rel) else {
        return Ok(());
    };
    let info = load_theme_info(project_root, &theme_id)?;
    let theme_dir = theme_dir_for_id(project_root, &theme_id);
    for (name, rel) in &info.macros {
        let path = theme_dir.join(rel);
        let src = fs::read_to_string(&path)
            .with_context(|| format!("read theme macro {}", path.display()))?;
        env.add_template_owned(name.clone(), src)
            .with_context(|| format!("compile theme macro `{name}`"))?;
    }
    Ok(())
}

pub fn build_resume_tex(
    project_root: &Path,
    config: &LegacyConfig,
    resolved: &ResolvedSections,
    month_display: MonthDisplay,
    composition_section_order: Option<&[String]>,
) -> Result<String> {
    let mut body = String::new();
    let mut env = Environment::new();
    filters::register_resume_filters(&mut env, month_display);
    register_theme_macros(&mut env, project_root, &config.preamble)?;

    for section_name in section_render_order(config, composition_section_order) {
        let Some(section) = config.sections.iter().find(|s| s.name == section_name) else {
            continue;
        };
        if !section.enabled {
            continue;
        }

        let Some(yaml_value) = resolved.get(&section.name) else {
            // Composition may omit whole sections (content picker); skip rather than fail.
            continue;
        };

        let template_path = project_root.join(&section.template);
        let template_src = fs::read_to_string(&template_path)
            .with_context(|| format!("read template {}", template_path.display()))?;

        let template_name = section.template.replace('/', "_");
        env.add_template_owned(template_name.clone(), template_src)
            .with_context(|| format!("compile template {}", template_path.display()))?;
        let tmpl = env
            .get_template(&template_name)
            .with_context(|| format!("load template {}", template_path.display()))?;

        let mut ctx_value = yaml_value.clone();
        if let Some(title) = &section.section_title {
            if let Some(obj) = ctx_value.as_object_mut() {
                obj.entry("section_title".to_string())
                    .or_insert_with(|| serde_json::Value::String(title.clone()));
            }
        }
        let ctx = Value::from_serialize(&ctx_value);
        let rendered = tmpl
            .render(ctx)
            .with_context(|| format!("render section {}", section.name))?;
        body.push_str(&rendered);
        body.push_str("\n\n\n");
    }

    let preamble_path = project_root.join(&config.preamble);
    let preamble = fs::read_to_string(&preamble_path)
        .with_context(|| format!("read preamble {}", preamble_path.display()))?;

    Ok(format!(
        "{preamble}\n\n\\begin{{document}}\n\n{body}\\end{{document}}\n"
    ))
}

/// Composable sections follow draft order; config-only sections (e.g. toolbelt) keep YAML placement.
fn section_render_order(config: &LegacyConfig, composition_order: Option<&[String]>) -> Vec<String> {
    use std::collections::HashSet;

    let config_names: Vec<String> = config
        .sections
        .iter()
        .filter(|s| s.enabled)
        .map(|s| s.name.clone())
        .collect();
    let Some(comp) = composition_order else {
        return config_names;
    };
    let config_set: HashSet<String> = config_names.iter().cloned().collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for key in comp {
        if config_set.contains(key) && seen.insert(key.clone()) {
            out.push(key.clone());
        }
    }
    for name in config_names {
        if seen.insert(name.clone()) {
            out.push(name);
        }
    }
    out
}

/// Normalized document for golden tests (whitespace inside `{…}` and blank lines).
pub fn normalize_tex(tex: &str) -> String {
    let mut out = String::new();
    for line in tex.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    collapse_brace_whitespace(&out)
}

fn collapse_brace_whitespace(tex: &str) -> String {
    let mut result = String::with_capacity(tex.len());
    let chars: Vec<char> = tex.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == ' ' {
            result.push('{');
            i += 2;
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }
        if chars[i] == ' ' && i + 1 < chars.len() && chars[i + 1] == '}' {
            result.push('}');
            i += 2;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use store::{import_legacy_content, ImportOptions, Store};

    #[test]
    fn jobdesk_build_matches_golden_fixture() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let golden_path = root.join("tests/fixtures/golden_resume_catalog.tex");
        if !golden_path.exists() {
            eprintln!("skip: no golden fixture at {}", golden_path.display());
            return;
        }

        let db_path = root.join("target/test-jobdesk.db");
        let _ = std::fs::remove_file(&db_path);
        let store = Store::open(&db_path).unwrap();
        import_legacy_content(&root, &store, ImportOptions::default()).unwrap();
        // PDF skills come from element evidence links only (not skills.main); fixture DB has none yet.

        let built = jobdesk_build_to_dir(
            &store,
            &root,
            &root.join("settings/jobdesk.yaml"),
            &root.join("target/test-output"),
            false,
            "classic",
        )
        .unwrap();

        let golden = fs::read_to_string(&golden_path).unwrap();
        assert_eq!(
            normalize_tex(&built.tex),
            normalize_tex(&golden),
            "catalog resolve build diverged from golden fixture"
        );
    }

}
