# Proposed portfolio entry: JobDesk

**Status:** DRAFT — owner review required; do not import or modify the canonical portfolio automatically.

This proposal is based on the owner's interview and the coding agent's architectural/implementation reports. Repository features are reported by the coding agent and have not been independently audited for this document. The owner's role in directing substantial agent-assisted implementation should not be rewritten as independent authorship of all source code.

## Element metadata

| Field | Proposed value |
|---|---|
| `id` | `project.jobdesk` |
| `kind` | `project` |
| `name` | JobDesk |
| `date_range.start` | **Confirm with owner or dated repository history; do not guess** |
| `date_range.end` | Ongoing |

## Résumé-facing `details[]` — candidate bullets

Choose and tailor a subset for each application; these are **proposals**, not a claim that the owner personally wrote every subsystem.

- Conceived and directed agent-assisted development of JobDesk, a local-first Rust/SQLite workspace for structured portfolio management, tailored résumé composition, and job-application tracking.
- Developed the product's underlying information model around a canonical portfolio and purpose-specific résumé outputs, informed by earlier work on the FRACTAL planning system.
- Identified shortcomings in legacy project-technology formatting and directed a transition toward individually selectable skill/technology associations for tailored résumés.
- Diagnosed recurring résumé-preview failures at the architectural level by directing an inventory of rebuild triggers and rendering paths, identifying inconsistent pipeline usage as a contributing factor.
- Directed evidence-based engineering reviews of lifecycle behavior, rendering consistency, revision/submission invariants, and MCP portfolio retrieval, using code traces and regression tests as acceptance criteria.

These emphasize product and engineering direction. For an implementation-heavy Rust role, a different entry may be appropriate **only after** verifying the owner's hands-on implementation and ability to explain the relevant source code in detail.

## `portfolio_narrative` — Markdown

### Origin and problem discovery

JobDesk originated as an experiment in a self-owned, Overleaf-like résumé workflow. After a recruiter recommended exploring a LaTeX template in place of an existing Word résumé, I started with small Markdown content files and a Go-based command-line program that produced a LaTeX/PDF résumé. There was no browser editor or live preview.

At the same time, I tracked job applications using a spreadsheet. Its repetitive data entry and tendency to drift out of sync with application status were persistent frustrations. Existing platforms offered useful ideas, but I did not want to depend on expensive subscriptions or on AI résumé tools that attempted to optimize my credentials without adequately understanding my experience.

I gradually added application tracking to the same repository. That prompted broader questions: how can I remember and represent the work I have actually done, reuse that information across very different postings, and identify real gaps in my candidacy rather than merely adjust wording for keyword matching?

### Conceptual model

A major conceptual influence was my work on FRACTAL. I came to view a résumé much as I view a schedule: the output of a model, not the underlying model itself.

The portfolio is intended to be a broad, accurate record of experience, projects, decisions, and capabilities. A résumé is a selected representation of that information for a particular purpose. The goal is not to manufacture an idealized professional persona, but to make legitimate work easier to discover, explain, and present.

That distinction informs the separation between canonical portfolio entries and application-specific résumé compositions. A further objective is to compare opportunities against actual capabilities and use identified weaknesses to guide practical development before applying.

### Technical direction and development method

I initially commissioned a rough prototype through coding agents because the requirements were unclear and my experience with web development was limited. I evaluated suggested technologies and crates, directed the overall design, and corrected approaches that did not fit the intended behavior. Coding agents produced substantial portions of the implementation; I do not claim independent authorship of every crate or feature.

I selected Rust as the direction for the larger application, partly because I wanted a language suitable for an uncertain future scope and had been dissatisfied with the earlier Go/LaTeX workflow's speed. I did not independently establish that Go itself was responsible for the compilation bottleneck, nor that switching languages caused a measured speedup.

I selected SQLite rather than relying on separate JSON files because I wanted a portable, structured store, controlled application workflows for modifying data, and relational indexing/querying capabilities. Those capabilities later became particularly useful for associating portfolio entries with skills and occupational taxonomy information.

