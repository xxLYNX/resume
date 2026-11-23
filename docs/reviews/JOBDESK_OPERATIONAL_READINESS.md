# JobDesk operational readiness review

**Date:** 2026-11-12  
**Assignment:** Operational use readiness — **no code or schema changes**.  
**Inspection:** Repository source, `workspace/jobdesk.db` (read-only SQL), [`workspace/portfolio-export-for-agent.txt`](../../workspace/portfolio-export-for-agent.txt) (2026-11-06 export; may be stale vs live DB).

---

## 1. Git publication (context)

Milestones L/M commits may still be unpublished to Forgejo if push auth failed in the agent environment. Operational use does **not** depend on remote publication; local `workspace/jobdesk.db` and binary from the repo are sufficient. Verify with `git log origin/master..HEAD` on your machine if needed.

---

## 2. End-to-end workflow: Discover → Track

Legend: **Works** = implemented and usable in `jobdesk serve` today; **Partial** = significant manual steps; **No** = not implemented.

| Stage | Implementation | User interaction | Status | Workaround | Blocker? |
|-------|----------------|------------------|--------|------------|----------|
| **Discover** | User finds URL off-tool | Browse employer boards, LinkedIn, etc. | Works | — | No |
| **Capture** | `POST /api/opportunities/ingest`, Opportunities tab (`86-opportunities.js`) | Paste URL → Fetch & save | **Partial** | Greenhouse job-board URLs use API adapter (`greenhouse_ingest.rs`); other hosts use generic HTML (`posting_fetch.rs`) | Sites blocking server fetch or without JSON-LD → partial/failed extraction |
| **Review** | `PATCH /api/opportunities/{id}/interpretation`, UI review | Edit fields, Save, Mark reviewed | Works | Manual fill when `partial`/`failed` | No |
| **Assess** | `GET /api/applications/{id}/analyze`, `posting_hints` | Requires **application** + pasted `posting_text` + **draft composition** | **Partial** | Copy reviewed `description_plain` to application; run Analyze in Applications tab (`90-applications.js`) | **No posting-level or full-portfolio assessment** (`NEXT_MILESTONE_DISCOVERY.md`) |
| **Select portfolio evidence** | Portfolio tab + composition editor; skill links on elements | Toggle sections/items in draft composition | Works | Use Catalog + skill tags; **portfolio_narrative** not in analyze | Narrative evidence not in automated reports |
| **Compose résumé** | `draft_composition_json`, composition UI, presets, quickfill | Enable/disable elements per app | Works | Clone app or preset (`90-applications.js`, `presets`) | No |
| **Inspect** | `POST /api/preview` draft/submitted, PDF build | Preview iframe in Applications | Works | CLI `jobdesk build --application <id>` | No |
| **Apply** | User acts outside JobDesk | Employer ATS/email | Works (external) | — | No |
| **Track** | `applications` phase/status, `submissions` after Submit | Save metadata, Submit, update workflow | Works | CLI `application set-status` | No |

**Ingest does not create applications** — asserted in `opportunities.rs` tests and Greenhouse ingest paths.

### Tests and entry points (reference)

| Area | Tests / docs |
|------|----------------|
| Opportunities ingest | `crates/server/src/opportunities.rs` (`ingest_tests`), `e2e/opportunities/tests/*.spec.ts` |
| Greenhouse adapter | `e2e/opportunities/tests/greenhouse-ingest.spec.ts`, `crates/domain` greenhouse fixtures |
| Application lifecycle | `docs/APPLICATION_LIFECYCLE.md`, store validation |
| Analyze | `crates/domain/src/application_analyze.rs`, K milestone doc, UI analyze in `90-applications.js` |
| Submit / manifest | `applications::submit_application`, manifest tests in resolve/render crates |

---

## 3. Relationship deep-dives

### Opportunity ↔ postings

**Can JobDesk represent multiple URLs for one opportunity?**  
**No.** Each saved URL is one `postings` row (`normalized_url` UNIQUE). No `opportunity_id` or `posting_links` table.

**What users can do today without losing sources:**

