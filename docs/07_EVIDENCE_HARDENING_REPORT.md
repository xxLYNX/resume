---
title: JobDesk — Evidence-Driven Hardening and MCP Portfolio Access
date: 2026-11-06
baseline_commit: 81193f5cde81a03b10fb6533593a168b98abb4d9
post_change_commit: uncommitted (awaiting owner approval)
---

# Evidence-Driven Architectural Hardening and MCP Implementation

## Validation report (E)

| Item | Value |
|------|--------|
| Baseline commit | `81193f5` |
| Working tree | Source changes in `domain`, `store`, `jobdesk`, `resolve` tests; planning docs untracked; **production DB modified** (authorized Jobright delete + backup) |
| Backup | `workspace/jobdesk.db.bak-2026-11-06` — `PRAGMA integrity_check` → **ok** |
| Tests before (baseline) | **29** unit tests passed (`cargo test` on 2026-11-06 discovery pass) |
| Tests after | **44** unit tests passed (`cargo test` full workspace, 2026-11-06) |
| Commands | `cargo test`; `cargo run -p jobdesk -- --workspace workspace application delete …`; `sqlite3` integrity/count queries |

### Changed files (code)

- `crates/domain/src/application_lifecycle.rs` — authoritative lifecycle rules
- `crates/store/src/applications.rs` — store uses domain lifecycle; integration tests
- `crates/jobdesk/src/mcp_tools.rs` — MCP tool handlers + tests
- `crates/jobdesk/src/mcp.rs` — stdio wiring only
- `crates/resolve/src/tests.rs` — manifest freeze vs live skill links
- `crates/store/src/migrate.rs` — fresh schema version test
- `docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md` — MCP tool list

---

## A. Evidence-backed architectural report

### A1. Application lifecycle consistency

| Field | Content |
|-------|---------|
| **Finding** | Lifecycle enforcement was split: validation enums in `domain/application.rs`, transition rules in private `reconcile_phase_status` in `store/applications.rs`, UI disables invalid dropdowns in `90-applications.js`, CLI `set-status` inferred phase+workflow but **`application update --status` alone did not infer phase**, leaving inconsistent `(phase, workflow)` pairs that failed validation or blocked legitimate CLI updates. |
| **Classification** | **Confirmed defect** (workflow-only CLI update path); **Demonstrated architectural liability** (rules not in one domain module). |
| **Code evidence** | `domain/application.rs` — `validate_phase_and_workflow`, `phase_and_workflow_from_status`; former `store/applications.rs:809-839` `reconcile_phase_status`; `store/applications.rs:529-565` `update_application`; `jobdesk/src/main.rs:443-478` CLI; `server/static/js/90-applications.js:80-107`, `519-524` (UI sends both fields). |
| **Behavioral evidence** | **Before fix:** `update_application` with only `status: Some("interview")` kept `phase: draft` → `validate_phase_and_workflow` error. **After fix:** `store/applications.rs` lifecycle test `cli_style_workflow_only_update_after_submit` passes. Domain tests in `application_lifecycle.rs` cover transition matrix cases. Legacy `phase=submitted` + zero submissions still rejected on lifecycle touch (`legacy_mismatch_*` test). |
| **Impact** | CLI and HTTP partial updates could disagree with UI; operators see “Save failed” or inconsistent tracker state. |
| **Proposed remedy** | **Done:** `domain::apply_lifecycle_patch` + `coerce_phase_and_workflow`; store calls it when phase or status touched. |
| **Risk of remedy** | Low; HTTP already sent both fields. |
| **Decision** | **Fix now** |

**Transition matrix (authoritative: `domain/application_lifecycle.rs`)**

| has_submission | Request | Result |
|----------------|---------|--------|
| false | phase draft (any workflow) | → `(draft, draft)` |
| false | phase submitted | **Reject** |
| false | workflow only `interview` | infer submitted → **Reject** (no manifest) |
| true | phase draft | **Reject** |
| true | phase submitted + workflow `interview` | **Accept** |
| true | workflow only `interview` | **Accept** (infers submitted phase) |

