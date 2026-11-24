//! Workspace preferences (`preferences.toml`). Not a résumé and not `settings/jobdesk.yaml`.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::list_theme_ids;

pub const PREFERENCES_SCHEMA_VERSION: i64 = 1;
pub const PREFERENCES_FILE_NAME: &str = "preferences.toml";

const DEFAULT_CATALOG_PREVIEW: &str = "classic";
const DEFAULT_SHOWCASE: &str = "classic";
const DEFAULT_APPLICATION: &str = "ats-classic";
const LOCK_FILE_NAME: &str = ".preferences.toml.lock";

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_schema_version")]
    pub schema_version: i64,
    #[serde(default)]
    pub resume: ResumePreferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResumePreferences {
    #[serde(default)]
    pub default_themes: DefaultThemes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultThemes {
    #[serde(default = "default_catalog_preview")]
    pub catalog_preview: String,
    #[serde(default = "default_showcase")]
    pub showcase: String,
    #[serde(default = "default_application")]
    pub application: String,
}

impl Default for DefaultThemes {
    fn default() -> Self {
        Self {
            catalog_preview: default_catalog_preview(),
            showcase: default_showcase(),
            application: default_application(),
        }
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            resume: ResumePreferences::default(),
        }
    }
}

fn default_schema_version() -> i64 {
    PREFERENCES_SCHEMA_VERSION
}
fn default_catalog_preview() -> String {
    DEFAULT_CATALOG_PREVIEW.to_string()
}
fn default_showcase() -> String {
    DEFAULT_SHOWCASE.to_string()
}
fn default_application() -> String {
    DEFAULT_APPLICATION.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultThemeSlot {
    CatalogPreview,
    Showcase,
    Application,
}

impl DefaultThemeSlot {
    fn key(self) -> &'static str {
        match self {
            Self::CatalogPreview => "catalog_preview",
            Self::Showcase => "showcase",
            Self::Application => "application",
        }
    }
}

pub fn preferences_path(workspace: &Path) -> PathBuf {
    workspace.join(PREFERENCES_FILE_NAME)
}

/// Load preferences. A missing file yields defaults and does not create a file.
/// Malformed TOML, a bad schema, or an unknown theme id is an error.
/// A directory at the preferences path is an error, not a missing file.
pub fn load_preferences(project_root: &Path, workspace: &Path) -> Result<Preferences> {
    let path = preferences_path(workspace);
    ensure_preferences_location(&path)?;
    if !path.is_file() {
        let prefs = Preferences::default();
        validate_theme_ids(project_root, &prefs)?;
        return Ok(prefs);
    }
    let bytes = fs::read(&path).with_context(|| format!("read preferences {}", path.display()))?;
    preferences_from_bytes(project_root, &path, &bytes)
}

/// Set one default theme.
///
/// An existing file is validated before any of its values change. Validation
/// failure leaves the bytes on disk untouched. The update then takes an
/// advisory lock, reads the file again, and replaces it by renaming a uniquely
/// named temporary file in the same directory.
pub fn set_default_theme(
    project_root: &Path,
    workspace: &Path,
    slot: DefaultThemeSlot,
    theme_id: &str,
) -> Result<Preferences> {
    let path = preferences_path(workspace);
    ensure_preferences_location(&path)?;
    if path.is_file() {
        let original =
            fs::read(&path).with_context(|| format!("read preferences {}", path.display()))?;
        preferences_from_bytes(project_root, &path, &original)?;
    }
    ensure_theme_installed(project_root, theme_id).with_context(|| {
        format!(
            "requested theme `{theme_id}` for resume.default_themes.{} — preferences file not modified",
            slot.key()
        )
    })?;

    let _lock = PreferencesLock::acquire(workspace)?;
    ensure_preferences_location(&path)?;
    let mut doc = if path.is_file() {
        let original =
            fs::read(&path).with_context(|| format!("read preferences {}", path.display()))?;
        let text = std::str::from_utf8(&original).with_context(|| {
            format!(
                "preferences {} is not UTF-8 — file not modified",
                path.display()
            )
        })?;
        preferences_from_bytes(project_root, &path, &original)?;
        text.parse::<DocumentMut>().with_context(|| {
            format!(
                "malformed preferences {} — file not modified",
                path.display()
            )
        })?
    } else {
        default_document()
    };

    if doc.get("schema_version").is_none() {
        doc["schema_version"] = toml_edit::value(PREFERENCES_SCHEMA_VERSION);
    }
    let themes =
        doc["resume"].or_insert(toml_edit::table())["default_themes"].or_insert(toml_edit::table());
    themes[slot.key()] = toml_edit::value(theme_id);

    let rendered = doc.to_string();
    let prefs = parse_preferences(project_root, &path, &rendered)?;
    atomic_write(&path, &rendered)?;
    Ok(prefs)
}