- Save **each URL separately** in Opportunities (correct provenance per source).
- Correlate manually in tracker notes/tags on **applications** (free-text `company`, `title`, `tags`, `source`).
- Use an external note (Obsidian, spreadsheet) for opportunity ID → list of posting IDs.
- Do **not** rely on system merge by employer, title, or Greenhouse `requisition_id` (stored in provenance for that posting only).

### Opportunity requirements ↔ portfolio

**Full-portfolio evidence search for a saved posting?**  
**No.** `store::application_analyze` loads **all catalog elements** for *composition gap* detection but skill alignment is driven by **posting text vs draft composition pins** and catalog skill assertions (`crates/store/src/applications.rs` lines ~401–461).

**Manual assistance required today:**

1. Read reviewed posting description (Opportunities UI).
2. Create application; **paste** description into `posting_text`.
3. Build draft composition including relevant elements.
4. Run **Analyze** and **Posting hints** — interpret `partial` vs `gap` per K doc (catalog vs draft, not “cannot do job”).
5. Manually scan projects with `portfolio_narrative` in Portfolio tab (not in analyze).
6. Optional: MCP `get_catalog_*`, `get_skill_evidence` (`mcp_tools.rs`) in Cursor for grounded search.

### Opportunity ↔ résumé composition

**Tailored composition without duplicating portfolio data?**  
**Yes.** Composition stores **references** (`element_id` per section), not copied payloads (`Composition` type, composition editor).

**Multiple compositions / which was submitted?**

- **One live draft** per application (editable after submit with preview warning).
- **Presets** hold alternate compositions (`presets` table).
- **Multiple submissions** per application possible; each has `manifest_path`, `content_hash`, `is_canonical`.
- **Submitted** artifact is the manifest from **Submit** at that time — not auto-named “v1/v2” in UI beyond submission list.

### Opportunity ↔ application

**Initiate application without ingest side effects?**  
**Yes.** Applications → Create (`POST /api/applications`) or CLI `application add`. Ingest never touches `applications`.

**Information to copy today (no `posting_id` link):**

| From Opportunities | To Application |
|--------------------|----------------|
| Reviewed title, company | `title`, `company` |
| Normalized URL | `posting_url` (optional) |
| `description_plain` | `posting_text` (for analyze) |
| Employment meta if extracted | `employment_type`, `work_mode`, `location`, `compensation` (application meta fields) |

User must **explicitly create** the application after research.

---

## 4. Local workspace state (read-only)

Inspected `workspace/jobdesk.db`:

| Table | Count | Notes |
|-------|-------|--------|
| `postings` | **1** | `https://job-boards.greenhouse.io/flexport/jobs/8009198` (Account Executive — **connector test target, not career fit**) |
| `applications` | **0** | Tracker empty at inspection time |
| `elements` | **15** | Cybersecurity-focused portfolio (education, experience, projects) |

**November 2026 export** (`portfolio-export-for-agent.txt`) references a prior **Jobright Security Analyst** application; that row is **not** present in the live DB (export stale or data reset). Trust live DB counts.

**No saved opportunities** in the workspace match cybersecurity / security research roles for a turnkey exercise. The only saved posting is Flexport sales QA data.

**Minimum input for first real exercise (owner):**

1. One target job URL (prefer Greenhouse boards URL if on Greenhouse; otherwise any public HTTPS job page).
2. Confirmation role is intentional (cyber / adjacent SWE).
3. Optional: off-tool notes if the site blocks server fetch.

---

## 5. First operational exercise (procedure)

**Recommended target:** None in DB. Owner supplies URL, e.g. a **Security Analyst / Detection / Vulnerability Research** posting on a public board.

**Use the following walkthrough** (no autonomous apply, no DB writes in this review).

### 5.1 Capture and review

```bash
cargo run -p jobdesk -- serve --open
```

1. **Opportunities** → paste URL → **Fetch & save**.
2. Open detail → check **extraction outcome** and diagnostics.
3. If Greenhouse job-board URL → expect `retrieval_kind=greenhouse_job_board_api` when API succeeds.
4. Correct title/employer/description → **Save edits** → **Mark reviewed**.

### 5.2 Explicit requirements (manual)

JobDesk does **not** split requirements into rows. Operator:

