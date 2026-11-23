---
title: JobDesk — Architecture Alignment Report (Discovery)
date: 2026-11-06
status: review-gate
repo_commit: 81193f5cde81a03b10fb6533593a168b98abb4d9
branch: master
local_uncommitted: "00_MASTER_OPERATING_MODEL.md, 02_JOBDESK_EVIDENCE_SLICE_SPEC.md, 04_JOBDESK_AGENT_DISCOVERY_HANDOFF.md (docs only; no source edits)"
---

# JobDesk — Architecture Alignment Report

## 1. Executive conclusion

JobDesk on `master` at `81193f5` is a **mature local-first Rust/SQLite résumé workspace**: append-only portfolio revisions, per-application composition pins, immutable submit manifests, O\*NET-backed skill taxonomy with **element-level** evidence links, deterministic **application analyze** (Milestone K), and a scoped **stdio MCP** server. That already supports much of workflow **D** (tailored résumé + frozen send) and the **composition/gap** slice of workflow **C** at taxonomy granularity—not at requirement or claim granularity.

What the **corrected product vision** still lacks is not another render engine but **richer portfolio narrative and provenance without mandatory proof**, **honest opportunity assessment** (desirability vs feasibility, inferred duties labeled), **gap typing** (missing skill vs under-documented capability vs unknown), and **external/out-of-band application** semantics. Proposal `02` correctly diagnoses coarse skill tags but **over-normalizes too early** (`CapabilityClaim`, `EvidenceItem`, campaign tables) before the simpler portfolio and application surfaces are exhausted.

**Recommended smallest slice (hybrid minimal):**

1. **L0 / J.1** — Non-destructive **submission integrity**: classify legacy `phase=submitted` + zero manifests as **external submission** (metadata only); never fabricate manifests.
2. **L1** — **Payload-level narrative extensions** on `project` / `experience` (optional contribution, outcome, support/provenance label, reference locators)—validated JSON, revision-append UX, no new ontology tables.
3. **L2** — **Application decision notes** (optional JSON on `applications`: desirability dimensions, feasibility notes, go/no-go)—MCP read first; LLM fills via editor, user saves.
4. **L3** — **Analyzer v2 (still deterministic)** — classify gaps using support labels + composition; distinguish *partial* (evidence elsewhere) from *documentation gap*; require non-empty `posting_text` for strategic use.

**Defer:** normalized `capability_claims` / `evidence_items` / `posting_requirements` until one real campaign needs reusable granular claims across multiple postings. **Defer:** Milestone G public portfolio until résumé-side narrative is trustworthy.

---

## 2. Current implementation map

| Area | Location | Verified behavior |
|------|----------|-------------------|
| **Schema version** | `crates/store/src/migrate.rs` → `schema_meta.schema_version = 15` | Confirmed on `workspace/jobdesk.db` |
| **Catalog** | `elements`, `element_revisions` | Kinds: `header`, `objective`, `experience`, `education`, `project`, `course`, `certification`, `award` (`domain/validation.rs`, `ARCHITECTURE.md`) |
| **Payload validation** | `crates/domain/src/validation.rs` | Projects: `name`, `date_range`, `details[]` strings only—no contribution/outcome/provenance fields |
| **Skills taxonomy** | `skill_nodes`, `tool_competency_edges` | O\*NET import; ~8k nodes in live DB (per export snapshot) |
| **Evidence links** | `element_skill_links(element_id, skill_id, source)` | **Element-level**, not revision-level (migration v10 dropped `revision_skill_links`) |
| **Résumé skill projection** | `store/skills.rs` `skill_ids_on_composition`, toolbelt section | PDF toolbelt from pinned elements’ asserted links |
| **Applications** | `store/applications.rs`, `applications` table | `phase`, `status`, `posting_text`, `draft_composition_json`, tracker fields (I) |
| **Submissions** | `submissions` + `workspace/artifacts/manifests/` | Submit writes manifest; sets `phase=submitted` |
| **Phase guardrails** | `reconcile_phase_status` in `applications.rs`; UI `90-applications.js` | Save cannot set `phase=submitted` without submission |
| **Analyze** | `domain/application_analyze.rs`, `GET /api/applications/{id}/analyze` | Posting keyword ↔ taxonomy ↔ draft evidence; composition_gaps; objective heuristics |
| **Posting hints** | `domain/posting_hints.rs`, F.3b | Same token/skill matching as analyze |
| **MCP** | `crates/jobdesk/src/mcp.rs` | Read-only: list/get application, catalog summary, skill evidence, hints, analyze, composition |
| **Export/import** | `store/export.rs`, `store/import.rs` | YAML to `workspace/staging/content/`—not a privacy-filtered public export |
| **UI** | `crates/server/static/js/` | Portfolio CRUD (J), evidence pickers (F), Applications analyze panel (K) |
| **Tests** | `cargo test` (2026-11-06) | **29 unit tests passed** across `domain`, `resolve`, `render_tex`, `similarity`, `store/catalog`, `server`, `onet_import`; uses temp/fixture DBs—not the live `workspace/jobdesk.db` |

