use std::path::Path;

use anyhow::Result;
use domain::Composition;
use resolve::{ResolveConfig, ResolvedSections};
use store::Store;
use theme::load_theme_info;

pub fn resolve_composition_checked(
    store: &Store,
    project_root: &Path,
    theme_id: &str,
    composition: &Composition,
) -> Result<ResolvedSections> {
    let config = load_theme_info(project_root, theme_id)
        .map(|t| ResolveConfig::from_theme(&t))
        .unwrap_or_default();
    resolve::resolve_composition_with_config(store, composition, &config)
}