1. Read `description_plain` and list **must-have** vs **nice-to-have** in a note (Obsidian or interpretation-adjacent scratch).
2. Mark ambiguous lines as **unresolved** (operating model).

### 5.3 Portfolio evidence mapping (read-only, from live catalog)

Use Portfolio tab or export; align to **existing** elements only (IDs from DB / export):

| Portfolio anchor | Relevant themes (from stored text, not invented) |
|------------------|--------------------------------------------------|
| `objective.main` | Cybersecurity analyst positioning |
| `education.*` (GMU BS/MS cyber) | Education requirements |
| `experience.affinity-db-*` | Security audit, Ghidra, Python/VBA review |
| `experience.safeway-*` | Responsible disclosure, operations leadership |
| `experience.knowledge-in-motion-2024-11` | R&D / systems (adjacent if role is pure IR) |
| `project.ghostrace-cve-research` | Vuln research, speculative execution |
| `project.machine-learning-malware-detector` | Malware / ML detection |
| `project.cybersecurity-lab` | SOC-style lab, detection stack, IAC |
| `project.cloud-service-model-risk-assessment` | Cloud risk / NIST |
| `project.biometric-authentication-modalities-research` | Auth research |
| `project.self-driving-car-ba-algorithm-research` | SWE depth (optional for non-ML roles) |

**Asserted skill links today (DB):** Affinity DB → Ghidra, Programming; Cybersecurity Lab → Programming (`portfolio-export-for-agent.txt` — verify in UI **Skills** on element).

**Gaps to expect without automation:** Requirements mentioning SIEM product X, active clearance, years in SOC tier-1, or specific certs may show as **manual unproven** until evidence exists in portfolio — absence is **missing documentation**, not automated “lack of skill.”

### 5.4 Supported claims vs unproven (example process)

After pasting posting into a **new application**:

- **Analyze `evidenced`** — skill in posting and on **pinned** draft evidence.
- **`partial`** — skill in catalog but not on draft (include element or add skill link).
- **`gap` in K report** — no asserted skill link; may still have narrative proof manually.

Cross-check **composition_gaps** for high-value elements omitted from draft.

### 5.5 Learning / demo project decision

If requirements repeat **unaddressed** themes (e.g. production SOAR, specific cloud IR):

- Record as **acquirable** or **timeboxed prep** in external campaign notes (operating model).
- JobDesk does not manage prep campaigns yet — tracker tags only.

### 5.6 Tailored résumé

1. **Applications → New** — copy meta from Opportunities.
2. Paste posting text.
3. Composition editor: enable header, objective, education, selective experiences/projects (prior export omitted biometric/cloud for a security-analyst-style draft — operator choice).
4. **Save** → **Preview** (draft).
5. Optional: **quickfill** from similar app if history exists (currently **none** in DB).

### 5.7 Inspect and track

1. Preview PDF in UI or `jobdesk build --application <id> --pdf`.
2. When ready to record a send: **Submit** → `phase=submitted`, manifest frozen.
3. Update workflow (`interview`, `rejected`, etc.) per [`APPLICATION_LIFECYCLE.md`](../APPLICATION_LIFECYCLE.md).

**Do not submit** to employer from this exercise unless explicitly authorized later.

---

## 6. Friction report (prioritized)

### P0 — Operational blocker

| Issue | User problem | Workaround | Evidence | Smallest fix | Before 1st app? |
|-------|--------------|------------|----------|--------------|-----------------|
| *(none identified)* | Core path: portfolio + create app + compose + preview + submit **works** | — | `applications.rs`, UI 90-*, lifecycle doc | — | — |

**Conclusion:** Nothing in code **prevents** completing a real application if the user accepts manual requirement extraction and posting-to-application copy. Empty application tracker is a **data** state, not a product blocker.

### P1 — Significant friction

