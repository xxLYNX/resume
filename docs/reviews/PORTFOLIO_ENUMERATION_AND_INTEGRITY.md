# Portfolio enumeration and integrity audit

**Date:** 2026-11-12  
**Scope:** Read-only review of `workspace/jobdesk.db` (schema **17**), staged YAML export, repo `content/` seed, and `workspace/portfolio-export-for-agent.txt`.  
**Out of scope:** Code changes, DB writes, commits, opportunity fit scoring automation.

---

## Executive summary

The authoritative portfolio is **small and coherent** (15 catalog elements, 19 revisions) with strong cyber-research and lab content, but **two overlapping duplicate experience pairs** now inflate default résumé composition and YAML export. **Asserted skill evidence is extremely sparse** (3 links) relative to rich narratives, which limits truthful gap analysis and toolbelt rendering. Several **reference artifacts are stale** relative to the live DB. Implementation **append-only revision integrity is sound**; the main product gaps are **duplicate-element lifecycle**, **evidence modeling**, and **portfolio-wide assessment** (no applications in DB today).

---

## 1. Authoritative sources

| Source | Role | Staleness |
|--------|------|-----------|
| `workspace/jobdesk.db` | **Authoritative** persisted catalog, skills, postings | Current (schema 17) |
| `workspace/staging/content/*.yaml` | Last `jobdesk export` mirror under workspace | Updated 2026-11-12; includes **5** experience rows (duplicates) |
| `content/*.yaml` (repo root) | Legacy **import seed** only | **Stale:** 2 experience rows, no Knowledge In Motion, old `date` fields |
| `workspace/portfolio-export-for-agent.txt` | Human/agent snapshot | **Stale:** schema 15, 13 elements, describes deleted application |

**Not portfolio:** Owner DB also holds **7** opportunity postings (6 CrowdStrike + 1 Flexport from ingestion exercises). They do not alter the catalog but pollute owner workspace if mistaken for applications.

**Applications:** `applications` count **0**; `submissions` **0**. The export file’s Jobright application is **no longer in the DB**.

---

## 2. Complete portfolio inventory

### 2.1 Summary counts

| Kind | Elements | Head revisions | Notes |
|------|----------|----------------|-------|
| header | 1 | 1 | |
| objective | 1 | 1 | |
| education | 2 | 1 each | |
| experience | **5** | 1–3 | **2 duplicate pairs** (see §3) |
| project | 6 | 1 each | |
| **Total** | **15** | **19** revision rows | |

**Skill reference taxonomy:** 8,808 `skill_nodes` (8,773 `tool`, 35 `competency` O*NET). This is **imported reference data**, not owner narrative.

**Asserted evidence links:** **3** rows in `element_skill_links` (`source = asserted` only).

---

### 2.2 Element catalog (head revision)

#### Header — `header.main` (rev 1)

| Field | Value |
|-------|--------|
| name | Cullen Kelley |
| email | cullenscout1970@gmail.com |
| phone | 703-732-0221 |
| github | https://github.com/xxLYNX |
| linkedin | https://linkedin.com/in/cullen-kelley/ |

#### Objective — `objective.main` (rev 1)

Seeks **analyst** role emphasizing security operations, malware research, vulnerability assessment (engineering postings may trigger objective mismatch warnings in `application_analyze` — see `crates/domain/src/application_analyze.rs`).

#### Education

| Element ID | Degree / school | Dates |
|------------|-----------------|-------|
| `education.masters-of-applied-it-cyber-security-concentration` | Masters of Applied IT, Cyber Security Concentration — George Mason University \| GPA 3.82 | Aug 2024 – Dec 2025 |
| `education.bachelor-of-applied-science-cyber-security-concentration` | Bachelor of Applied Science, Cyber Security Concentration — George Mason University \| GPA 3.83 | Aug 2021 – Aug 2024; Phi Kappa Phi, Dean's List |

#### Experience (head payloads)

