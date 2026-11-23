---
title: JobDesk — Architecture Review Response (Reconnaissance)
date: 2026-11-06
status: review-gate-no-implementation
prior_report: 05_ARCHITECTURE_ALIGNMENT_REPORT.md
repo_commit: 81193f5cde81a03b10fb6533593a168b98abb4d9
experiments: "Disposable copies under /tmp only; production workspace/jobdesk.db untouched"
---

# JobDesk — Architecture Review Response

This report responds to owner feedback on `05_ARCHITECTURE_ALIGNMENT_REPORT.md`. **No source, migrations, production DB changes, or commits were made.** Experiments used disposable database copies and existing CLI commands.

---

## A. Architectural health

### Component map and responsibilities

| Layer | Crate / surface | Responsibility | Assessment |
|-------|-----------------|----------------|------------|
| **Domain types & rules** | `crates/domain` | `Composition`, `SubmissionManifest`, payload validation, phase/workflow enums, posting token match, `build_application_analyze` | **Appropriate boundary.** Pure logic, no I/O. |
| **Persistence** | `crates/store` | SQLite open + `migrate`, catalog CRUD, applications, skills graph, import/export | **Functional but monolithic.** `Store` is a single façade (~2.9k LOC across `applications.rs`, `skills.rs`, `migrate.rs`, `catalog.rs`). |
| **Resolution** | `crates/resolve` | Composition → template JSON; manifest replay; toolbelt injection from live evidence | **Clear role.** Keeps render inputs out of `store`. |
| **Rendering** | `crates/render_tex`, `crates/render_pdf` | MiniJinja + PDF; `jobdesk_build_aligned` mirrors server preview modes | **Some duplication with server** (see below). |
| **HTTP API** | `crates/server` | axum routes; catalog/applications/skills; **large preview pipeline in `lib.rs` (~833 lines)** | Preview/build orchestration competes with `render_tex::legacy_build`. |
| **CLI / MCP** | `crates/jobdesk` | User entrypoints; MCP stdio tools | MCP **lags HTTP** for portfolio read (truncated summary only). |
| **UI** | `crates/server/static/js` | Portfolio, composition, applications, skills panes | Uses REST catalog APIs fully; agents do not. |

**Primary data flows**

1. **Portfolio truth:** `elements` + append-only `element_revisions.payload_json` + optional `element_skill_links` (element-scoped, not revision-scoped).
2. **Application overlay:** `applications.draft_composition_json` pins `{element_id, revision_id, resume_tag_skill_ids}`.
3. **Submit freeze:** `resolve_composition` → denormalized `manifest.resolved` + composition pins written to `workspace/artifacts/manifests/*.json` + `submissions` row.
4. **Submitted replay:** `resolve_from_manifest` reads **only** `manifest.resolved` — no DB payload reload for section bodies (`crates/resolve/src/lib.rs` lines 120–133).

### Findings (concrete, with recommendation)

#### 1. Submitted vs draft resolution split (retain — deliberate)

- **Files:** `resolve/src/lib.rs` (`resolve_composition` vs `resolve_from_manifest`); `render_tex/src/legacy_build.rs` (`preview_mode == "submitted"`).
- **Behavior:** Draft previews re-read pinned **revisions** from DB and recompute **toolbelt** from current `element_skill_links`. Submitted previews use frozen `manifest.resolved` (including toolbelt at submit time).
- **Consequence:** Historical sends stay reproducible for text/tags that were resolved at submit. Post-submit skill link edits affect **draft** and **portfolio default** views only — not canonical manifest. This matches “immutable submission” intent.
- **Recommendation:** **Retain.** Document explicitly for maintainers (avoid “fixing” submitted preview to read live links).

#### 2. Element-level skill links vs revision-pinned narrative (tradeoff / mild debt)

- **Files:** `store/migrate.rs` (v10: `revision_skill_links` → `element_skill_links`); `composition.rs` pins `revision_id` but skills hang off `element_id`.
- **Consequence:** Two revisions of the same project share one evidence tag set. A tailored résumé pin can point at an older revision while skill tags reflect **latest** element-level assertions on draft/toolbelt paths. Manifest still frozen at submit.
- **Recommendation:** **Defer structural change.** For conversational enrichment, agents should read **revision payload + element skill links** together and treat tags as “current element evidence,” not revision-specific. If per-revision tags become necessary, prefer **narrow extension** (optional `revision_id` on links or tags embedded in composition only) over a claim ontology.

