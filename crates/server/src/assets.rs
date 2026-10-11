use std::path::PathBuf;

/// Root of jobdesk UI assets (`index.html`, `css/`, `js/`).
pub fn static_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static")
}