| Element ID | Position | Company | Location | Dates | Rev |
|------------|----------|---------|----------|-------|-----|
| `experience.knowledge-in-motion-2024-11` | Software R&D Engineer | Knowledge In Motion | Reston, VA | Jan 2026 – Present | 3 |
| `experience.affinity-db-leesburg-va-jun-2024-aug-2024` | Software Security Consultant | Affinity DB \| Leesburg, VA | (in company string) | Jun 2024 – Aug 2024 | 2 |
| `experience.affinity-db-jun-2024-aug-2024` | Software Security Consultant | Affinity DB | Leesburg, VA (structured) | Jun 2024 – Aug 2024 | 1 |
| `experience.safeway-great-falls-va-sep-2020-jun-2024` | eCommerce Department Manager and Trainer | Safeway \| Great Falls, VA | (in company string) | Sep 2020 – Jun 2024 | 2 |
| `experience.safeway-sep-2020-jun-2024` | eCommerce Department Manager and Trainer | Safeway | Great Falls, VA (structured) | Sep 2020 – Jun 2024 | 1 |

**Highlights (substantive):**

- **Knowledge In Motion:** FRACTAL scheduling/planning R&D (constraint scheduling, automated planning).
- **Affinity DB:** Python/VBA third-party code security audits; Ghidra, x64dbg, oletools, Didier Stevens tooling; documentation for stakeholders.
- **Safeway:** eCommerce operations/training; **responsible disclosure** to Safeway IT; regional eCommerce launches.

#### Projects (head payloads)

| Element ID | Name | Span | `portfolio_narrative` |
|------------|------|------|------------------------|
| `project.cybersecurity-lab` | Cybersecurity Lab | Feb 2023 – Present | **absent** |
| `project.machine-learning-malware-detector` | Machine Learning Malware Detector | Aug 2024 – Present | absent |
| `project.self-driving-car-ba-algorithm-research` | Self-Driving Car BA Algorithm Research | Sep 2025 – Dec 2025 | absent |
| `project.ghostrace-cve-research` | GhostRace CVE Research | Jan 2024 – May 2024 | absent |
| `project.cloud-service-model-risk-assessment` | Cloud Service Model Risk Assessment | Aug 2024 – Dec 2024 | absent |
| `project.biometric-authentication-modalities-research` | Biometric Authentication Modalities Research | Jan 2025 – May 2025 | absent |

**Project bullets (themes):** homelab SOC (OPNsense/Suricata, Wazuh SIEM, Grafana/Prometheus/Loki), supply-chain CI (Trivy, nix), zero-trust (Tailscale, Keycloak), malware RE / HackTheBox; ML malware classifier (201k samples, 94% accuracy claim); GhostRace PoC; cloud risk (NIST SP 800-210); biometrics research; SLAM/bundle adjustment benchmarks.

None of the project payloads include structured `technologies` or `resume_tags` fields; evidence is prose-only except where skill links exist.

---

### 2.3 Revision history (non-head)

| Element ID | Revisions | Notes |
|------------|-----------|-------|
| `experience.affinity-db-leesburg-va-jun-2024-aug-2024` | 1 → 2 | rev 2 **2025-10-14**; payload length unchanged (581 bytes) — likely normalization/metadata |
| `experience.knowledge-in-motion-2024-11` | 1 → 3 | Text edits across Oct 2025 revisions |
| `experience.safeway-great-falls-va-sep-2020-jun-2024` | 1 → 2 | **2025-10-10** |
| All others | 1 only | |

Append-only semantics: revisions are never updated in place (`crates/store/src/catalog.rs` `create_revision` inserts new rows).

---

### 2.4 Asserted skill evidence (full inventory)

| Element ID | skill_id | Resolved label | kind |
|------------|----------|----------------|------|
| `experience.affinity-db-leesburg-va-jun-2024-aug-2024` | `tool.ghidra` | Ghidra | tool |
| `experience.affinity-db-leesburg-va-jun-2024-aug-2024` | `competency.onet.2-b-3-e` | Programming | competency |
| `project.cybersecurity-lab` | `competency.onet.2-b-3-e` | Programming | competency |

**Duplicate Affinity element** (`experience.affinity-db-jun-2024-aug-2024`) has **no** skill links. Toolbelt aggregation for compositions that pin only the new ID would show **no** Ghidra evidence unless links are copied.

**Narrative vs evidence (selected):** Wazuh, Suricata, Python, Prometheus, Grafana, Nix, Trivy, Keycloak, Tailscale, OPNsense, Loki appear in catalog text but have **no** asserted links. Splunk does not appear in narrative.

---

## 3. Data integrity and completeness findings

### 3.1 Duplicate / overlapping elements