#### 3. Phase/workflow rules duplicated across three surfaces (simplify over time)

- **Files:** `domain/application.rs` (`validate_phase_and_workflow`, `phase_and_workflow_from_status`); `store/applications.rs` (`reconcile_phase_status`); `server/static/js/90-applications.js` (dropdown guards); `jobdesk/src/main.rs` (`application set-status`).
- **Consequence:** Legacy rows (`phase=submitted`, zero submissions) are **detected in UI** but **not auto-repaired**. CLI `set-status interview` on such rows would hit `reconcile_phase_status` and fail if user tried to touch phase — edge case for disposable Jobright row only.
- **Recommendation:** **Simplify later:** one exported “lifecycle transition” API used by CLI, HTTP, and MCP; **do not** add external-submission metadata solely for legacy rows (owner direction). **Delete disposable app** when authorized.

#### 4. Preview/build pipeline duplication (actual complexity)

- **Files:** `server/src/lib.rs` (`build_preview`, `run_preview_build`); `render_tex/src/legacy_build.rs` (`jobdesk_build_aligned`).
- **Consequence:** Two paths must stay aligned when changing preview semantics (theme hash, submitted vs draft, composition override). Risk of drift, not yet observed as a bug in tests.
- **Recommendation:** **Defer refactor** until next preview feature; when touching preview, prefer calling one shared function from server rather than parallel logic.

#### 5. Monolithic migration function (maintainability debt)

- **Files:** `store/migrate.rs` — single `migrate()` with `schema_version` gates 7–15, inline DDL, data backfills (e.g. v4 phase backfill, v11 strip `skills` sections, v15 strip project `technologies`).
- **Consequence:** Hard to audit “fresh vs upgraded” behavior; v4 backfill **created the Jobright class of inconsistency** (phase=submitted without submit). Fresh DBs never run v4 on existing rows — only empty apps table.
- **Recommendation:** **Retain behavior** for compatibility; **do not** collapse migrations without owner-approved cutover. Optional future: append-only numbered migration files **when** next schema change lands — not a standalone refactor.

#### 6. Analyzer semantics vs product principles (product defect, not schema)

- **Files:** `domain/application_analyze.rs` — `SkillAlignmentStatus::Gap` when posting matches taxonomy but `element_skill_links` absent; `Partial` when links exist but not on draft.
- **Consequence:** UI/MCP copy can read like “you lack capability” when the system only knows “no taxonomy link asserted.” This **violates** owner gap-analysis principles. `composition_gaps` correctly signals **omitted from application**, not missing capability.
- **Recommendation:** **Revise analyze labeling and suggestions** (increment 7 in roadmap) without new tables — e.g. rename statuses to `unlinked_in_catalog`, `linked_not_on_draft`, `on_draft`; never infer competence from `details[]` text (analyzer already does not scan narrative).

#### 7. Tests: mix of invariants and implementation checks

- **Strong:** golden LaTeX (`render_tex`), manifest replay path, resume tag validation (`resolve/tests.rs`), analyze partial row (`application_analyze.rs` test).
- **Weak / missing:** no MCP tests; no test that submitted PDF context unchanged after skill link mutation; no lifecycle test for delete application; no migration regression suite on synthetic v3 DB.
- **Recommendation:** Add tests tied to **invariants** (manifest replay independence, delete application isolation) with first MCP increment — not broad coverage for its own sake.

#### 8. Portfolio vs résumé coupling (mostly intentional)

- **Files:** `resolve/default_composition` (latest revs); application picker pins subsets; `export_catalog_to_content` exports latest only (`store/export.rs`).
- **Consequence:** Super-résumé “master record” is the **full catalog in SQLite**, not repo `content/*.yaml`. YAML import is seed/round-trip, lossy for skill links and UI-created ids.
- **Recommendation:** **Retain** split; agents should use **DB APIs**, not repo YAML alone, for enrichment.