**Assumptions (not re-verified in code this pass):** Tectonic/pdflatex availability; Forgejo vs GitHub mirror lag; full O\*NET node count exact figure.

**Doc vs code:** `02` and `00` describe `CapabilityClaim` / campaign records as **future JobDesk scope**; **no such tables or types exist** in the repository (grep confirms proposals only).

---

## 3. A/B/C/D workflow matrix

| Workflow | Current capability | Missing for corrected vision | Simplest change | Later |
|----------|-------------------|------------------------------|-----------------|-------|
| **A — Opportunity assessment** | Application row + tags + compensation/work mode (I); analyze gives skill alignment + objective warn | Personal desirability vector; feasibility separate from keyword match; inferred duty labeling; hard constraints gate; APPLY/PREP/EXPLORE/PASS decision record | Optional `decision_notes_json` (or columns) on `applications`; UI section + MCP read; manual/LLM-filled, user-saved | Structured `posting_requirements` rows; campaign dashboard (02 L6) |
| **B — Portfolio discovery & authoring** | Portfolio CRUD, revision append, `details[]` bullets, skill discovery UI, relationships pane | Per-bullet contribution; provenance/support level; reference locators; interview → **reviewed** promotion path in-product | Optional payload fields + reference list; MCP `get_element` full payload; doc’d Cursor interview → Save revision | Normalized claims/evidence when reuse across reqs justifies migration |
| **C — Gap analysis & development** | K.1 analyze: evidenced/partial/gap/extra **per taxonomy skill**; composition_gaps for omitted elements | Requirement-level matrix; gap types (skill vs representation vs unknown vs constraint); no training-plan orchestration | Analyzer uses support labels on elements; reporting buckets in JSON/UI copy | `requirement_claim_assessments` (02 L4) |
| **D — Presentation & feedback** | Composition picker, preview, Submit manifest, view-as-submitted, ledger PDF, draft≠submit warning | Outcome writeback (stage failed, claim challenged); public portfolio projection | Outcome fields on application workflow; Milestone G `show_on_portfolio` | Second theme / HTML export |

### Use-case walkthroughs (current system)

#### 1. Portfolio entry without proof (historical research)

| Step | Current path |
|------|----------------|
| Create | Portfolio → **New entry** → `project` (or use existing `project.biometric-authentication-modalities-research`) |
| Narrative | `details[]` string bullets in payload (`validation.rs` Project branch) |
| Contribution / unverified outcome | **No dedicated fields**—only free text in bullets |
| Skills | Optional `element_skill_links` via evidence UI—**not required** |
| Awkward | Entire project shares one skill tag set; cannot mark one bullet “self-reported” vs “artifact-backed”; research project omitted from Jobright draft composition but **exists** in catalog |

**Minimum missing:** optional structured `contribution`, `outcome`, `support` (enum), `references[]` on project/experience payloads.

#### 2. Portfolio enrichment via conversation (NEBULA)