**UI role:** Advisory/disabled options only; **store is enforcement** for API/CLI.

---

### A2. Résumé preview and build orchestration

| Field | Content |
|-------|---------|
| **Finding** | Server preview and CLI build share **`jobdesk_build_aligned`** for default catalog, application draft, and submitted modes; server adds one **extra branch** for unsaved live `draft_composition` on preview requests. |
| **Classification** | **Acceptable tradeoff** with **demonstrated architectural liability** (server `run_preview_build` duplicate match arms). |
| **Code evidence** | `server/src/lib.rs:590-634` `run_preview_build`; `render_tex/src/legacy_build.rs:85-145` `jobdesk_build_aligned`; submitted path both use `resolve_from_manifest` (`legacy_build.rs:107-127`, `resolve/src/lib.rs:120-133`). |
| **Behavioral evidence** | Traced paths: (1) portfolio default → `jobdesk_build_aligned(..., None, draft)` → `default_composition` + `resolve_composition`; (2) app draft → same with `get_application().draft_composition`; (3) live DOM → `jobdesk_build_composition_to_dir` only on server; (4) submitted → manifest replay only. **No test** yet comparing server vs CLI `.tex` hash for same inputs — **limitation**. |
| **Impact** | Future preview-only logic could drift in `server/lib.rs` without touching CLI. |
| **Proposed remedy** | Defer consolidation; when changing preview, route live-composition branch through a shared helper in `render_tex`. |
| **Decision** | **Defer** — consolidation is cross-crate; not required for MCP increment. |

---

### A3. Database migrations and schema integrity

| Field | Content |
|-------|---------|
| **Finding** | Fresh `Store::open` yields schema **v15** DDL identical to production copy (discovery diff). Migration logic is monolithic with historical one-time data fixes (v4 phase backfill, v10 skill link migration, v11 skills section strip). |
| **Classification** | **Demonstrated architectural liability** (hard to replay historical versions in tests); **not** a demonstrated data corruption on v15. |
| **Code evidence** | `store/migrate.rs` — `migrate()`, version gates `< 7` … `< 15`. |
| **Behavioral evidence** | `migrate_tests::fresh_database_schema_version_is_15`; discovery `diff` of `.schema` fresh vs prod empty. **Not reproduced:** full chain from synthetic v1 DB (no fixtures in repo). |
| **Impact** | Future migrations add risk to single function; v4 backfill caused legacy phase/submission skew (Jobright — removed). |
| **Decision** | **Defer** modular migration files; **retain** monolith until next schema change with append-only migration + test. |

---

### A4. Revision and skill-association invariants

| Field | Content |
|-------|---------|
| **Finding** | Submitted résumé content is frozen in `manifest.resolved`; live draft toolbelt reflects current `element_skill_links` at element scope. |
| **Classification** | **Acceptable tradeoff** (element-level tags, revision-pinned text). |
| **Code evidence** | Submit stores resolved map (`store/applications.rs:630-651`); replay `resolve_from_manifest` (`resolve/src/lib.rs:120-133`). |
| **Behavioral evidence** | **`resolve/tests.rs::submitted_manifest_resolved_unchanged_when_skill_links_change`** — after new skill link, live `toolbelt` changes; manifest replay unchanged. |
| **Impact** | Draft/analyze can imply different skills than last send until re-submit; intended. |
| **Decision** | **Retain**; document for agents interviewing users about pinned revisions vs current tags. |

---

### A5. Analyzer gap semantics (not fixed this increment)

| Field | Content |
|-------|---------|
| **Finding** | `SkillAlignmentStatus::Gap` means “posting mentions taxonomy node, no `element_skill_links`” — not “user lacks capability.” |
| **Classification** | **Demonstrated architectural liability** (misleading product semantics). |
| **Code evidence** | `domain/application_analyze.rs:85-94`, `130-160`. |
| **Decision** | **Defer** relabeling to dedicated analyze increment (owner roadmap step 7). |

---

### A6. Application delete and manifest files