### What is *not* debt (avoid refactor bait)

- Separate `similarity`, `onet_import`, `theme` crates.
- Deterministic analyze + MCP read model (no in-process LLM).
- Toolbelt as resolve-time injection rather than composable section.

---

## B. Database baseline

### Experiments performed

| Step | Result |
|------|--------|
| Copy production DB | `/tmp/jobdesk-baseline-copy.db` from `workspace/jobdesk.db` |
| Fresh open via `Store::open` | `cargo run -p jobdesk -- --workspace /tmp/fresh-ws-empty application list` → creates `/tmp/fresh-ws-empty/jobdesk.db` |
| Schema diff | **No DDL differences** between fresh and production copy (sorted `.schema` identical) |
| `schema_meta.schema_version` | **15** on both |
| Fresh + `import` | `/tmp/rebuild-test/jobdesk.db` — 12 elements, 39 `skill_nodes` (seed catalog only) |
| Production copy counts | 13 elements, 17 revisions, 1 application, 0 submissions, 8808 skill_nodes, 3 element_skill_links |

### Structural equivalence

**Yes:** A newly initialized database receives the same table definitions and schema version **15** as production after `migrate()`.

**No (data equivalence):** A fresh DB is **not** a substitute for production content.

- O\*NET import (`jobdesk skills import-catalog --onet-dir …`) is **manual** — explains 39 vs 8808 nodes.
- Repo `import` **does not restore** `element_skill_links`, application rows, extra revisions, or UI-created element ids (production has `experience.knowledge-in-motion-2024-11`; seed import produces different slugs and omits that row entirely).
- Import/export YAML covers header/objective/education/experience/projects lists only — **not** full super-résumé state.

### Migration chain observations

- **v4:** Backfilled `phase=submitted` from legacy flat `status` — root cause of Jobright `phase=submitted` with zero submissions (no submit flow at time).
- **v10:** Migrated to `element_skill_links`; dropped `revision_skill_links` — fresh DBs never create the old table.
- **v11:** Removed `skills.main` elements and stripped `skills` sections from stored compositions — one-time JSON mutation.
- **v15:** Strips deprecated `technologies` from project payloads when upgrading from &lt;15 — idempotent via version gate.
- **v14:** Explicit no-op placeholder — harmless.

### Controlled reconstruction — benefits and risks

| Approach | Benefits | Risks |
|----------|----------|-------|
| **Keep current DB; delete Jobright when authorized** | Preserves 13 elements, 17 revisions, 3 skill links, O\*NET import | One bad legacy tracker row remains until delete |
| **Fresh DB + import + O\*NET + manual retag** | Clean tracker; same schema | **Lossy** unless skill links and non-YAML elements re-entered; element id drift breaks mental model |
| **File copy backup + selective delete** | Safest operational hygiene | Does not simplify schema |

**Recommendation:** **Do not rebuild** production DB for architectural reasons. Schema is healthy at v15. When owner authorizes removal of Jobright, use existing delete (see C). Optional: **backup** `workspace/jobdesk.db` before delete. Rebuild only if future corruption or intentional reset — restore via **DB backup**, not YAML import alone.

**Reject (per owner):** External-submission metadata / compatibility layer **only** for Jobright.

---

## C. Submission and revision invariants

### Faithful reproduction of historical submissions

**When a canonical submission exists:** Re-render uses `manifest.resolved` only (`resolve_from_manifest`). Changing portfolio revisions or `element_skill_links` **does not change** submitted preview/PDF, verified by design in `resolve/src/lib.rs` and `legacy_build.rs`.

**Manifest also stores:** `composition` with pinned `revision_id`s and `resume_tag_skill_ids` — audit trail of what was sent even if revisions later deleted.

**Actual defect — revision deletion:** If a pinned `revision_id` is removed from DB, **draft** resolve fails in `load_item_payloads`; **submitted** replay still works via `resolved`. Deleting **elements** referenced in a saved manifest does not delete manifest file; replay still uses denormalized JSON.

**Skill links after submit:** Manifest `resolved.toolbelt` is frozen; live links can diverge — **not a reproduction bug** for submitted mode.

