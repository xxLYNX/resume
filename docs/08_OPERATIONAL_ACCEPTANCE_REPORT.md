---
title: JobDesk — Implementation Verification and Operational Acceptance
date: 2026-11-06
baseline: 81193f5
---

# Operational Acceptance Report

## 1. Implementation diff review (vs `81193f5`)

| Change | Behavior altered | Owner | Tests |
|--------|------------------|-------|-------|
| `domain/application_lifecycle.rs` | Phase/workflow transitions; workflow-only inference; **explicit contradictory pairs rejected** | **domain** | 8 unit tests in module; store + CLI integration |
| `store/applications.rs` | `update_application` calls `apply_lifecycle_patch`; lifecycle/delete integration tests | **store** (persistence) | 3 store tests; CLI tests |
| `jobdesk/mcp_tools.rs` + `mcp.rs` | MCP portfolio read tools; no silent truncation on full element/revision | **jobdesk::mcp_tools** | 3 unit + 2 stdio integration |
| `resolve/tests.rs` | Characterization: manifest frozen vs live skill links | **resolve** | 1 test |
| `store/migrate.rs` | Fresh DB schema version assertion | **store/migrate** | 1 test |
| `server/lib.rs` | Preview vs `jobdesk_build_aligned` TeX parity | **server** + **render_tex** | 1 test |
| `docs/MILESTONE_K_*` | Tool list / privacy notes | docs | — |

**Coupling check:** MCP handlers call `Store` only — no parallel catalog logic. Lifecycle rules live only in **domain**; CLI/HTTP both hit **store**.

**Regression risk:** Contradictory `phase`+`status` in one PATCH now **errors** (was silent normalize via `coerce`). UI always sends consistent pairs (`90-applications.js:519-523`) — compatible.

---

## 2. Lifecycle validation (CLI + store/HTTP-equivalent)

**Authoritative owner:** `domain::apply_lifecycle_patch` / `coerce_phase_and_workflow`.

| Scenario | CLI / store | Result |
|----------|-------------|--------|
| Workflow-only `interview` after submit | `lifecycle_cli::cli_update_status_only_after_submit` | **OK** → `submitted`/`interview` |
| `set-status interview` without submit | `cli_set_status_interview_without_submit_fails` | **Rejected** |
| PATCH `phase=draft` + `status=interview` | `store_http_equivalent_contradictory_*` | **Rejected** (explicit error) |
| Workflow-only without submit | domain + store tests | **Rejected** |

**HTTP:** Handlers call `store.update_application` only (`server/applications.rs:99`) — same code path as CLI `update`/`set-status`. No separate HTTP-only rules.

**Intentional transforms (not errors):**

- `phase=submitted` + `workflow=draft` → coerced to `submitted`/`submitted` when manifest exists.
- `phase=draft` + any workflow → coerced to `draft`/`draft` when only phase is draft **and** workflow omitted or draft; **not** when both provided and contradictory (now error).

**Silent discard removed for:** explicit contradictory pairs in one request.

---

## 3. MCP stdio protocol (end-to-end)

**Commands executed:**

```bash
cargo test -p jobdesk mcp_stdio
# Manual read-only production workspace:
( echo '{"jsonrpc":"2.0","id":1,"method":"initialize",...}'
  echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_catalog_element","arguments":{"element_id":"project.cybersecurity-lab"}}}'
) | target/debug/jobdesk --root … --workspace workspace mcp
```

**Results:**

- `initialize` → protocol `2024-11-05`, server `jobdesk`.
- `tools/list` includes `get_catalog_element`, `get_catalog_revision`, `get_element_usage`.
- `get_catalog_element` JSON **matches** `Store::get_element` for `project.cybersecurity-lab` (integration test asserts payload equality).
- Unknown element → JSON-RPC **`error`**, message contains `unknown element`.
- 600KB payload → JSON-RPC **`error`**, message contains `exceeds limit`, **no** `[truncated]`.

**Oversized recovery:** Use `get_catalog_revision` for one revision (same payload if single rev); for future multi-revision giants, add optional `fields` filter or streaming — **not implemented**; document as limitation.

