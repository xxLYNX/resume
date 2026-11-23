---
title: JobDesk — Repository-First Alignment and Design Review
date: 2026-11-06
status: handoff-for-discovery-only
---

# JobDesk — Repository-First Alignment and Design Review

## Your assignment

You are working in the **actual JobDesk repository**. Your first task is **not to implement** the attached proposal. It is to inspect the current codebase and its design, compare them against the **product intent** below, and recommend the smallest useful, architecturally appropriate next increment.

Read these as *inputs, not unquestionable specifications*:

1. This document — primary statement of the newly clarified intent and required output.
2. `00_MASTER_OPERATING_MODEL.md` — employment workflows and system boundaries; interpret with the clarifications below.
3. `02_JOBDESK_EVIDENCE_SLICE_SPEC.md` — **candidate architecture**, not an approved migration/schema contract.
4. `portfolio-export-for-agent.txt` — **export snapshot**, not a substitute for the current database or a guarantee of current runtime state.
5. The repository's own canonical `ARCHITECTURE.md`, `JOBDESK.md`, `CHECKLIST.md`, current milestone docs, migrations, domain models, tests, UI, CLI, and MCP interface — these determine what has actually been built and intended.

The current checkout and real code take priority over descriptions in external reports when they conflict. Document disagreements. The GitHub mirror may lag Forgejo or a local uncommitted checkout. Do not assume a public document proves a current implementation.

## Product vision and corrected interpretations

**JobDesk is a local-first, structured *super-résumé* and employment decision workspace.** Its portfolio is the broad, private source of reusable information about professional experience, projects, education, research, activities, skills and accomplishments. It should support multiple projections: tailored résumés, job applications, interview preparation, and eventually a curated personal website or public portfolio. A portfolio entry need not be currently résumé-worthy or backed by a reproducible artifact to be worth recording.

The major workflows are **user activities that JobDesk should support**, not necessarily a request for four heavyweight autonomous agent pipelines:

- **A — Opportunity assessment:** “Would I want to do this work?” and “Am I plausibly able to do this work, now or with bounded preparation?” Assess desirability and feasibility separately. Employer text may be ambiguous; inferred duties must be labeled as such. Include practical constraints; don't reduce desirability to skill match or automatically generate an authoritative scalar rank.
- **B — Portfolio discovery and authoring:** Capture what the user actually did and can explain, including things not present in repositories. Expect an optional LLM-assisted **interview** that elicits role, contributions, technical decisions, context, outcomes, uncertainty, and omitted projects; optionally review supplied code, research or deliverables. Summaries and links are useful. There will always be some implicit trust in user-supplied statements. **Do not make externally verifiable proof a prerequisite for portfolio entry.** Distinguish self-report, artifact support, and independently checked observations without treating any of them as infallible.
- **C — Gap analysis and targeted development:** Compare opportunity requirements with the full portfolio and identify (i) actual skill gaps, (ii) capability present but poorly represented, (iii) unknown/uncertain areas, (iv) structural/hard constraints. Then optionally prioritize learning or a narrowly scoped demonstration. **Analysis is the immediate product need; automatic training-plan orchestration is not.**
- **D — Presentation, application and feedback:** Select and adapt a subset of the master portfolio for a specific role or publication context, produce a truthful tailored résumé, preserve the exact submitted result, support outreach/interview preparation, and record actual outcomes. A website projection is a legitimate future consumer; do not force its implementation into this milestone.

**Critical conceptual distinction:** A project's narrative description, a user-asserted capability, an artifact/reference that supports it, and a independently reproduced result are distinct, but they need not each become a heavyweight object or required workflow step. Prefer a useful, flexible authoring experience over a burdensome forensic certification pipeline. A portfolio entry can be informative at any support level; stronger support should affect how confidently the system recommends the claim for a high-stakes application, not whether the entry can exist at all.

The existing skills taxonomy is a *reference vocabulary*, not a list of the user's acquired skills. LLM-generated suggestions and keyword matches must remain proposals until reviewed; do not treat the absence of a taxonomy link as proof of incapability.

## Non-negotiable engineering constraints

- **Discovery/review only in this pass.** Do not edit source, apply migrations, change the database, commit, rebase, push, install dependencies, or alter remotes. You may run read-only discovery and tests where safe; report any tests that could mutate an active workspace before running them.
- No fabricated achievements, interviews, benchmarks, deployment claims, employer outcomes, or skill ratings.
- Preserve existing catalog revision history, composition behavior, submission snapshots, privacy boundaries and data compatibility.
- Do not silently migrate or “repair” existing submitted-application records. Determine what lifecycle semantics actually mean first.
- Do not invent a mandatory external LLM service. An LLM-assisted interview should be an optional layer above deterministic stored data, with reviewed proposals rather than automatic promotion of capability assertions.
- Keep the application workflow usable during development. Do not expand this into a generalized knowledge graph, a new scheduler, or a compulsory evidence-verification service.
- Do not add agent co-author trailers to commits. **No commits are authorized in this phase.**

## Inspection procedure

### 1. Establish what is real