fn preferences_from_bytes(project_root: &Path, path: &Path, bytes: &[u8]) -> Result<Preferences> {
    let raw = std::str::from_utf8(bytes).with_context(|| {
        format!(
            "preferences {} is not UTF-8 — file not modified",
            path.display()
        )
    })?;
    parse_preferences(project_root, path, raw)
}

fn parse_preferences(project_root: &Path, path: &Path, raw: &str) -> Result<Preferences> {
    let doc: DocumentMut = raw.parse().with_context(|| {
        format!(
            "malformed preferences {} — file not modified",
            path.display()
        )
    })?;
    validate_document(project_root, path, &doc)?;
    let prefs: Preferences = toml::from_str(raw).with_context(|| {
        format!(
            "malformed preferences {} — file not modified",
            path.display()
        )
    })?;
    if prefs.schema_version != PREFERENCES_SCHEMA_VERSION {
        bail!(
            "preferences {} key schema_version is {}; this build understands {PREFERENCES_SCHEMA_VERSION} only — file not modified",
            path.display(),
            prefs.schema_version
        );
    }
    validate_theme_ids(project_root, &prefs)?;
    Ok(prefs)
}

fn validate_document(project_root: &Path, path: &Path, doc: &DocumentMut) -> Result<()> {
    if let Some(version) = doc.get("schema_version") {
        match version.as_integer() {
            Some(n) if n == PREFERENCES_SCHEMA_VERSION => {}
            Some(n) => bail!(
                "preferences {} key schema_version is {n}; this build understands {PREFERENCES_SCHEMA_VERSION} only — file not modified",
                path.display()
            ),
            None => bail!(
                "preferences {} key schema_version has type {}; expected integer {PREFERENCES_SCHEMA_VERSION} — file not modified",
                path.display(),
                version.type_name()
            ),
        }
    }

    let Some(resume) = doc.get("resume") else {
        return Ok(());
    };
    if resume.as_table_like().is_none() {
        bail!(
            "preferences {} key resume has type {}; expected table — file not modified",
            path.display(),
            resume.type_name()
        );
    }
    let Some(themes) = resume.get("default_themes") else {
        return Ok(());
    };
    if themes.as_table_like().is_none() {
        bail!(
            "preferences {} key resume.default_themes has type {}; expected table — file not modified",
            path.display(),
            themes.type_name()
        );
    }
    for key in ["catalog_preview", "showcase", "application"] {
        let Some(value) = themes.get(key) else {
            continue;
        };
        let Some(id) = value.as_str() else {
            bail!(
                "preferences {} key resume.default_themes.{key} has type {}; expected string — file not modified",
                path.display(),
                value.type_name()
            );
        };
        ensure_theme_installed(project_root, id).with_context(|| {
            format!(
                "preferences {} key resume.default_themes.{key} is \"{id}\" — file not modified",
                path.display()
            )
        })?;
    }
    Ok(())
}

fn validate_theme_ids(project_root: &Path, prefs: &Preferences) -> Result<()> {
    for (key, id) in [
        (
            "catalog_preview",
            prefs.resume.default_themes.catalog_preview.as_str(),
        ),
        ("showcase", prefs.resume.default_themes.showcase.as_str()),
        (
            "application",
            prefs.resume.default_themes.application.as_str(),
        ),
    ] {
        ensure_theme_installed(project_root, id)
            .with_context(|| format!("preferences key resume.default_themes.{key} is \"{id}\""))?;
    }
    Ok(())
}

fn ensure_theme_installed(project_root: &Path, theme_id: &str) -> Result<()> {
    let installed = list_theme_ids(project_root)?;
    if !installed.iter().any(|id| id == theme_id) {
        bail!(
            "theme `{theme_id}` is not installed (found: {})",
            installed.join(", ")
        );
    }
    Ok(())
}