| Issue | User problem | Workaround | Evidence | Smallest fix | Before 1st app? |
|-------|--------------|------------|----------|--------------|-----------------|
| **No opportunity entity** | Multiple URLs for one role stay mentally disjoint | Save each URL; manual notes | No `opportunity_id` in schema | Defer; tags + external index | No |
| **Posting ↔ application unlink** | Re-copy description and URL | Manual copy table §3 | `applications.posting_id` deferred | Optional FK + “Create application from posting” UI later | No |
| **Assessment is application-scoped** | Hints/analyze ignore Opportunities row | Paste text into application | `application_analyze` inputs | “Copy from reviewed posting” button or analyze-on-posting-id | No — paste works |
| **Full portfolio not analyzed** | `portfolio_narrative` and off-draft elements ignored by analyze | Manual read of projects | `application_analyze` composition focus | Portfolio-wide analyze milestone | No |
| **Non-GH sites often `partial`** | Weak structured fields | Edit interpretation + raw source view | L closure, Flexport HTML partial | Next connector per playbook | No — manual review |
| **Duplicate experience elements** | Two Affinity DB, two Safeway IDs in DB | Pick one canonical pin in composition | `sqlite elements` list | Catalog cleanup (user edit/merge policy) | No |
| **Stale export vs DB** | Agent/docs reference old applications | Use live DB | 0 applications vs export | Re-export for agents | No |

### P2 — Enhancement

| Issue | User problem | Workaround | Evidence | Smallest fix |
|-------|--------------|------------|----------|--------------|
| Named composition revisions per app | A/B resume variants | Presets + clone application | `presets`, `clone_from` | Composition versioning UI |
| Submission history UX | Hard to compare past sends | CLI manifest paths | ARCHITECTURE deferred UI | List submissions in UI |
| LinkedIn / USAJOBS connectors | Poor generic extraction | Manual paste | L generic path | Playbook adapter when needed |
| Requirements matrix in-app | Spreadsheet outside | External notes | Not implemented | Milestone N (discovery doc) |

### Deferred (inform use first)

- First-class **opportunity** and **requirement** entities.
- Automatic cross-posting deduplication.
- LLM requirement extraction with review gates.
- `applications.posting_id` and ingest-to-application bridge.
- Portfolio-wide fit/gap engine replacing manual matrix.
- Browser-only capture import without URL fetch.

---

## 7. Closing summary

### Usable immediately

- Full **portfolio** CRUD in serve, YAML import/export, skill links, project `portfolio_narrative`.
- **Opportunities** capture (Greenhouse API + generic HTML), review lifecycle, source archive.
- **Applications** tracker, composition editor, preview/PDF, **Submit** manifests, phase/workflow.
- **Analyze / posting hints** on applications with pasted posting text.
- **MCP** read tools for catalog and evidence (`jobdesk mcp`).
- **Quickfill** / presets for composition reuse.

### Handle manually

- Opportunity grouping across multiple URLs.
- Requirement decomposition and portfolio-wide evidence matrix.
- Copy posting → application fields.
- Non-Greenhouse sites with failed/partial extraction.
- Interpreting analyze `gap` as missing **evidence**, not missing **ability**.

### Genuinely blocks first application

**Nothing in the implemented product blocks the first application**, provided the target posting can be captured or pasted and the user performs manual assessment steps. Engineering is not a prerequisite to **starting** the job search; it is a prerequisite to **reducing repetition** after patterns emerge.

### Recommended first end-to-end exercise

**No suitable saved posting** in `workspace/jobdesk.db`.  
**Owner action:** Ingest one **cybersecurity-relevant** public URL (not Flexport AE), then follow §5.

Example URL classes that fit current connectors: **Greenhouse job-board** security roles; any site with **JSON-LD JobPosting** in HTML.

### Smallest next engineering intervention (only if necessary)

**Not required before the first application.**

After one real cycle, if paste/copy friction dominates, the smallest high-value change is:

> **“Create application from reviewed posting”** — prefill `company`, `title`, `posting_url`, `posting_text`, and meta from latest reviewed `posting_interpretation` without adding `posting_id` or opportunity tables.

Larger but aligned: portfolio-wide analyze (see `NEXT_MILESTONE_DISCOVERY.md`) — defer until manual matrix pain is confirmed.

---

**Mission reminder:** Employment is the goal. JobDesk is operational for a first tailored apply-and-track cycle **today** with documented manual steps for assessment and multi-source opportunities.
