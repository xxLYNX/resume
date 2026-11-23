---
title: JobDesk — Evidence and Campaign Slice (Implementation Contract)
created: 2026-11-06
status: proposal-no-code-changes-made
---

# JobDesk — Incremental Evidence and Campaign Slice

## Why this, now

JobDesk already has a Rust/SQLite catalog with append-only revisions, application-specific composition, a skills taxonomy, `element_skill_links`, submission manifests, O*NET posting hints, deterministic analysis and a scoped MCP server (according to public docs and historical agent output). **Do not rewrite these.** The immediate deficiency is that a portfolio entry → `Programming` edge cannot represent what was implemented, how reliable the claim is, which version proves it, whether it ever ran, or whether the user can reproduce it.

**Milestone L candidate:** Evidence provenance and claim review. A minimal new surface must let one existing portfolio project produce multiple accurately bounded claims, each supported by independently locatable evidence and reusable across job requirements.

## Non-goals of this slice

No automatic application submissions; no broad semantic AI judge; no scraping of private hosts; no automatic claims from repository names; no universal knowledge graph; no rewriting O*NET taxonomy; no mandatory LLM in JobDesk; no replacement for Obsidian, FRACTAL or CONSTELLATION. The local DB remains authoritative for career records; repos remain authoritative for code.

## Objects and relationships (contract before migration)

```
PortfolioElement (existing) 1 ── * CapabilityClaim
CapabilityClaim      * ── * EvidenceItem
CapabilityClaim      * ── * SkillNode (existing taxonomy)
Application          1 ── * PostingRequirement
PostingRequirement   * ── * CapabilityClaim (human-reviewed fit, with rationale)
Application          1 ── * CampaignDecision / CampaignAction
Application          1 ── * SubmissionRecord (existing, immutable)
```

### `CapabilityClaim`

Fields: stable id; parent `element_id`; specific behavior statement (“implemented X under Y constraints and observed Z”), `scope/context`, `personal_contribution`, depth 0–5, claim status, claim confidence, assessment date, last independent verification date, reviewer, and optional `limits/known_gaps`. A depth score is **not** an auto-generated truth value.

Claim statuses: `candidate`, `supported`, `verified`, `disputed`, `superseded`, `retracted`. “Verified” means the record names the verification method and the actual source locator; do not infer it because a tool output says `tests passed` in a transcript. Separate **implementation stage** from **claim credibility** (e.g., a fully verified claim can truthfully say “implemented but never deployed”).

### `EvidenceItem`

Fields: id; type (`repo_commit`, `code_range`, `test_run`, `benchmark`, `deployment_log`, `runtime_check`, `writeup`, `publication`, `stakeholder_record`, `demo`, `research_note`); exact source locator; version/hash or commit; observed/captured timestamp; context/environment; summary; optional checksum; visibility (`public`, `shareable_redacted`, `private`); verification method and outcome. Support *negative* evidence or failed assertions, not only successes. Evidence may support several claims, and one claim often requires several independent items.

### `PostingRequirement`

Fields: id, application id, original quoted requirement, source snapshot reference, classification (`responsibility`, `must`, `preferred`, `inferred`, `constraint`), centrality (`critical`, `high`, `medium`, `low`), evidence threshold, ambiguity, and human notes. Inference must be visibly marked.

### `RequirementClaimAssessment`

Fields: requirement id, claim id, relation (`direct`, `adjacent`, `weak`, `not_relevant`), rationale, assessment author (`human` or `agent_suggestion`), confidence, and review status. Inferred lexical/semantic matches are **candidate associations**, not automatic competence ratings.

### `CampaignDecision` and `CampaignAction`

Decision: state (`apply_now`, `timeboxed_prep`, `explore`, `pass`), reason, hard-gate results, desirability dimensions + confidence, feasibility, decision date. Action: description, target evidence/claim/requirement, next gate, owner, status, completed timestamp, captured outcome and optional time cost. Build on existing applications, avoid creating a separate generic task-manager inside JobDesk.

## Schema implementation strategy