fn ensure_preferences_location(path: &Path) -> Result<()> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => Ok(()),
        Ok(_) => bail!(
            "preferences {} exists but is not a file — not modified",
            path.display()
        ),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("inspect preferences {}", path.display())),
    }
}

fn default_document() -> DocumentMut {
    let raw = format!(
        "schema_version = {PREFERENCES_SCHEMA_VERSION}\n\n[resume.default_themes]\ncatalog_preview = \"{DEFAULT_CATALOG_PREVIEW}\"\nshowcase = \"{DEFAULT_SHOWCASE}\"\napplication = \"{DEFAULT_APPLICATION}\"\n"
    );
    raw.parse().expect("built-in preferences document")
}

struct PreferencesLock {
    file: fs::File,
}

impl PreferencesLock {
    fn acquire(workspace: &Path) -> Result<Self> {
        fs::create_dir_all(workspace)
            .with_context(|| format!("create preferences directory {}", workspace.display()))?;
        let path = workspace.join(LOCK_FILE_NAME);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .with_context(|| format!("open preferences lock {}", path.display()))?;
        file.lock()
            .with_context(|| format!("lock preferences {}", path.display()))?;
        Ok(Self { file })
    }
}

impl Drop for PreferencesLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

struct TempCleanup {
    path: PathBuf,
    armed: bool,
}

impl Drop for TempCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .context("preferences path has no directory")?;
    let target_name = path
        .file_name()
        .context("preferences path has no file name")?;
    let (tmp_path, mut tmp) = exclusive_temp(parent, target_name)?;
    let mut cleanup = TempCleanup {
        path: tmp_path,
        armed: true,
    };
    tmp.write_all(contents.as_bytes())
        .with_context(|| format!("write {}", cleanup.path.display()))?;
    tmp.sync_all()
        .with_context(|| format!("sync {}", cleanup.path.display()))?;
    drop(tmp);
    fs::rename(&cleanup.path, path)
        .with_context(|| format!("replace {} — original left unchanged", path.display()))?;
    cleanup.armed = false;
    sync_parent_after_replace(path, parent)?;
    Ok(())
}

fn exclusive_temp(dir: &Path, target_name: &OsStr) -> Result<(PathBuf, fs::File)> {
    let mut last = None;
    for _ in 0..64 {
        let path = dir.join(temp_file_name(target_name));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => last = Some(err),
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("create temporary preferences file in {}", dir.display())
                });
            }
        }
    }
    Err(anyhow::Error::from(
        last.expect("retry loop records an AlreadyExists error"),
    ))
    .context("could not allocate a unique temporary preferences file")
}

fn temp_file_name(target_name: &OsStr) -> std::ffi::OsString {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let mut name = std::ffi::OsString::from(".");
    name.push(target_name);
    name.push(format!(".{}.{}.{}.tmp", std::process::id(), nanos, seq));
    name
}

fn sync_parent_after_replace(path: &Path, parent: &Path) -> Result<()> {
    sync_dir(parent).with_context(|| {
        format!(
            "replaced {}; syncing directory {} failed — the new file may already have been installed",
            path.display(),
            parent.display()
        )
    })
}

