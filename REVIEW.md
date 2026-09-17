# Milestone A–B review (pre–Milestone C)

Snapshot before applications/submissions work. Tracks what we built, what we fixed in polish, and what Milestone C should not assume.

---

## What works well

| Area | Status |
|------|--------|
| Go legacy path | `config.yaml` + `templates/` untouched; `go run main.go` still valid |
| Rust path | `settings/jobdesk.yaml` + `themes/classic/` (MiniJinja) |
| Golden test | Catalog resolve + file-based `content/` both vs golden fixture |
| Catalog | Elements + append-only revisions in SQLite |
| UI | List/filter, revision picker, JSON edit, save (validated), preview from DB |
| Separation | Settings dir vs root config; classic vs legacy templates |

---

## Issues found (and fixes)

### Critical (polish pass)

1. **Re-import wiped revision history** — **Fix:** skip existing; `--force` appends revisions.
2. **Export order ≠ YAML order** — **Fix:** `sort_order` on import/export.
3. **CLI `build` ignored the catalog** — **Fix:** superseded by foundation pass (below).

### Foundation pass (three distinct issues)

1. **`content/` overwrite** — Preview/save no longer export to repo `content/`. **`jobdesk export`** writes `workspace/staging/content/`; optional `--mirror-repo-content` for intentional overwrite.
2. **Payload validation** — `domain::validate_payload` on import append, `create_revision`, and API save (400 on invalid JSON shape per `ElementKind`).
3. **Resolve pipeline** — Primary path: SQLite `default_composition` → `resolve_composition` → `build_resume_tex`. `settings/jobdesk.yaml` lists templates only; no content paths.

### Moderate (documented; deferred)

4. **ARCHITECTURE checkboxes** — Updated for B + foundation; C items still open.
5. **Go `-config` flag** — README/EXAMPLES still mention it; `main.go` hardcodes `config.yaml`.
6. **Skills association table** — Not started; skills still one blob (`skills.main`).
7. **PDF engine** — Trait mentioned in docs; still concrete `render_pdf` only.
8. **YAML formatting** — `serde_yaml` export may reorder keys (cosmetic).
9. **Existing DBs** — `sort_order` migration; old DBs may need fresh import for order.
10. **Security** — Localhost-only, no auth (acceptable for stated scope).

---

## Milestone B checklist (honest)

- [x] `serve` + preview API (resolve from DB)
- [x] Catalog read/write APIs + validation
- [x] 2-pane UI (JSON editor, not structured forms)
- [ ] File watch / auto-rebuild on external `content/` edits (import remains manual)
- [ ] Structured editor driven by `theme.toml` optional fields
- [x] Export to `workspace/staging/content/` (not default overwrite of `content/`)

---

## Milestone C (landed)

- Applications CRUD + draft composition JSON in UI
- Submit writes manifest v1 + SQLite submission row
- Preview: catalog default, application draft, or canonical submission manifest
- Draft-vs-submitted warning uses `snapshot_hash`

**Milestone D:** search/filter, status workflow, tags, similarity + quickfill (see `similarity` crate).

Still deferred: presets table, structured composition editor, PDF path on submit.

---

## Commands (current)

```bash
cargo run -p jobdesk -- import
cargo run -p jobdesk -- import --force
cargo run -p jobdesk -- export
cargo run -p jobdesk -- export --mirror-repo-content   # careful
cargo run -p jobdesk -- build --pdf
cargo run -p jobdesk -- serve --open
```

---

## Changelog

| Date | Change |
|------|--------|
| 2026-09-17 | Initial review + import/export/build polish |
| 2026-09-17 | Foundation: staging export, validation, resolve-first build |
