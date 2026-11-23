---
title: Commit preparation and lifecycle policy (final)
date: 2026-11-06
baseline: 81193f5
status: reference-for-commit
---

# Lifecycle policy (edge cases)

## Explicit `phase=submitted` + `status=draft` in one request

**Policy:** **Reject** — contradictory explicit inputs must not be silently discarded.

**Implementation:** `apply_lifecycle_patch` in `crates/domain/src/application_lifecycle.rs` (lines 41–45) returns `InvalidTransition` before `coerce_phase_and_workflow`.

**Test:** `explicit_submitted_phase_with_draft_workflow_rejected`.

## Explicit `phase=submitted` with status omitted (persisted workflow was `draft`)

**Policy:** **Normalize omitted workflow** — the client did not assert a pipeline label in this request. Merge uses `(submitted, persisted status)`, then `coerce_phase_and_workflow` maps workflow `draft` → pipeline `submitted` when phase is `submitted` and a submission exists (lines 78–79).

**Contrast:** This is **not** the same as sending `status=draft` explicitly (which is rejected).

**If persisted workflow was `interview`:** unchanged — `phase_only_submitted_keeps_existing_pipeline_workflow`.

**Tests:** `phase_only_submitted_normalizes_persisted_draft_workflow`, `phase_only_submitted_keeps_existing_pipeline_workflow`.

**Authoritative owner:** `domain::apply_lifecycle_patch`; `store::update_application`; CLI/HTTP use the same store path.

---

# MCP revision-read demonstration (stdio)

Example revision: `project.cybersecurity-lab` → `4b932532-49ee-4349-bd26-ae9194e72849`.

```bash
target/debug/jobdesk --root /path/to/repo --workspace workspace mcp
# JSON-RPC lines: initialize → tools/call get_catalog_element → get_catalog_revision
```

- Invalid revision id → JSON-RPC error: `unknown revision \`…\``
- Payload match to SQLite: `crates/jobdesk/tests/mcp_protocol.rs`
- Oversized element → explicit error (`exceeds limit`), not silent truncation: `mcp_stdio_oversized_element_returns_error_not_truncation`

---

# Final validation (2026-11-06)

| Check | Result |
|-------|--------|
| `cargo fmt --all --check` | Not run — `cargo fmt` not installed in verification environment |
| `cargo clippy … -D warnings` | Not run — `cargo clippy` not installed |
| `cargo test` | **55** passed, 0 failed |
| `git diff --check` | Clean |

## Stage for code commit

- `crates/domain/src/application_lifecycle.rs` (new)
- `crates/domain/src/lib.rs`
- `crates/store/`, `crates/jobdesk/` (incl. `tests/`)
- `crates/resolve/src/tests.rs`, `crates/server/src/lib.rs`
- `Cargo.lock`, `docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md`

## Do not stage

- `workspace/jobdesk.db`, `workspace/jobdesk.db.bak-*`, `workspace/artifacts/`
- Untracked planning docs `00_*`, `02_*`, `04_*`–`08_*` unless owner wants a separate docs commit

---

# Recommended commit(s)

**Single commit (recommended):**

```
feat(jobdesk): centralize lifecycle and add MCP portfolio read tools

Move application phase/workflow rules to domain apply_lifecycle_patch.
Add get_catalog_element, get_catalog_revision, and get_element_usage over
MCP stdio with protocol and lifecycle regression tests.
```

**Alternative — two commits:**

1. `feat(domain): authoritative application lifecycle patch`
2. `feat(jobdesk): MCP portfolio read tools and acceptance tests`

No co-author trailers.

---

# Next: JobDesk portfolio interview (preparation)

Use MCP + repository inspection for observable facts; owner confirms contributions, decisions, and outcomes. Do not auto-create `project.jobdesk` in the catalog.

**Repository-observable (not authorship claims):** Rust/SQLite local workspace, multi-crate layout, milestones through K (analyze + MCP), append-only revisions, submit manifests, deterministic analyze.

**Owner questions:** motivation vs YAML-only tools; personal scope of design/implementation; hardest correctness issues; current daily use; safe public claims; résumé positioning; privacy boundaries.

**Draft narrative (candidate only):** Local-first employment workspace in Rust/SQLite with portfolio revisions, application compositions, immutable submits, skill taxonomy links, posting analysis, and stdio MCP for agent-assisted enrichment.