| Field | Content |
|-------|---------|
| **Finding** | `delete_application` removes DB row; submissions CASCADE; **manifest JSON files under `workspace/artifacts/manifests/` are not deleted**. |
| **Classification** | **Demonstrated architectural liability** (orphan files). |
| **Decision** | **Defer** — no submissions existed for Jobright; fix when submission history hygiene is prioritized. |

---

## B. Remediation summary

| Action | Detail |
|--------|--------|
| **Fixed** | Centralized lifecycle in `domain/application_lifecycle.rs`; store `update_application` uses `apply_lifecycle_patch` (fixes workflow-only CLI/API updates). |
| **Added tests** | Domain lifecycle (7); store lifecycle (3); MCP tools (3); resolve manifest freeze (1); migrate fresh version (1). |
| **MCP** | `get_catalog_element`, `get_catalog_revision`, `get_element_usage`; full JSON up to 512KB or explicit error. |
| **DB maintenance** | Backup + delete Jobright `7fe40e06-7964-44df-9910-56009347f926`; portfolio counts unchanged (13/17/3). |
| **Retained tradeoffs** | Element-level skill links; manifest denormalization; server live-composition preview branch. |
| **Deferred** | Preview orchestration merge; modular migrations; analyzer gap labeling; manifest file GC on delete. |
| **Behavior ownership** | Application lifecycle transitions → **`domain::apply_lifecycle_patch`**; persistence → **`store::update_application`**; MCP portfolio read → **`jobdesk::mcp_tools::call_tool`** → **`Store::get_element` / `get_revision`**. |

---

## C. MCP implementation

Tools (see `crates/jobdesk/src/mcp_tools.rs`):

- **`get_catalog_element`** — full `ElementDetail`
- **`get_catalog_revision`** — full `RevisionDetail`
- **`get_element_usage`** — `ElementUsageReport`
- Existing summary tools unchanged (truncation policy unchanged except catalog full-read tools)

Privacy: documented in `docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md` — local DB read does not imply LLM privacy; no silent truncation on full element/revision reads.

---

## D. JobDesk portfolio enrichment demonstration

**Repository-observable facts (no portfolio row for JobDesk today):**

- Rust workspace: `domain`, `store`, `resolve`, `render_tex`, `server`, `jobdesk`, etc. (`ARCHITECTURE.md`).
- Milestones through **K** (application analyze + MCP); schema **v15**.
- Local-first SQLite workspace; MCP stdio; no mandatory cloud LLM.
- Product split: Portfolio (revisions) vs Applications (composition + submit manifests).

**MCP sufficiency:** An agent can load any existing catalog element (e.g. `project.cybersecurity-lab`) with full revision history. For **JobDesk as a project**, the user would create a portfolio entry first; the agent can then enrich via interview using repo inspection separately.

**Initial interview questions (require user input):**

1. What problem were you solving that off-the-shelf résumé/tracker tools did not address?
2. Which parts did you implement yourself vs adopt (Tectonic, O\*NET import, MiniJinja themes)?
3. What were the hardest correctness issues (preview/submit drift, phase/status, skill evidence model)?
4. What is the intended audience for JobDesk — only you, or eventual open-source users?
5. What outcomes can you claim today (milestones shipped, daily use, applications tracked) without overstating deployment?

**Draft enrichment plan (user-approved promotion only):**

1. MCP/`ARCHITECTURE.md` pass → outline `project.jobdesk` bullets (scope, stack, milestones).
2. Interview → capture contribution narrative, constraints, failures.
3. User saves new revision in Portfolio UI; optionally assert skills (e.g. Rust, SQLite, product design).
4. Defer public website projection (Milestone G).

---

## E. Reproduction commands

```bash
cd /home/voyager/Projects/resume
cargo test
cargo test -p jobdesk mcp_tools
cargo test -p store lifecycle
cargo test -p resolve submitted_manifest
```

---

## F. Open items for owner approval

- Git commit of code changes (not created per protocol).
- Whether to pursue preview orchestration consolidation or analyzer relabeling next.
- Whether to add `project.jobdesk` portfolio entry after interview.

---

**Stop gate:** Code and DB deletion complete pending your review; no git commit unless requested.