Read the active repository documentation and inspect the corresponding implementation, including:

- Portfolio element kinds, payload schemas, editable descriptions, revisions, and any existing optional fields for narratives, contributions, outcomes and links.
- Skills catalog / O*NET nodes, `element_skill_links` and how skill assertions enter résumé outputs.
- Application posting ingestion, analyzer output, match explanations and source retention.
- Application lifecycle, external/submitted status versus immutable snapshots.
- Data access layers, schema version, migrations, export/import, public projection/privacy filtering.
- UI, CLI, existing MCP tools and opportunities to add an optional interactive interview without rewriting core flows.
- Canonical tests and current behavior; report which you actually executed.

Ground all findings in file paths, symbol names, migration IDs or short code references. Identify version/branch/commit and whether local changes exist; do not modify them.

### 2. Walk through four real use cases in the *current* system

1. **Portfolio entry without proof:** Add a historical research project from the user's recollection, with narrative description, individual contribution and an unverified outcome. What fields are already available? What breaks or feels awkward?
2. **Portfolio enrichment via conversation:** The agent interviews the user about NEBULA, inspects a supplied configuration or log, and proposes a concise summary plus several skills/claims. What would the agent return for the user to review, and how would JobDesk store the approved result? No real transcript/host access is required in this review.
3. **Opportunity analysis:** Ingest a CrowdStrike-like endpoint engineering posting, ask about desirability, compare its requirements against confirmed and unconfirmed portfolio information, and distinguish actual gaps from missing evidence. What already works? What is missing?
4. **Projection:** Produce a tailored résumé or filtered public profile from the full master portfolio, preserving its source and revision history without leaking private supporting material.

Show the **current path**, the **minimum missing capability**, and any important tradeoff for each. Do not claim a capability is missing without checking the code.

### 3. Challenge `02`, explicitly

Create a two-column inventory: each proposed new table, field, workflow, status, UI step and MCP tool versus an existing equivalent or adjacent capability. Classify as:

- Already exists — reuse as is.
- Exists but needs a narrow extension.
- Truly absent and necessary for first usable iteration.
- Potentially valuable later.
- Misaligned, over-modeled or undesirable.

Specifically evaluate whether `CapabilityClaim`, `EvidenceItem`, and many-to-many link tables are needed **immediately**, or whether extending current portfolio payloads, descriptions, skill links, and optional reference records achieves the first goal with less migration risk. If you recommend normalized claims, explain why simpler alternatives fail. Consider how a later migration could preserve data collected in a simpler initial system.

Do not assume the `candidate → supported → verified` sequence measures truth adequately. Explore simpler provenance labels (self-described, source-linked, reviewed/reproduced) as possible alternatives, and separate confidence in an assertion from evidence of implementation/deployment.

### 4. Propose choices, not one assumed implementation

Compare at least two viable approaches:

- **Minimal enrichment:** extend the existing portfolio schema/UI, optional summaries/structured contributions/links, and reviewed skill associations. Use the analyzer as far as possible without a new evidence ontology.
- **Normalized evidence slice:** use first-class claim/evidence records only for cases that benefit materially from independently reusable, granular assertions.

You may propose a **hybrid** if it demonstrably minimizes complexity while allowing later expansion. Explain migration implications, change surface, UX, LLM boundaries, and what first end-to-end workflow becomes possible for each. Recommend one and identify explicit *deferrals*.

### 5. Handle the submitted-application inconsistency as an investigation

The supplied text export lists an application in `submitted` phase/status without a frozen submission manifest. Inspect whether the current code intentionally allows externally submitted applications; distinguish a real lifecycle problem, incomplete export, legacy record, or expected behavior. Provide a **non-destructive** recommendation only. Do not auto-create a fictitious manifest or relabel historical data without verification.

## Required output (review gate)

Produce a single Markdown **architecture alignment report** with:

1. **Executive conclusion** (max ~250 words): what already exists, what's missing for the corrected vision, recommended smallest slice.
2. **Current implementation map**: paths, schemas, services, UI and tests; note verified observations versus assumptions.
3. **A/B/C/D workflow matrix**: current capability / missing / simplest change / later work.
4. **Proposed `02` reconciliation**: each significant proposal labeled reuse, revise, defer or reject; explicitly flag unnecessary database tables.
5. **Design alternatives and tradeoffs**: minimum two paths, plus recommendation.
6. **Minimum viable vertical slice**: user story, end-to-end steps, only necessary data model/API/UI changes, privacy considerations and acceptance tests.
7. **Submission-history audit finding**: observed cause or unresolved diagnostic paths; no destructive repairs.
8. **Risk register**: migrations, backward compatibility, overclaiming, LLM trust, public/private projection, review burden.
9. **Proposed implementation sequence**: small independently testable increments, with a stop condition after the first useful workflow.
10. **Open questions for owner**: only decisions that materially change design; don't ask for details the code already answers.

Finally, include a **copy/paste-ready compact summary** that can be sent back to ChatGPT for a joint architecture review.

**STOP after the report. Ask for approval before writing any implementation code or migration.**
