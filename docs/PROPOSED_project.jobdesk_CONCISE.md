# JobDesk — Proposed portfolio entry

> **Review draft — not imported into JobDesk.** This document is intended for the `project.jobdesk` catalog element, not as a direct JSON/YAML import file. Personal contributions below are based on the owner's interview; implemented functionality is based on coding-agent architecture reports and the public repository. Review before publication or use on a résumé.

## Element metadata

| Field | Value |
|---|---|
| ID | `project.jobdesk` |
| Kind | `project` |
| Name | JobDesk |
| Date range | **January 2026 – present** |
| Status | Active development; used to generate several résumés |
| Repository | https://github.com/xxLYNX/resume |

**Start-date source:** [First/root commit, January 3, 2026](https://github.com/xxLYNX/resume/commit/235a0036bbe4742499705188623f1bb07b2c02f3), titled *Initial commit: Modular LaTeX resume build system*. This establishes the beginning of the recorded repository history, not necessarily the first local work.

## Résumé-facing accomplishments (`details[]` candidates)

These are alternative bullets to select or tailor, **not** a recommendation to place all of them on one résumé.

- Conceived and directed agent-assisted development of a local-first Rust/SQLite employment workspace for maintaining a structured portfolio, generating tailored résumés, and tracking applications.
- Defined a portfolio-centered information model that separates canonical experience and project records from application-specific résumé compositions.
- Identified limitations in the legacy project-stack representation and directed a redesign toward individually selectable technology and skill associations.
- Investigated recurring résumé-preview failures by directing an audit of rebuild triggers and rendering paths, exposing inconsistent pipeline usage and motivating architectural remediation.
- Directed code-backed reviews and acceptance testing of application lifecycle rules, rendering consistency, submission history, and MCP portfolio access.

## `portfolio_narrative` (Markdown; not résumé-rendered)

JobDesk began in January 2026 as a Go/Markdown/LaTeX command-line résumé builder. It evolved into a local-first Rust/SQLite application for storing professional experience independently of the résumés generated from it. The central design principle, influenced by work on FRACTAL, is that a résumé is a **purpose-specific output of a broader portfolio**, not the underlying record itself.

I defined and iterated the product requirements, chose major technical directions, and supervised substantial coding-agent implementation. Rust was selected for flexibility as the application expanded; SQLite provided portable persistence, structured relationships, and indexing for portfolio revisions, skill associations, and application data. My primary role was product and architectural direction, specification, critical review, and debugging strategy—not independent implementation of every subsystem.

Notable interventions included directing a change from string-based project technology lists toward individually selectable associations, and investigating recurring preview failures by mapping rebuild events to the rendering paths they used. I subsequently directed evidence-driven architectural reviews emphasizing shared business rules, reproducible tests, and avoiding additive workarounds.

The application now supports revisioned portfolio entries, application-specific résumé composition, LaTeX/PDF generation, skill associations, manually maintained application tracking, posting analysis, and read-only MCP portfolio access. I have used it to generate several résumés and identify underdocumented projects and skills. **URL-based job-posting ingestion and more complete opportunity/gap assessment remain planned**, rather than demonstrated end-to-end capabilities.

## Review notes (not part of `portfolio_narrative`)

- The start date is grounded in the first public Git commit; local work could predate it.
- Project technologies are **not** automatically personal skill attestations. Review individual competency associations before linking them in JobDesk.
- Confirm specific feature descriptions against the local repository when entering this record.
- The new entry should be created through the normal JobDesk portfolio UI or supported API only after approval; do not ask an agent to import this Markdown file verbatim.
