# JobDesk operating model

**Status:** Canonical product vocabulary for operational use (2026-11-12).  
**Scope:** Conceptual entities and workflows. This document does **not** assert that every concept has a first-class database table or API.

---

## Purpose

JobDesk supports a systematic employment search: preserve authoritative evidence, capture employer publications faithfully, project tailored résumés, and track candidacies without conflating research artifacts with applications.

---

## Conceptual entities

### Portfolio

The **portfolio** is the authoritative record of the candidate’s experience, projects, education, certifications, awards, courses, and supporting material.

- Portfolio records may contain **more** than any single résumé should show (e.g. long-form project `portfolio_narrative`).
- **Skill and technology tags** (`skill_nodes`, links on elements/revisions) aid discovery and alignment hints. They are **not**, by themselves, proof of demonstrated competence.
- Portfolio data is edited in place via **revisions**; the catalog is not duplicated inside application rows.

**Implemented today:** `elements`, `element_revisions`, skill link tables — see [`ARCHITECTURE.md`](../ARCHITECTURE.md).

---

### Opportunity

An **opportunity** is the underlying employment position the candidate is considering.

- One opportunity may be described by **multiple postings** (employer site, ATS, LinkedIn, USAJOBS, etc.).
- Postings can disagree on title, location, compensation, or duties. Those discrepancies are **information**, not bugs to auto-merge away.
- JobDesk does **not** currently store a first-class **opportunity** entity. The concept guides how users should think about saved postings and tracker rows.

**Not implemented:** `opportunities` table, `posting_links`, automatic deduplication by employer/title/location, or `applications.posting_id` (explicitly deferred in `ARCHITECTURE.md`).

---

### Posting

A **posting** is one **source-specific publication** the user chose to save — identified by a normalized URL and its own history.

Each posting has:

- **Retrieval history** — immutable snapshots and fetch attempts when supported.
- **Machine interpretation** — extraction outcome and suggested fields.
- **User corrections** — overrides and optional `reviewed_at`.

A posting is **evidence about** an opportunity, not the opportunity itself. Saving a posting does **not** create an application.

**Implemented today:** `postings`, `posting_snapshots`, `posting_interpretations`, `posting_fetch_attempts` (schema v17); Opportunities UI and `POST /api/opportunities/ingest`.

---

### Requirements and assessment

**Requirements** are statements derived from postings the user has captured and **reviewed** (employer text, not silent inference).

An **assessment** compares those requirements against the **entire portfolio**, surfacing:

| Status | Meaning |
|--------|---------|
| Demonstrated capability | Relevant evidence exists and supports the claim. |
| Related but incomplete | Evidence is adjacent or thin for the stated requirement. |
| No portfolio evidence found | Search found nothing supporting the requirement (not proof of inability). |
| Established capability gap | Independently known gap (e.g. hard constraint, missing credential), not inferred from absence alone. |
| Unresolved / ambiguous | Employer wording unclear or requirement not yet reviewed. |

**Missing evidence ≠ missing ability.**

**Implemented today (partial):** Application-scoped **analyze** and **posting hints** (`application_analyze`, `posting_hints`) — keyword/skill alignment against **draft composition** and pasted `posting_text`, not structured requirements or full-portfolio assessment. See [`docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md`](MILESTONE_K_APPLICATION_INTELLIGENCE.md).

**Not implemented:** Requirement rows, passage citations, portfolio-wide posting analyze, user-reviewed requirement lists tied to opportunities.

---

### Résumé (composition and artifact)

A **résumé composition** is a structured **projection** of the portfolio for a particular pursuit:

```text
ResumeComposition = Select(Portfolio, OpportunityRequirements, PresentationConstraints)
ResumeArtifact     = Render(ResumeComposition, Template)
```

- The composition selects catalog elements, section order, and inclusion toggles. It is **not** a second authoritative biography.
- One pursuit may have **multiple compositions** over time (draft edits, presets, cloned applications).
- **Submitted artifacts** must remain identifiable and reproducible (manifest + content hash).

**Implemented today:** `applications.draft_composition_json`, `presets.composition_json`, composition editor UI, preview/build, **Submit** → `submissions` with manifest path and `content_hash`. Multiple submissions per application are stored; canonical flag exists on submissions.

**Not implemented:** Named composition revisions per application separate from the single live draft JSON (use presets or clone applications as workarounds).

---

### Application

An **application** is an **intentional** attempt to pursue an opportunity: tracker metadata, draft composition, optional pasted posting text, lifecycle phase/workflow, and submission history.

- Created only when the user (or CLI) creates an application — **never** from posting ingest (enforced in code and tests).
- Tracks pipeline after submit (`phase`, workflow `status`, submissions).

**Implemented today:** `applications`, `submissions`; lifecycle rules in [`docs/APPLICATION_LIFECYCLE.md`](APPLICATION_LIFECYCLE.md).

**Not implemented:** Automatic link from saved opportunity posting to application (`posting_id` deferred).

---

## Entity relationship (conceptual)

```text
Portfolio (authoritative)
    │
    ├── Evidence links (skills, narratives, revisions)
    │
Opportunity (conceptual; user mental model)
    │
    ├── Posting A (URL, snapshots, interpretations) ──► Requirements (future / manual today)
    ├── Posting B (…)
    │
    ├── Assessment (future / partial via application analyze today)
    │
    ├── Résumé composition(s) ──► Render ──► Artifact(s)
    │
    └── Application (tracker, submit, outcomes)
```

---

## Mapping: concept → database (honest)

| Concept | DB / API today | Notes |
|---------|----------------|--------|
| Portfolio element | `elements`, `element_revisions` | Kinds: header, objective, experience, education, project, course, certification, award |
| Skill tag | `skill_nodes`, `element_skill_links`, `revision_skill_links` | Hints only until tied to narrative evidence |
| Opportunity | *(none)* | User correlates postings manually |
| Posting | `postings` + children | One row per normalized URL |
| Requirement | *(none)* | Use reviewed `description_plain` + external notes |
| Assessment | `GET …/analyze` on **application** | Not posting-scoped; not full portfolio |
| Résumé composition | `draft_composition_json`, `presets` | One draft per application |
| Résumé artifact | `submissions`, workspace manifests/PDFs | Submit freezes manifest |
| Application | `applications` | Independent of `postings` |

---

## Invariants (do not violate in operations)

1. **Ingest ≠ apply** — Opportunities ingest never creates `applications` rows.
2. **Submit ≠ edit catalog** — Submit freezes what was sent; portfolio edits do not rewrite past manifests.
3. **Posting URL ≠ opportunity** — Do not merge postings without explicit human review.
4. **Tag match ≠ competence** — Treat analyze/hints as decision support, not certifications.
5. **Absent evidence ≠ absent skill** — Gaps in the report mean gaps in **documented** evidence.
6. **Manual paste ≠ verified capture** — Pasted `posting_text` on an application has no snapshot provenance unless the user also saved a posting URL through Opportunities.

---

## Related documents

- [`ARCHITECTURE.md`](../ARCHITECTURE.md) — implementation checklist  
- [`docs/reviews/NEXT_MILESTONE_DISCOVERY.md`](reviews/NEXT_MILESTONE_DISCOVERY.md) — evidence/requirements gap analysis (design)  
- [`docs/APPLICATION_LIFECYCLE.md`](APPLICATION_LIFECYCLE.md) — phase and workflow  
- Informal prior draft: [`00_MASTER_OPERATING_MODEL.md`](00_MASTER_OPERATING_MODEL.md) (not authoritative; broader employment-acquisition system)
