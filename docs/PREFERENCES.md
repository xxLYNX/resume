# Workspace preferences

**File:** `<workspace>/preferences.toml` (default `workspace/preferences.toml`).  
**Example:** [`preferences.example.toml`](../preferences.example.toml).  
**Code:** `theme::preferences` (`crates/theme/src/preferences.rs`).

TOML only. There is no `preferences.yaml`. `settings/jobdesk.yaml` stays the legacy build layout; this change does not convert it.

## Defaults when the file is absent

| Key | Default |
|-----|---------|
| `resume.default_themes.catalog_preview` | `classic` |
| `resume.default_themes.showcase` | `classic` |
| `resume.default_themes.application` | `ats-classic` |

Reading a missing file does not create one. A directory at that path is an error. Theme ids are checked against `themes/*/theme.toml`. Before an update changes any value, the existing file is validated: `schema_version` must be integer `1` when present, and every stored theme id must be installed. A bad type, an unsupported version, an unknown theme, or malformed TOML returns an error that names the setting and leaves the file bytes unchanged. The update does not replace an invalid value in order to repair it.

A successful update locks `<workspace>/.preferences.toml.lock`, reads the file again, and writes a uniquely named temporary file in that directory (`create_new`, then `sync_all`). The temporary file is removed if the write or rename fails. A failure before `rename` reports that the original file was left unchanged. The rename is the replacement. If that rename succeeds and syncing the parent directory then fails, the error says the new file may already have been installed. Comments and keys that were not edited stay in the file.

## What this does not do

Saved preset `theme_id` values, submission manifests, and draft compositions are not rewritten.

Live application preview still uses the code constant `classic` (`store::DEFAULT_THEME_ID`). Applying `application = "ats-classic"` there now would change how existing applications render, because those drafts do not store a theme. That wiring belongs to the Settings UI milestone, which must:

- Offer these three defaults and no other document types.
- Write only through `set_default_theme`.
- Leave existing preset themes and publication `theme_id`s alone.
- Stay on localhost. No website deploy from this file.

Showcase publication (separate work, not started) must snapshot its theme at approve time. A later edit to `showcase` in this file must not alter that publication.

Backup: copy `preferences.toml` with `jobdesk.db`. It is workspace-local and not in git.