**No production submissions exist today** (`submissions` count 0). Invariants are architectural, not empirically tested against real manifests in this workspace.

### Jobright removal (disposable application)

| Question | Finding |
|----------|---------|
| Supported delete? | **Yes** — `DELETE FROM applications WHERE id = ?` (`store/applications.rs`); CLI `application delete`; HTTP `DELETE /api/applications/{id}` |
| Effect on portfolio? | **Verified on copy:** after delete, **13 elements, 17 revisions, 3 skill links unchanged** (`/tmp/delete-test-ws`) |
| Submissions cascade? | FK `ON DELETE CASCADE` on `submissions.application_id` — irrelevant for Jobright (0 rows) |
| Orphan files? | Submit writes `workspace/artifacts/manifests/{uuid}.json` — **delete_application does not remove manifest files**. Jobright has none. Future submits could orphan files if DB row removed without file GC — **minor hygiene debt**, not blocking Jobright delete |

**Recommendation when authorized:** Delete Jobright via UI or `cargo run -p jobdesk -- application delete 7fe40e06-7964-44df-9910-56009347f926`. No J.1 metadata required.

---

## D. Revised first implementation increment

**Goal:** Enable an external LLM agent to run **conversational portfolio enrichment** with the user retaining canonical control — **before** new portfolio schema.

### Gap today

The browser UI already exposes full catalog reads:

- `GET /api/catalog/elements/{id}` → all revisions + latest payload (`server/src/catalog.rs`)
- `GET /api/catalog/revisions/{id}` → single revision
- `GET /api/catalog/elements/{id}/skill-links`
- `GET /api/catalog/elements/{id}/usage`

MCP exposes only `get_catalog_summary` (max **200** elements, **truncated** payload snippets, 12k char cap) and `get_skill_evidence` (`crates/jobdesk/src/mcp.rs`).

Agents working in Cursor **cannot** see full revision history or complete payloads through MCP — the blocker is **MCP parity**, not SQLite schema.

### Smallest coherent increment: read-only MCP portfolio mirror

**New MCP tools** (stdio, same DB lock model):

| Tool | Behavior | Store/API equivalent |
|------|----------|----------------------|
| `get_catalog_element` | Full `ElementDetail` (id, kind, all revision summaries, latest payload) | `Store::get_element` |
| `get_catalog_revision` | Full `RevisionDetail` | `Store::get_revision` |
| `list_catalog_elements` | `list_elements` optional `kind` filter; **no** 200 cap by default; optional `limit` param | `list_elements` |
| `get_element_usage` | Applications/presets referencing element | `element_usage` |
| *(existing)* `get_skill_evidence` | Keep | skill links |

**Optional (same increment if cheap):** `list_catalog_revisions_for_element` — redundant if `get_catalog_element` returns revision list + agent calls `get_catalog_revision` per id.

**Privacy / safety boundaries**

- MCP remains **local DB only**; no outbound network.
- Apply **per-field prudence**, not verification gates: return full portfolio payloads (user-owned). Do **not** add instructions or schema that encourage uploading classified/NDA material.
- Keep **truncation** on `posting_text` previews in application tools; portfolio tools return full JSON (agent prompts should warn user about pasting secrets into chat).
- **No write tools** in this increment — user approves via existing Portfolio UI or future explicit write MCP behind review.

**Affected modules**

- `crates/jobdesk/src/mcp.rs` — tool defs + handlers
- Reuse `store::Store` methods — **no** `store` schema change
- Tests: new unit/integration tests in `jobdesk` crate invoking handler logic with temp DB + fixture element (mirror `server` catalog tests pattern)

**Acceptance criteria**

1. Agent can fetch `project.cybersecurity-lab` with **complete** latest payload and **all** revision ids/metadata.
2. Agent can fetch any revision body by id and reconcile with composition pins if given an application id via existing `get_composition`.
3. Agent can read skill links and usage before suggesting edits.
4. Payload size &gt; 12k chars returns **clear truncation or chunked policy** documented in tool description (prefer raise limit for single-element reads only).
5. `cargo test` passes; at least one test asserts MCP tool output matches `Store::get_element` for a fixture.
6. No change to manifest, analyze, or migration behavior.

