# Showcase résumé

One Showcase per workspace. It is not an application, a preset, or a second copy of the catalog.

Schema **v18** adds `showcase` and `showcase_publications`. Schema **v19** adds optional `applications.draft_theme_id` for per-application themes (NULL renders as `classic`).

## J1 persistence and approval

Opening a database creates the singleton draft if it is missing. The first row uses theme `classic` and empty section items (not `default_composition`).

`save_showcase_draft` replaces `theme_id` and `draft_composition_json`. It does not change publication rows or `current_publication_id`.

`approve_showcase` (`render_tex`) captures the draft once, renders from the theme snapshot, and commits a publication when the captured `updated_at`, composition, theme, and pointer still match. Empty compositions, stale captures, and missing or mismatched PDFs are rejected without altering the current approved pointer.

Restore moves `current_publication_id` only; it does not rewrite the working draft.

## JobDesk UI (J2)

### Resume editor target

Portfolio **Showcase** switches to the Resume tab with target **Showcase (working draft)**. The same composition editor, section pins, theme selector, and PDF preview are used. The catalog is never silently copied into Showcase.

HTTP:

- `GET /api/showcase` — working draft, current publication, history list
- `POST /api/showcase/draft` — save `theme_id` + composition (validated theme)
- `POST /api/showcase/apply-default-theme` — set the **saved** draft theme to the configured Showcase default from `workspace/preferences.toml` (requires matching `updated_at`; does not change composition or publications)
- `POST /api/showcase/approve` — approve the **saved** draft (`updated_at` must match; server calls `render_tex::approve_showcase`)
- `POST /api/showcase/restore` — move current publication pointer
- `GET /api/showcase/publications/{id}/resume.pdf` — serve approved PDF with checksum check

Browser UX:

- **Approve** and **Use configured Showcase default theme** are disabled while the editor differs from the last saved draft.
- Approval asks for confirmation and relies on the PDF preview (including contact fields).
- History lists approved publications with view, restore pointer, and **copy composition to working draft** (separate from restore).

### Theme defaults vs saved themes

| Surface | What “default” means | What is stored |
|---------|----------------------|----------------|
| Settings → Showcase | Preference for *new* explicit applies via **Use configured Showcase default theme** | `preferences.toml` only; does not change the DB row when you save Settings |
| Showcase DB row at open | Explicit `classic` on the singleton | Stays until user saves a draft or runs **apply-default-theme** |
| Portfolio preview | Initial theme selector when themes load | Selector only until user previews/saves elsewhere |
| Applications | Default for **new** applications (API/CLI) | `applications.draft_theme_id`; NULL → render `classic` |

Changing a default in Settings does **not** retroactively change existing application drafts, submissions, Showcase approved publications, or presets.

### Settings

Header **Settings** edits `workspace/preferences.toml` slots `catalog_preview`, `showcase`, and `application` via `GET /api/preferences` and `POST /api/preferences/default-theme` (atomic TOML update, theme ids validated).

### Application themes (J2)

New applications receive `draft_theme_id` from an explicit API choice, clone inheritance, or the Applications default in preferences. Legacy rows with NULL keep rendering as `classic`.

`PATCH /api/applications/{id}` validates `draft_theme_id` even when composition is omitted; unknown themes are rejected.

`jobdesk build --application ID` (draft mode) uses the stored application theme when `--theme` is omitted. `--theme` overrides. Submitted mode still uses the canonical submission manifest theme.

### Header `location_label`

Optional header field (e.g. `Fairfax County, VA`) renders as a footnote-sized line under contact links in `classic` and `ats-classic`.

## Limitations (J2)

- No browser automation in CI; HTTP integration tests cover APIs. Full click-through of Portfolio → Showcase is documented for manual verification.
- No public website publisher, GitHub integration, or `jobdesk showcase export` (J3).