fn sync_dir(dir: &Path) -> Result<()> {
    let file = fs::File::open(dir).with_context(|| format!("open directory {}", dir.display()))?;
    file.sync_all()
        .with_context(|| format!("sync directory {}", dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    const VALID: &str = concat!(
        "# owner note\n",
        "schema_version = 1\n",
        "\n",
        "[resume.default_themes]\n",
        "catalog_preview = \"classic\"\n",
        "showcase = \"classic\"\n",
        "application = \"classic\"\n",
        "\n",
        "[owner]\n",
        "keep = true\n",
    );

    fn fixture_root() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for id in ["classic", "ats-classic"] {
            let dir = root.path().join("themes").join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("theme.toml"), "version = 1\n").unwrap();
        }
        root
    }

    fn assert_no_temps(dir: &Path) {
        for entry in fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.ends_with(".tmp") && name != ".preferences.toml.part",
                "leftover temporary file {name}"
            );
        }
    }

    fn reject(
        root: &Path,
        dir: &Path,
        original: &str,
        slot: DefaultThemeSlot,
        theme: &str,
        needles: &[&str],
    ) {
        let path = preferences_path(dir);
        fs::write(&path, original).unwrap();
        let before = fs::read(&path).unwrap();
        let err = set_default_theme(root, dir, slot, theme).expect_err("update should fail");
        let msg = format!("{err:#}");
        for needle in needles {
            assert!(msg.contains(needle), "{msg}");
        }
        assert_eq!(fs::read(&path).unwrap(), before, "{msg}");
        assert!(load_preferences(root, dir).is_err(), "{msg}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!dir.join(LOCK_FILE_NAME).exists(), "{msg}");
        assert_no_temps(dir);
    }

    #[test]
    fn missing_file_uses_defaults_and_does_not_create_it() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        let prefs = load_preferences(root.path(), dir.path()).unwrap();
        assert_eq!(prefs.resume.default_themes.catalog_preview, "classic");
        assert_eq!(prefs.resume.default_themes.showcase, "classic");
        assert_eq!(prefs.resume.default_themes.application, "ats-classic");
        assert!(!preferences_path(dir.path()).exists());
        assert_no_temps(dir.path());
    }

    #[test]
    fn preferences_directory_is_not_a_missing_file() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        let path = preferences_path(dir.path());
        fs::create_dir(&path).unwrap();
        let load_err = load_preferences(root.path(), dir.path()).unwrap_err();
        let load_msg = format!("{load_err:#}");
        assert!(load_msg.contains("not a file"), "{load_msg}");
        let set_err = set_default_theme(
            root.path(),
            dir.path(),
            DefaultThemeSlot::Showcase,
            "classic",
        )
        .unwrap_err();
        let set_msg = format!("{set_err:#}");
        assert!(set_msg.contains("not a file"), "{set_msg}");
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    }

    #[test]
    fn malformed_file_is_not_overwritten() {
        let root = fixture_root();
        reject(
            root.path(),
            tempfile::tempdir().unwrap().path(),
            "schema_version = [\n",
            DefaultThemeSlot::Showcase,
            "classic",
            &["malformed", "not modified"],
        );
    }

    #[test]
    fn schema_version_wrong_type_is_not_repaired() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        reject(
            root.path(),
            dir.path(),
            "schema_version = \"1\"\n",
            DefaultThemeSlot::Application,
            "ats-classic",
            &["schema_version", "string", "not modified"],
        );
        let dir = tempfile::tempdir().unwrap();
        reject(
            root.path(),
            dir.path(),
            "schema_version = 1.0\n",
            DefaultThemeSlot::Application,
            "ats-classic",
            &["schema_version", "float", "not modified"],
        );
    }

    #[test]
    fn unsupported_schema_version_is_not_repaired() {
        let root = fixture_root();
        reject(
            root.path(),
            tempfile::tempdir().unwrap().path(),
            "schema_version = 2\n[resume.default_themes]\nshowcase = \"classic\"\n",
            DefaultThemeSlot::Showcase,
            "classic",
            &["schema_version", "is 2", "not modified"],
        );
    }

    #[test]
    fn unknown_theme_is_not_replaced() {
        let root = fixture_root();
        let original = concat!(
            "# do not repair\n",
            "schema_version = 1\n",
            "\n",
            "[resume.default_themes]\n",
            "catalog_preview = \"classic\"\n",
            "showcase = \"gone\"\n",
            "application = \"classic\"\n",
        );
        reject(
            root.path(),
            tempfile::tempdir().unwrap().path(),
            original,
            DefaultThemeSlot::Showcase,
            "classic",
            &["resume.default_themes.showcase", "gone", "not modified"],
        );
    }

    #[test]
    fn unknown_theme_in_another_slot_blocks_update() {
        let root = fixture_root();
        let original = concat!(
            "schema_version = 1\n",
            "[resume.default_themes]\n",
            "catalog_preview = \"missing-theme\"\n",
            "showcase = \"classic\"\n",
            "application = \"classic\"\n",
        );
        reject(
            root.path(),
            tempfile::tempdir().unwrap().path(),
            original,
            DefaultThemeSlot::Application,
            "ats-classic",
            &[
                "resume.default_themes.catalog_preview",
                "missing-theme",
                "not modified",
            ],
        );
    }

    #[test]
    fn unknown_requested_theme_does_not_create_a_file() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        let err = set_default_theme(
            root.path(),
            dir.path(),
            DefaultThemeSlot::Showcase,
            "missing",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("missing"), "{msg}");
        assert!(msg.contains("not modified"), "{msg}");
        assert!(!preferences_path(dir.path()).exists());
        assert_no_temps(dir.path());
    }

    #[test]
    fn update_keeps_comments_and_unrelated_keys() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        let path = preferences_path(dir.path());
        fs::write(&path, VALID).unwrap();
        set_default_theme(
            root.path(),
            dir.path(),
            DefaultThemeSlot::Application,
            "ats-classic",
        )
        .unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("# owner note"), "{raw}");
        assert!(raw.contains("keep = true"), "{raw}");
        assert!(raw.contains("application = \"ats-classic\""), "{raw}");
        assert!(raw.contains("showcase = \"classic\""), "{raw}");
        assert!(!path.with_file_name(".preferences.toml.part").exists());
        assert_no_temps(dir.path());
    }

    #[test]
    fn failed_write_preserves_original_bytes() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        let path = preferences_path(dir.path());
        let original = VALID.as_bytes();
        fs::write(&path, original).unwrap();
        fs::File::create(dir.path().join(LOCK_FILE_NAME)).unwrap();
        let original_perms = fs::metadata(dir.path()).unwrap().permissions();
        let mut readonly = original_perms.clone();
        readonly.set_readonly(true);
        struct Restore<'a>(&'a Path, fs::Permissions);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                let _ = fs::set_permissions(self.0, self.1.clone());
            }
        }
        let _restore = Restore(dir.path(), original_perms);
        fs::set_permissions(dir.path(), readonly).unwrap();

        let err = set_default_theme(
            root.path(),
            dir.path(),
            DefaultThemeSlot::Application,
            "ats-classic",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("temporary preferences file") || msg.contains("Permission denied"),
            "{msg}"
        );
        assert!(
            !msg.contains("may already have been installed"),
            "{msg}"
        );
        assert_eq!(fs::read(&path).unwrap(), original, "{msg}");
        drop(_restore);
        assert_no_temps(dir.path());
    }

    #[test]
    fn concurrent_update_re_reads_after_lock() {
        let root = fixture_root();
        let dir = tempfile::tempdir().unwrap();
        let path = preferences_path(dir.path());
        fs::write(&path, VALID).unwrap();
        let lock = PreferencesLock::acquire(dir.path()).unwrap();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let started = Arc::new(Barrier::new(2));
        let root_path = root.path().to_path_buf();
        let workspace = dir.path().to_path_buf();
        let done_flag = done.clone();
        let started_thread = started.clone();
        let worker = thread::spawn(move || {
            started_thread.wait();
            set_default_theme(
                &root_path,
                &workspace,
                DefaultThemeSlot::Application,
                "ats-classic",
            )
            .unwrap();
            done_flag.store(true, Ordering::SeqCst);
        });
        started.wait();
        thread::sleep(Duration::from_millis(300));
        assert!(
            !done.load(Ordering::SeqCst),
            "set_default_theme ran without waiting for the preferences lock"
        );
        let while_locked = VALID.replace("showcase = \"classic\"", "showcase = \"ats-classic\"");
        fs::write(&path, &while_locked).unwrap();
        drop(lock);
        worker.join().unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("showcase = \"ats-classic\""), "{raw}");
        assert!(raw.contains("application = \"ats-classic\""), "{raw}");
        assert!(raw.contains("# owner note"), "{raw}");
        assert!(raw.contains("keep = true"), "{raw}");
        assert_no_temps(dir.path());
    }

    #[test]
    fn directory_sync_failure_after_replace_names_the_installed_file() {
        let missing = Path::new("/tmp/jobdesk-preferences-missing-dir");
        let err = sync_parent_after_replace(Path::new("/tmp/preferences.toml"), missing)
            .expect_err("missing directory cannot be synced");
        let msg = format!("{err:#}");
        assert!(msg.contains("may already have been installed"), "{msg}");
        assert!(msg.contains("replaced"), "{msg}");
        assert!(!msg.contains("original left unchanged"), "{msg}");
    }
}