**Privacy:** Local MCP read; external LLM consumption is operator responsibility (per Milestone K doc).

---

## 4. Preview / build parity

**Test:** `server::preview_tests::preview_build_matches_jobdesk_build_aligned_for_catalog_default`

- Compares `jobdesk_build_aligned(..., None, "draft", classic)` with `build_preview` for catalog default.
- **`content_hash` and full `.tex` string equal.**

**Ownership boundary:** Shared rendering via `render_tex::jobdesk_build_aligned`; server adds only cache/PDF wrapper and live-composition branch (unchanged).

---

## 5. Database checks

| Check | Result |
|-------|--------|
| Production `applications` after Jobright delete | **0** |
| Elements / revisions / skill links | **13 / 17 / 3** (unchanged) |
| Backup `workspace/jobdesk.db.bak-2026-11-06` | `PRAGMA integrity_check` → **ok**; restore copy shows **1** Jobright row (expected) |
| Fresh schema v15 | `migrate_tests::fresh_database_schema_version_is_15` |

**Next schema change should add:** migration unit test on disposable DB; optional fixture upgraded from v14 snapshot if available.

---

## 6. MCP portfolio discovery (production workspace, read-only)

**Entry read via MCP:** `project.cybersecurity-lab` — “Cybersecurity Lab”, Feb 2023–present, bullets on NixOS flake, SOC stack, forge/CI, zero-trust/Tailscale.

**Proposed enrichment workflow:**

1. MCP `get_catalog_element` + `get_skill_evidence` + `get_element_usage`.
2. Agent interviews owner on scope, solo vs team, production vs lab-only claims.
3. Owner saves revised bullets / optional references in Portfolio UI.
4. Optional skill tags after review (not proof gates).

### JobDesk as new portfolio project (draft — not in catalog)

**Repository-observable:**

- Rust workspace: `domain`, `store`, `resolve`, `render_tex`, `server`, `jobdesk`; SQLite catalog; Milestones through **K** (analyze + MCP).
- Local-first; append-only revisions; submit manifests; O\*NET skill taxonomy; deterministic analyze.
- Recent hardening: centralized lifecycle, MCP full catalog read, tests for manifest immutability.

**Worth discussing:** split portfolio vs applications; element-level vs revision-pinned skills; manifest denormalization; MCP as optional agent layer without in-process LLM.

**Requires user input:** daily use claims, deployment, authorship %, failures/lessons, whether to open-source, desired résumé positioning.

**Owner questions:**

1. What prompted building JobDesk instead of extending YAML/ Obsidian-only workflow?
2. Which milestones reflect work you personally designed and implemented?
3. What is safe to claim publicly (screenshots, architecture, no private lab details)?
4. Target roles where JobDesk is a portfolio anchor vs supporting detail?

**Draft narrative (candidate, not canonical):**

> *JobDesk* — local-first Rust/SQLite employment workspace: structured portfolio with append-only revisions, tailored application compositions, immutable submit manifests, O\*NET-backed skill evidence, deterministic posting analysis, and stdio MCP for agent-assisted authoring. Implemented core crates (catalog, resolve, render, API, CLI) and milestone delivery through application intelligence (2026).

---

## 7. Corrections during verification

| Issue | Fix |
|-------|-----|
| Explicit `phase=draft` + `status=interview` silently normalized | **Rejected with `InvalidTransition`** in `apply_lifecycle_patch` |
| MCP only unit-tested | Added **`tests/mcp_protocol.rs`** stdio harness |
| Preview/CLI drift unproven | Added **server parity test** |

---

## 8. Test summary

```bash
cargo test   # full workspace — all passed (2026-11-06)
```

Approximate **51** passing tests (was 44 before acceptance pass): +3 CLI lifecycle, +2 MCP stdio, +1 domain contradiction, +1 preview parity.

---

## 9. Remaining limitations

- No live **curl HTTP** lifecycle suite (store/CLI demonstrate same `update_application`).
- Analyzer `Gap` label semantics unchanged (deferred).
- Orphan manifest files on application delete (deferred).
- Oversized single-revision payloads: error only; no chunking API yet.

---

**No git commit or portfolio writes performed.** Ready for owner commit authorization.