**Conversational workflow (exercise after ship)**

1. MCP load element + revisions + links.
2. Agent interviews user; optionally reads user-supplied **permissible** repo paths outside JobDesk.
3. Agent outputs proposed revision JSON + skill link changes **in chat**.
4. User edits via Portfolio UI (or CLI `catalog add` / revision API) — **canonical promotion**.

---

## E. Updated roadmap

Reconciliation with owner feedback vs `05`:

| Prior proposal (`05`) | Verdict | Revised action |
|----------------------|---------|----------------|
| J.1 external submission metadata | **Rejected** | Delete Jobright when authorized; no new columns for external origin |
| Mandatory / core `support_level` | **Rejected** | Optional free-text or reference list only when user chooses; never block entry creation |
| Payload narrative fields first | **Deferred** | **After** MCP read parity + real enrichment exercise on NEBULA / Cybersecurity Lab |
| `decision_notes_json` / desirability | **Deferred** | After portfolio agent loop proves useful (step 6–7) |
| Normalized claims/evidence tables | **Deferred** | No demonstrated need |
| Analyzer v2 gap **typing** | **Retained, revised** | Relabel statuses; separate taxonomy match from capability; no narrative NLP |
| Hybrid minimal enrichment | **Retained** | Sequence driven by confidence, not schema expansion |
| DB rebuild | **Rejected** for now | Backup + delete disposable app suffices |

### Provisional sequence (owner-aligned)

1. **Architectural & DB confidence** — this report; optional owner-approved Jobright delete + file backup.
2. **Material defects** — analyzer misleading “Gap” language (small domain copy/logic change when approved); optional manifest file GC on application delete (only if owner wants hygiene).
3. **MCP read-only portfolio mirror** — **first implementation increment (Section D)**.
4. **Conversational enrichment** — dogfood on real projects; user promotes via UI.
5. **Optional narrative schema** — only if JSON in `details[]` proves insufficient (references as optional array, not verification).
6. **Desirability / feasibility notes** on applications.
7. **Analyzer requirement alignment** — inferred vs explicit posting duties still **manual/LLM outside DB** unless structured requirement storage justified.
8. **Full application workflow exercise** — posting text, analyze, compose, submit.

**Stop condition after step 4:** If agents can enrich portfolio entries with user review without schema changes, **postpone step 5**.

### Critique of prior report and instructions

- **`05` was right** to prioritize MCP and defer claim tables, but **wrong** to prioritize J.1 external metadata and `support_level` as first-class product concepts — owner feedback correctly removes those.
- **Instructions are sound** on verification and confidentiality; the codebase already aligns on “no proof required to record work” at the **data model** level (bullets + optional links). Gaps are **agent access** and **analyzer wording**, not missing forensic tables.
- **Be critical of codebase:** the largest maintainability risk is **preview duplication** and **monolithic migrations**, not lack of normalized claims. Refactor only when touching those areas for feature work.

---

## Compact summary (copy/paste)

```
JobDesk @ 81193f5: schema v15 fresh ≡ prod DDL. Data not equivalent via import alone (skill links, O*NET, UI elements, revisions). Jobright delete: supported, portfolio untouched (verified on /tmp copy). No external-submission metadata needed.

Architecture: resolve_from_manifest freezes submitted output; draft uses live revisions + element_skill_links. Element-level tags + revision-pinned text = known tradeoff. server/lib.rs vs legacy_build preview duplication = defer refactor. Analyzer "Gap" = missing taxonomy link, NOT missing capability — relabel when implementing.

First increment: MCP read-only parity with catalog HTTP API (get_catalog_element, get_catalog_revision, list_catalog_elements, get_element_usage) — NO schema. User promotes changes via UI. Reject support_level mandate and J.1 metadata.

Roadmap: (1) confidence + authorized Jobright delete (2) analyzer semantics (3) MCP portfolio read (4) conversational enrichment (5) optional schema only if needed (6+) desirability, analyzer duties, full apply flow.
```

---

**Review gate:** Awaiting owner approval before implementation. Authorized next step when ready: **Section D (MCP portfolio read mirror)** and/or **authorized Jobright delete** on production workspace.