| Step | Current path |
|------|----------------|
| Agent reads | MCP `get_catalog_summary`, `get_skill_evidence`, `get_element` **not exposed**—summary snippets only |
| Agent proposes | **Outside DB**—user must translate to Portfolio Save (new revision) + skill pickers |
| User approves | Manual UI save → new `element_revisions` row; `set_element_skill_links` API |
| Store result | Revision JSON + asserted links (`source = 'asserted'`) |

**Minimum missing:** MCP read of full element/revision; optional **draft proposal** file or staging JSON (no auto-promote); optional payload fields for locators returned from interview.

#### 3. Opportunity analysis (CrowdStrike-like endpoint posting)

| Step | Current path |
|------|----------------|
| Ingest | Paste `posting_text` on application; URL/metadata fields |
| Desirability | **Not modeled** |
| Compare reqs | Token match against O\*NET `skill_nodes` + `element_skill_links`; statuses in `ApplicationAnalyzeReport` |
| Gaps vs missing evidence | **Partial** ≈ catalog has tag, not on draft; **Gap** ≈ no catalog tag—**cannot** distinguish “can do but undocumented” from “cannot” |
| Works today | `application analyze`, UI report, MCP `analyze_application` **if posting_text populated** |

Live DB example: Jobright app `7fe40e06-…` has **`posting_text` length 0**—analyze is structurally weak until user saves text.

#### 4. Projection (tailored résumé / public profile)

| Step | Current path |
|------|----------------|
| Tailored résumé | Application draft composition → preview → Submit → manifest |
| Master portfolio | Portfolio tab default composition (latest revisions) |
| History | Manifest stores `{element_id, revision_id}` pins + denormalized resolved text |
| Public / filtered | **Not implemented** (`ARCHITECTURE.md` Milestone G) |
| Privacy | Submit manifest is user-controlled composition; no separate “private reference leak” guard in export |

**Minimum missing for public:** `show_on_portfolio` + export filter; for applications: J.1 external submission honesty.

---

## 4. Proposed `02` reconciliation

| Proposal | Verdict | Existing / alternative |
|----------|---------|----------------------|
| `CapabilityClaim` table | **Defer** | Project/experience bullets + future payload `assertions[]` |
| `EvidenceItem` + M:N `claim_evidence` | **Defer** | Optional `references[]` on payload; links are element-scoped today |
| `claim_skill_links` | **Defer** | `element_skill_links` + optional per-assertion skill ids in JSON |
| `PostingRequirement` | **Defer** | `posting_text` + analyze hints; manual notes in decision JSON |
| `RequirementClaimAssessment` | **Defer** | Analyze `skill_gaps` / `composition_gaps` (weaker) |
| `CampaignDecision` / `CampaignAction` | **Reject for now** | Application `phase`/`status` + optional decision notes—avoid task-manager sprawl |
| `candidate→verified` claim statuses | **Revise** | Prefer lightweight provenance: `self_described`, `source_linked`, `reviewed`, `reproduced` on payload or assertion—not lifecycle implying truth |
| Depth 0–5 on claims | **Defer** | Human rubric in `00`; optional single self-audit field later |
| MCP `list_claims`, `get_evidence`, writes | **Defer reads** until tables exist; **extend** MCP with `get_catalog_element` first |
| L0 submission audit | **Reuse + extend** | J guardrails + J.1 deferred auto-fix—implement **external_submission** semantics |
| Analyzer grounded in specific claims | **Revise** | Extend K.1 with support labels before claim IDs |
| Four-table foundation migration | **Reject as first slice** | High migration risk; element-level links already migrated once |

**Unnecessary immediately:** `capability_claims`, `evidence_items`, `claim_evidence`, `claim_skill_links`, `posting_requirements`, `requirement_claim_assessments`, campaign action tables.

---

## 5. Design alternatives and tradeoffs

### Path A — Minimal enrichment (recommended first)

- Extend `project` / `experience` payloads (optional): `contribution`, `outcome`, `support_level`, `references[{label, locator, visibility}]`, optional `assertions[{statement, support_level, references}]` kept in JSON.
- `element_skill_links` unchanged; optionally document that tags apply to “ résumé-facing summary” until per-assertion skills needed.
- Application `decision_notes_json` for workflow A.
- Analyzer v2 reads `support_level` when classifying gaps.
- MCP: add read tools for full revision payload.