| Pair | Overlap | Severity |
|------|---------|----------|
| `experience.affinity-db-leesburg-va-jun-2024-aug-2024` ↔ `experience.affinity-db-jun-2024-aug-2024` | Same role, dates, highlights | **P0** |
| `experience.safeway-great-falls-va-sep-2020-jun-2024` ↔ `experience.safeway-sep-2020-jun-2024` | Same role, dates, highlights | **P0** |

**Likely cause:** Legacy import derives element IDs from `slugify(company)` + date slug (`crates/store/src/import.rs`). When `company` changed from `"Affinity DB"` to `"Affinity DB | Leesburg, VA"` (or Safeway variants), a **second element** was created instead of revising the existing ID. Newer duplicates were created **2025-11-12** with structured `location` objects.

**Consequence:** `default_composition()` and export include **all** experience elements (`crates/resolve/src/lib.rs`); a full résumé or default export lists **two Affinity and two Safeway jobs** — misleading without owner curation.

| Finding | Classification | Data fix vs code |
|---------|----------------|------------------|
| Duplicate experience pairs in DB | **P0** | **Data:** retire/merge IDs + pick canonical element; **Code:** merge UI, dedupe guards, stable import IDs |
| Staging `experience.yaml` lists 5 jobs | **P0** (symptom) | Re-export after data fix |
| Repo `content/experience.yaml` missing KIM, has 2 jobs only | **Documentation** | Update seed or stop treating `content/` as snapshot |

### 3.2 Conflicting facts and dates

| Issue | Detail | Classification |
|-------|--------|----------------|
| Objective vs target roles | Objective emphasizes **analyst**; CrowdStrike references are **engineer**-heavy | **P1** / **Documentation** (wording choice) |
| Masters end Dec 2025 | Consistent in DB; verify against transcript if applying before completion narratives | **Documentation** |
| Knowledge In Motion start Jan 2026 | Consistent with “Present” as of audit date Nov 2026 | OK |

No contradictory degrees or impossible date ranges detected in head revisions.

### 3.3 Missing or thin documentation

| Gap | Classification |
|-----|----------------|
| No `portfolio_narrative` on any project (field supported but unused) | **Documentation** — limits long-form evidence without résumé bloat |
| No certifications element kind in catalog | **Documentation** — R30634 mentions GCFA/GCDA/GCIH |
| No dedicated **course** children under education | Neutral |
| Sparse evidence tags vs narrative tool mentions | **P1** — affects assessment truthfulness |
| `portfolio-export-for-agent.txt` outdated | **Documentation** |

### 3.4 Skill / capability representation gaps

Capabilities appear in **text** but are **not** modeled as asserted skills:

- SIEM operations (Wazuh named; not Splunk/LogScale)
- Detection / NIDS (Suricata)
- Malware analysis / ML detection
- Vulnerability research (GhostRace, Safeway disclosure)
- Cloud security architecture (lab, cloud risk project)
- Application security review (Affinity)
- Python scripting (multiple bullets; only generic “Programming” competency linked twice)

**Do not infer** the owner lacks Splunk, threat-hunting employment, or certifications — they are **undocumented or unlinked**, not disproven.

### 3.5 Misleading résumé claim risks

| Risk | Mechanism |
|------|-----------|
| Double-counted employment | Default composition includes duplicate IDs |
| Over-broad “Programming” competency | Single O*NET competency linked to lab + Affinity; does not distinguish Python/SIEM/detection |
| ML “94% accuracy” | Stated in project bullets without external validation metadata |
| Safeway vuln disclosure | Strong claim; only in experience text, no evidence artifact link |

---

## 4. Implementation integrity

### 4.1 Strengths (confirmed behavior)

| Area | Evidence |
|------|----------|
| Append-only revisions | `create_revision` inserts only; history preserved (`catalog.rs`) |
| Revision pinning in compositions | `load_item_payloads` verifies `revision_id` matches `element_id` (`resolve/src/lib.rs`) |
| Portfolio narrative excluded from PDF | Stripped in `normalize_row` (`resolve/src/normalize.rs`); tested in `resolve/src/tests.rs` |
| Resume tags require asserted links | `validate_composition_resume_tags` (`resolve/src/resume_tags.rs`) |
| MCP full element payload | `get_catalog_element` returns all revisions metadata + latest payload (`mcp_tools.rs`) |
| Export round-trip narrative | `store/src/export.rs` tests |

### 4.2 Confirmed defects / limitations (with consequences)