The implemented application, as described in repository and agent reports, now has a Rust workspace, SQLite-backed portfolio entries and revisions, résumé composition and LaTeX/PDF rendering, manually maintained application tracking, skill associations, deterministic posting analysis, and read-only MCP portfolio retrieval.

### Engineering challenges and interventions

**Project-technology representation.** An earlier design combined a project name and its technology stack into a single presentation-oriented string. I wanted technologies to be distinct, selectable associations so that a tailored résumé could include only the relevant items. Coding agents repeatedly preserved aspects of the legacy representation despite detailed direction. I intervened to clarify the desired information model rather than accept further cosmetic changes to the old format.

**Recurring preview failures.** Preview rendering repeatedly malfunctioned during view changes and related interactions. After multiple fixes failed to provide lasting confidence, I halted feature work and directed an investigation of which events triggered rebuilds and whether those events shared a consistent rendering pipeline. The investigation found that they did not, which contributed to recurring problems and prompted architectural review. I do not recall every historical defect or the exact scope of the subsequent refactor; these details should be recovered from code and agent records rather than reconstructed from guesswork.

**Architectural acceptance and maintainability.** As the application evolved, I became concerned about coding agents accumulating compatibility layers and locally correct fixes without addressing root causes. I directed a series of reviews requiring precise call-path evidence, targeted tests, and explicit distinctions between confirmed defects, architectural liabilities, and acceptable tradeoffs. The latest reported implementation centralized application lifecycle rules and expanded read-only MCP portfolio access. Those changes were implemented substantially through coding agents under my direction and reviewed through their test and validation reports.

### Current utility and limitations

JobDesk has already been used to produce several résumés and has helped me identify projects and skills missing or underrepresented in my portfolio. It is functional, but not yet my complete employment workflow.

Currently, I read postings myself, use ChatGPT as a second perspective on desirability and qualification gaps, decide whether I can make a credible application, and consider targeted study or project work to address weaknesses. I still manually transfer most posting information into JobDesk and manually maintain application progress.

The application has **not** yet established an automated end-to-end ingestion, assessment, or submission workflow, and I have not established measurable improvements in hiring outcomes.

### Planned direction (not implemented capability)

The next major objective is to accept a job-posting URL, retrieve and preserve the published posting, extract structured information such as requirements, location, and compensation when available, and use those facts to support opportunity and capability-gap analysis. Reusable ATS adapters and on-demand site-specific extraction are approaches under consideration, not completed functionality.

The longer-term objective is a single, self-owned system joining portfolio discovery, credible résumé creation, job-opportunity assessment, targeted capability development, and application tracking.

## Skill and technology association review

**Technologies present in the project (subject to repository confirmation):** Rust, SQLite, Go (early prototype), LaTeX, JavaScript/browser UI, MCP, O*NET terminology. These are *project technologies*, not automatic attestations of the owner's proficiency in each.

**Owner-reported responsibilities suitable for assessment:** requirements elicitation, product modeling, architectural direction, technical specification and review, systematic debugging strategy, relational-data design rationale, AI-assisted software development oversight.

Do **not** automatically add all technologies as competency links. Select associations the owner can explain and defend, and resolve real IDs from JobDesk's actual taxonomy.

## Review and import checklist

- [ ] Confirm project start date from the owner or history; do not use the former arbitrary January 2025 placeholder.
- [ ] Review résumé bullet wording for authorship accuracy and desired job targeting.
- [ ] Check repository facts and correct any implementation detail that is merely agent-reported.
- [ ] Confirm the owner is comfortable including the recruiter feedback and AI-assisted development description in a private long-form history; tailor public presentation separately.
- [ ] Choose appropriate taxonomy links without assuming tool use equals proficiency.
- [ ] Save `portfolio_narrative` and the approved `details[]` to a new `project.jobdesk` through normal UI/API after explicit authorization.
- [ ] Confirm save/read/revision history, MCP visibility, and absence of narrative in résumé output.

**Important:** This Markdown file is a human-review proposal, **not** directly a schema-valid JobDesk import payload. Do not import it verbatim.