| Pros | Cons |
|------|------|
| No heavy migrations; fits revision-append model | Weaker query “all claims touching requirement X” |
| Fast UX win for interview workflow | JSON blobs harder to validate in SQL |
| Easy export in existing YAML | Later normalization requires migration script from JSON |

### Path B — Normalized evidence slice (`02` as written)

- First migration: claims + evidence + link tables; UI claim list per element.

| Pros | Cons |
|------|------|
| Reusable granular assertions; clean MCP | Large surface: CRUD, UI, privacy rules, tests |
| Strong match to `00` operating model | Risk to submission/catalog stability during build-out |
| Better multi-posting reuse | Overkill before one end-to-end campaign proves fields |

### Recommendation

**Path A now**, with a **defined JSON → normalized migration** once ≥2 applications use the same assertion across requirements. Path B when analyzer v2 still cannot express “this duty ↔ this proof” with payload assertions.

**Explicit deferrals:** public portfolio (G), campaign actions, LLM inside `jobdesk serve`, automatic verify promotion, requirement matrix tables, submission history UI (E post).

---

## 6. Minimum viable vertical slice

**User story:** As a job seeker, I capture a **self-described research project** with contribution and an unverified outcome, link optional skills, paste a **target posting**, record **desirability/feasibility notes**, run **analyze** to see taxonomy gaps vs documentation gaps, tune composition, and **Submit**—or mark a past send as **external** without a fake manifest.

**End-to-end steps**

1. Portfolio → new `project` → fill narrative + new optional fields → Save (revision 2+).
2. Evidence tab → assert skills (optional).
3. Applications → add posting → paste **full posting text** → Save.
4. Decision notes panel → save desirability/feasibility (new JSON field).
5. Analyze → read gap buckets → adjust composition / portfolio.
6. Submit **or** set external-submission flag (J.1) for historical applies.

**Data model / API (minimal)**

- Payload schema extension in `domain/validation.rs` + theme optional fields if résumé-visible fields needed.
- `applications.decision_notes_json TEXT DEFAULT '{}'` (migration v16).
- `applications.submission_origin TEXT` + `snapshot_status TEXT` (J.1)—**no change** to existing manifest rows.
- `GET/PUT` application decision notes; analyze consumes support labels.

**Privacy**

- `references[].visibility` excluded from any future public export; never auto-inject locators into PDF.
- MCP continues truncating large payloads.

**Acceptance tests**

- Project with `support_level: self_described` survives restart and export/import round-trip.
- Analyze reports **documentation_gap** when skill text appears in `details` but no taxonomy link (heuristic, optional).
- Legacy Jobright row: flagged **external/missing snapshot**; UI can Save as `draft` without creating manifest.
- Existing golden PDF + submit manifest tests still pass.

---

## 7. Submission-history audit finding

**Observation (verified on live DB):**

- Application `7fe40e06-7964-44df-9910-56009347f926` (Jobright): `phase = submitted`, `status = submitted`, **`submissions` count = 0**, `posting_text` empty.
- Matches `workspace/portfolio-export-for-agent.txt` lines 120–136.

**Cause (code-backed):**

- Schema v4 migration (`migrate.rs`) set `phase = 'submitted'` for all rows where legacy `status != 'draft'`, **without creating submission rows**. This predates Milestone J guardrails.
- Current `reconcile_phase_status` **blocks new** invalid states but **does not repair** legacy rows (`docs/MILESTONE_J_RESUME_LIBRARY.md` → J.1 deferred).
- UI warns: `phase/submission mismatch — save as draft or Submit` (`90-applications.js`).

**Not evidence of:** export bug alone—the live SQLite DB shows the same inconsistency.

**Non-destructive recommendation**

1. Add explicit semantics: `submission_origin = external | jobdesk`, `snapshot_status = missing | attached` (manifest path optional).
2. Migration v16 **labels only** mismatched rows—do **not** auto-generate manifests or relabel as JobDesk submits.
3. UI: one-click “Record as external application” sets origin + keeps pipeline status.
4. Document that `application set-status interview` on zero-submission rows **now fails** reconcile (expected)—user must fix origin first.