| Issue | Source | Consequence | Class |
|-------|--------|-------------|-------|
| Default composition includes **every** catalog element of each kind | `resolve/src/lib.rs` `default_composition` | Duplicates appear on “full resume” and any unpinned draft derived from default | **P1** (product); enabled by **P0** data |
| YAML export lists all elements per kind | `store/src/export.rs` `write_list` | `staging/content/experience.yaml` encodes duplicate jobs | **P1** |
| Import ID = f(company string) | `store/src/import.rs` `import_list` id_for | Re-import after company formatting change creates **new** element IDs; old rows remain | **P1** architectural |
| No DB constraint on semantic duplicate experiences | Schema | Owner duplicates persist undetected | **P1** architectural |
| Only `asserted` source in `element_skill_links` today | `skills.rs` | Inferred skills from text are invisible to toolbelt unless manually linked | **P1** limitation |
| `analyze_application` requires application row | `server/applications.rs`, empty DB | No end-to-end gap report for CrowdStrike postings without creating an application | **P2** / product gap |
| `get_catalog_summary` truncates snippets | `mcp_tools.rs` | OK at 15 elements; would truncate large catalogs at 200 | **P2** |
| O*NET tool catalog size (8773) | `skill_nodes` | Search noise; not a defect but affects hint quality | **P2** |
| `portfolio_narrative` stripped on resolve but **included** in MCP/DB API | By design | Agents see more than PDF; owners must not confuse channels | Documentation |

**Not defects:** Revision immutability; validation of payload shapes (`domain/validation.rs`); Greenhouse/Workday posting tables separate from catalog.

### 4.3 MCP and HTTP API coverage

| Need | Available? |
|------|------------|
| List elements | HTTP `/api/catalog/elements`; MCP `get_catalog_summary` |
| Full element + revisions | MCP `get_catalog_element`, `get_catalog_revision` |
| Per-element asserted skills | MCP `get_skill_evidence` |
| Global evidence graph | **No** single tool — must iterate elements |
| Portfolio-wide analyze vs posting | **No** without `application` + `posting_text` |
| Element usage in apps/presets | MCP `get_element_usage` (empty while no apps) |

---

## 5. CrowdStrike reference cases (coverage exercise)

Saved owner postings (recognized Workday ingest). Primary focus **R30634** — Detection Engineer, Falcon Complete (Remote).

### 5.1 R30634 — Detection Engineering / NG-SIEM (primary)

**Posting asks (abbreviated):** Threat research/hunting, high-fidelity SIEM detections, detections-as-code, Python, multi-vendor data, LLM-assisted workflows, 3+ years detection/SOC-adjacent experience, certifications (GCFA, GCDA, GCIH, etc.).

| Capability area | Supporting portfolio elements | Evidence strength |
|-----------------|------------------------------|-------------------|
| SIEM / detection stack | `project.cybersecurity-lab` (Wazuh SIEM, Suricata NIPS, logging) | **Moderate** — homelab, not production MDR scale |
| Threat / vuln research | `project.ghostrace-cve-research`, Safeway disclosure bullet | **Moderate** — academic/PoC + retail disclosure |
| Malware / datasets | `project.machine-learning-malware-detector` | **Moderate** — ML pipeline, not rule-writing |
| Python / automation | Affinity audits, ML project, lab IaC (Nix) | **Moderate** — narrative; weak skill links |
| Intelligence-driven detections | Not explicitly documented | **Insufficient / uncertain** |
| Certifications (GCFA, etc.) | Not in catalog | **Documentation** gap if held |
| Production detection engineering tenure | Education + ~4 months Affinity + projects; KIM is non-security R&D | **Thin** vs “3+ years” framing |
| LLM / coding agents | Not documented | **Insufficient** |

**Elements unlikely to help R30634:** `project.self-driving-car-ba-algorithm-research`, `project.biometric-authentication-modalities-research` (unless framed as research discipline).

### 5.2 Adjacent postings (summary)

| Req | Title theme | Best portfolio anchors | Gaps |
|-----|-------------|------------------------|------|
| R30561 | Data center technician | Lab infra tangentially | No datacenter employment narrative |
| R29756 | Windows tech support (GBR) | General IT from Safeway | Little Windows-specific evidence |
| R30635 | Detection engineer (CAN) | Same as R30634 | Same |
| R30636-1 | Sr Linux sensor engineer | `cybersecurity-lab` (Nix/Linux), GhostRace low-level | Limited production C/systems narrative |
| R29568-1 | Product / application security | Affinity code audit, cloud risk project | No formal AppSec title tenure |