1. Inspect current `crates/store` migrations, foreign keys, element revision identifiers and application/submission records. Run backup and `PRAGMA integrity_check` on a **copy** of the actual SQLite DB. The text export cannot substitute for this.
2. Add four new foundations initially: `capability_claims`, `evidence_items`, `claim_evidence`, and `claim_skill_links`. Foreign keys should reference stable existing element/skill identifiers; keep provenance to specific catalog revisions when the claim rests on exact historical wording. Keep a future-ready `source_version` field.
3. Add `posting_requirements` and `requirement_claim_assessments` *after* manual claim review is usable. Do not create a giant schema migration based only on today's speculative scoring rules.
4. Add an explicit migration audit for application records that are `submitted` with no manifest. Represent a genuine out-of-band application as a distinct recorded event, ideally `submission_origin=external` with `snapshot_status=missing|attached|reconstructed`. Existing immutable JobDesk submissions must remain immutable.
5. `GET` query projections should return all claims with supporting evidence IDs and state, without leaking private proof into public exports. Support both human-friendly Markdown and JSON exports.
6. Extend the existing MCP interface with **read-only** `list_claims`, `get_claim`, `get_evidence`, `get_requirement_matrix` before permitting writes. On later write tools, require explicit human review to promote claims to `verified`.

### Example *conceptual* record (not a verified claim)

```yaml
id: claim.nebula.pluto.ssh-role-hardening
portfolio_element: project.cybersecurity-lab
statement: "Configured role-specific SSH hardening for the NixOS workstation"
implementation_stage: reported_implemented
claim_status: candidate
scope: "pluto role; not all hosts"
contribution: "needs independent review"
source_leads:
  - kind: repo_file
    locator: "nebula/roles/pluto/flake.nix"
  - kind: historical_agent_audit
    locator: "cursor-agent-undertakings-report.md#L42551-L42565"
next_verification: "Review current source, evaluate configuration, and verify the effective sshd configuration on pluto"
visibility: shareable_redacted
```

This has more hiring value than tagging the entire lab with `Programming` because the claim can be challenged and re-verified.

## User workflow — minimum shippable vertical slice

`Portfolio → Cybersecurity Lab → Add claim → Add evidence locator → Review → Verify → Link skills → Expose to application analysis → Show trace in résumé composition`.

The first UI can be a simple claim list and detail form. Do **not** build a graph canvas, generalized ontology editor or new LaTeX engine. Do not automatically publish evidence URLs in PDF; link curated public proof when appropriate.

## User-visible validation criteria

- One project has **three distinguishable claims** with different states (candidate/supported/verified), and their statuses survive a restart.
- One claim links to **two** evidence items; one evidence item can support **two** claims. Deleting an unapproved link does not destroy the underlying source record.
- A claim can distinguish “code exists,” “test passed,” and “runs on host,” without accidentally upgrading one to another.
- An agent suggestion cannot change a claim to verified on its own.
- A skill or requirement appears as *supported* only when its linked claim is credible within the relevant scope; unrelated matched taxonomy tokens cannot become proof.
- The application analyzer shows which **specific claims and evidence** support critical requirements, plus unresolved gaps; it must explain its basis.
- Historic application with `phase=submitted` and no manifest is flagged as an unresolved external submission, not falsely represented as a frozen JobDesk submission.
- Public export does not leak private evidence locators or raw host addresses.
- Existing PDF golden test, catalog revision tests, `cargo test`, submission manifest behavior, and existing `application analyze` remain functional.

## Implementation issues — in order

1. **L0 — Historical data integrity:** test/report inconsistent phase-vs-manifest cases; define out-of-band submission semantics; backup/export/recover exact source of existing claims.
2. **L1 — Claim model:** migrations and CRUD with appendable validation events. Ground claims in existing portfolio anchors.
3. **L2 — Evidence model:** locator/version, many-to-many linking, visibility, manual verification, stale/superseded records.
4. **L3 — One UI workflow:** exercise only the Cybersecurity Lab (three granular claims), then JobDesk and PHP evidence packets.
5. **L4 — Requirement links:** map one current job posting to claims with explicit rationale; unknown requirements remain unknown.
6. **L5 — Analyzer/MCP projections:** read-only agent tools, transparent grounded reasoning, privacy-safe export.
7. **L6 — Optional later:** campaign prioritization dashboard and interest-experiment log **after** actual usage demonstrates which fields matter.

## Concurrency / AI / privacy contract

Keep the deterministic career data in JobDesk. The agent may propose links or draft explanations but must cite source records, expose uncertainty and request human promotion. Preserve commit IDs and timestamps. Avoid feeding secrets, private system inventories, malware samples, client code, or unredacted operational telemetry into external services. For public output, create explicit redacted evidence summaries.

## Existing integration examples to respect

- `cargo run -p jobdesk -- application analyze <id> --json`
- `cargo run -p jobdesk -- mcp serve --workspace workspace/`
- `cargo test`
- Forgejo `origin` primary; GitHub is documented as a read-only mirror. Do not rely on GitHub Issues as JobDesk's canonical issue tracker.

No database or repository change is claimed in this proposal.