**Unresolved without owner input:** whether Jobright was applied outside JobDesk (likely) vs data loss of a manifest file.

---

## 8. Risk register

| Risk | Mitigation |
|------|------------|
| Migration v16 breaks legacy apps | Label-only + reversible columns; test on DB copy |
| Payload JSON sprawl | Strict optional schema; validate on save |
| Overclaiming in analyze | Never auto-upgrade support_level; copy distinguishes taxonomy gap vs doc gap |
| LLM trust | No in-binary LLM; MCP read-only; proposals outside DB until Save |
| Public export leaks | Defer G until visibility rules; references default `private` |
| Review burden | Start with one project (Cybersecurity Lab) before bulk import |
| `element_skill_links` coarseness | Accept for v1; defer normalized claims |
| CLI `set-status` on legacy rows | Document + external origin path |

---

## 9. Proposed implementation sequence

| Increment | Deliverable | Stop condition |
|-----------|-------------|----------------|
| **0** | J.1 submission integrity schema + UI + tests on copy of `jobdesk.db` | Legacy Jobright row classified; no fabricated manifests |
| **1** | Payload narrative fields + Portfolio forms | One research project with contribution/outcome/support saved |
| **2** | `decision_notes_json` + Applications UI + MCP read | One posting with desirability notes persisted |
| **3** | Analyzer v2 gap buckets + posting_text validation hint | CrowdStrike-style posting produces actionable report with saved text |
| **4** | MCP `get_catalog_element` / `get_revision` | Cursor interview saves one NEBULA-enriched revision without new tables |
| **5** | Evaluate normalized claims need | **Stop** if assertions reuse across 2+ campaigns is still unnecessary |

**Stop after increment 3** if goal is “strategic apply/no-apply with honest gaps” without claim ontology.

---

## 10. Open questions for owner

1. **Jobright application:** Confirm external ATS submit with no preserved PDF—should JobDesk record `external` + `snapshot_missing` only, or will you attach a reconstructed manifest later?
2. **Résumé visibility:** Should new narrative fields (contribution, references) appear on PDF, or stay portfolio-only metadata?
3. **Support level vocabulary:** Accept `self_described | source_linked | reviewed | reproduced` instead of `02`’s `candidate/supported/verified`?
4. **First dogfood project:** Cybersecurity Lab vs NEBULA vs JobDesk itself for payload assertion pilot?
5. **Desirability storage:** Freeform JSON vs fixed five dimensions from `00` (task energy, environment, …)?

---

## Compact summary (copy/paste for joint architecture review)

```
JobDesk @ 81193f5 (master): Rust/SQLite local résumé workspace — portfolio revisions, application composition pins, submit manifests, element_skill_links + O*NET taxonomy, deterministic analyze (K), MCP read tools. Schema v15. cargo test: 29 passed (fixtures, not live DB).

Vision fit: D (tailor/submit) strong; B (authoring) OK via bullets+skills but no provenance/contribution fields; C partial (taxonomy gaps only); A weak (no desirability/feasibility/decision).

02 proposal: CapabilityClaim/EvidenceItem/Campaign tables — DEFER; unnecessary for first slice. Prefer payload optional fields (contribution, outcome, support_level, references[]) + application decision_notes_json + analyzer v2 gap typing. Hybrid: normalize claims later when reuse across postings proves need.

Submission audit: App 7fe40e06… phase=submitted, 0 submissions — legacy v4 migration + external apply; UI warns; reconcile blocks new invalid states. Recommend J.1 external_submission metadata, no fake manifests.

Next increments: (0) J.1 integrity (1) payload narrative (2) decision notes (3) analyzer v2 — STOP before claim tables unless reuse demanded.

Open: Jobright external confirm; PDF visibility for new fields; support_level enum; pilot project; desirability schema shape.
```

---

**Review gate:** No implementation code or migrations were made in this pass. Awaiting approval before increment 0.