---

## 6. Requirements for eventual portfolio-wide assessment

To support **opportunity-to-portfolio** analysis without manual SQL:

1. **Canonical element set** — deduplicated experience IDs with explicit “active / merged / superseded” metadata.
2. **Rich evidence layer** — asserted links for tools/competencies mentioned in narrative, or structured “evidence note” objects with confidence (demonstrated vs stated).
3. **Posting-to-catalog analyze path** that does not require a full `applications` row (or a documented bridge from `postings.machine_json` → analyze).
4. **Composition-aware reporting** — which elements are on default vs tailored drafts; flag duplicates in any composition.
5. **Export freshness indicator** — schema version + element count in agent exports.
6. **Certifications / training** element support in catalog if industry credentials are part of fit stories.
7. **Global skill/evidence index** API/MCP (`list_asserted_links`, `skills_without_evidence`).

---

## 7. Prioritized recommendations

| Priority | Finding | Recommended action | Data vs code |
|----------|---------|-------------------|--------------|
| **P0** | Duplicate Affinity + Safeway experience elements | Owner review: pick canonical ID per role; archive other via policy (delete element or mark inactive when supported) | Mostly **data**; needs **code** for merge/safe archive |
| **P0** | Default résumé/export double-lists jobs | After dedupe, re-run export; verify `default_composition` count | Data first; consider **code** dedupe guard |
| **P1** | 3 skill links vs rich narratives | Owner workshop: link tools (Wazuh, Suricata, Python, Ghidra, etc.) to lab/Affinity/ML project | **Data**; UI already supports links |
| **P1** | Import slug instability | Stabilize IDs on re-import (location-aware slug) or match-by date+company fuzzy | **Code** |
| **P1** | Objective misaligned with engineer postings | Owner rewrite objective or maintain role-specific objective elements | **Data** |
| **P1** | Stale `portfolio-export-for-agent.txt` | Regenerate from DB after corrections or deprecate file | **Documentation** |
| **P2** | No `portfolio_narrative` on projects | Add long-form writeups for lab/ML/GhostRace where résumé bullets are thin | **Data** |
| **P2** | Cannot run `analyze` without application | Product: “analyze posting id” using opportunities store | **Code** |
| **Documentation** | Certifications not cataloged | Add if applicable | **Data** |
| **Documentation** | CrowdStrike postings in owner DB | Optional cleanup of test postings (separate from catalog) | **Data** (opportunities) |

---

## 8. Classification index

| ID | Title | Class |
|----|-------|-------|
| DUP-EXP-1 | Affinity DB duplicate elements | **P0** |
| DUP-EXP-2 | Safeway duplicate elements | **P0** |
| EVID-SPARSE | Asserted skills (3) vs narrative | **P1** |
| IMP-DEFAULT-COMP | All elements in default composition | **P1** |
| IMP-IMPORT-ID | Company-string slug duplicate creation | **P1** |
| OBJ-ANALYST | Objective vs engineer roles | **P1** / **Documentation** |
| EXP-STALE | Agent export + repo `content/` stale | **Documentation** |
| COV-R30634 | Detection-engineer tenure & SIEM depth uncertain | **Documentation** (coverage, not capability denial) |
| MCP-NO-GLOBAL | No global evidence graph tool | **P2** |
| ANALYZE-NO-APP | Analyze requires application | **P2** |

---

## 9. Appendix — CrowdStrike posting inventory (owner DB)

| Requisition | Stored title (machine) |
|-------------|-------------------------|
| R30561 | Data Center Technician (Sacramento, CA) |
| R29756 | Technical Support Engineer - Windows OS (Remote, GBR) |
| R30634 | Detection Engineer, Falcon Complete (Remote) |
| R30635 | Detection Engineer, Falcon Complete (Remote, CAN) |
| R30636-1 | Sr. Software Engineer II - Sensor, Linux Guardian (Hybrid) |
| R29568-1 | Product Security Engineer, Application Security (Remote) |

Plus non-CrowdStrike: Flexport Account Executive (Greenhouse fixture/live artifact).

---

*End of audit. No corrective changes were applied.*
