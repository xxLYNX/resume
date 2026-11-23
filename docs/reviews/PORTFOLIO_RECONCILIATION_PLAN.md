# Portfolio reconciliation and evidence-enrichment preparation

**Date:** 2026-11-12  
**Baseline:** [`PORTFOLIO_ENUMERATION_AND_INTEGRITY.md`](PORTFOLIO_ENUMERATION_AND_INTEGRITY.md)  
**Authoritative DB:** `workspace/jobdesk.db` (schema 17) — **read-only** for this document.  
**Status:** Technical basis **accepted**; **execution preconditions** below incorporated. **Owner authorization still required** — no database mutations until explicit approval.

---

## Assignment A — Duplicate experience reconciliation

### A.1 Persisted reference scan (all six IDs)

| Mechanism | Affinity (both IDs) | Safeway (both IDs) |
|-----------|---------------------|-------------------|
| `element_skill_links` (asserted) | Only on `…-leesburg-va-…` (2 links) | **None** on either |
| `applications.draft_composition` | **0** applications in DB | same |
| `presets.composition_json` | **0** presets | same |
| `submissions` (frozen manifests) | **0** rows | same |
| `element_usage` API | Both report **unused** (no app/preset pins) | same |

**Implicit references (not tracked by `element_usage`):**

- `resolve::default_composition` includes **every** experience element (`crates/resolve/src/lib.rs`) — so both IDs appear on any “full catalog” résumé until one is removed.
- `export_catalog_to_content` writes **all** experience rows to `workspace/staging/content/experience.yaml` (currently **5** entries including both duplicates per role).

No submission or application composition pins any of the four duplicate IDs today; reconciliation risk is **catalog/default résumé pollution**, not broken application drafts.

---

### A.2 Affinity DB — field-by-field comparison (head revisions)

| Field | `experience.affinity-db-leesburg-va-jun-2024-aug-2024` (head rev **2**) | `experience.affinity-db-jun-2024-aug-2024` (head rev **1**) |
|-------|--------------------------------------------------------------------------|-------------------------------------------------------------|
| **Element created** | 2026-10-04T20:13:31Z | 2026-11-12T03:25:43Z |
| **sort_order** | 0 | 0 |
| **position** | Software Security Consultant | Same |
| **company** | `Affinity DB \| Leesburg, VA` (location embedded in string) | `Affinity DB` (clean) |
| **location** | *absent* | `{ city: Leesburg, state: VA }` |
| **date_range.start** | 2024-06 | Same |
| **date_range.end** | 2024-08 | Same |
| **highlights[0–2]** | Identical text (3 bullets) | Identical text |

**Information delta:** The newer element is **strictly richer** for display normalization (`company` + structured `location`). Narrative content is **byte-identical** on highlights and dates.

#### Affinity — revision history

| element_id | rev | created_at | payload_len | Notes |
|------------|-----|------------|-------------|--------|
| `…-leesburg-va-…` | 1 | 2026-10-04 | 581 | Initial import; company pipe-form |
| `…-leesburg-va-…` | 2 | 2026-10-14 | 581 | **Same length** — likely date/payload normalization re-save, no visible field change |
| `…-jun-2024-…` | 1 | 2026-11-12 | 610 | Structured location + clean company |

#### Affinity — skill associations

| element_id | skill_id | label |
|------------|----------|--------|
| `…-leesburg-va-…` | `tool.ghidra` | Ghidra |
| `…-leesburg-va-…` | `competency.onet.2-b-3-e` | Programming |
| `…-jun-2024-…` | — | **No links** |

---

### A.3 Safeway — field-by-field comparison (head revisions)

| Field | `experience.safeway-great-falls-va-sep-2020-jun-2024` (head rev **2**) | `experience.safeway-sep-2020-jun-2024` (head rev **1**) |
|-------|-----------------------------------------------------------------------|--------------------------------------------------------|
| **Element created** | 2026-10-04T20:13:31Z | 2026-11-12T03:25:43Z |
| **sort_order** | 1 | 1 |
| **position** | eCommerce Department Manager and Trainer | Same |
| **company** | `Safeway \| Great Falls, VA` | `Safeway` |
| **location** | *absent* | `{ city: Great Falls, state: VA }` |
| **date_range** | 2020-09 → 2024-06 | Same |
| **highlights[0–4]** | Identical (5 bullets, includes `99.3\%` escaping) | Identical |

**Information delta:** Same pattern as Affinity — newer ID adds structured location; bullets and dates match.

#### Safeway — revision history

| element_id | rev | created_at | payload_len |
|------------|-----|------------|-------------|
| `…-great-falls-va-…` | 1 | 2026-10-04 | 665 |
| `…-great-falls-va-…` | 2 | 2026-10-10 | 665 | Same length — normalization pass |
| `…-sep-2020-…` | 1 | 2026-11-12 | 694 |

#### Safeway — skill associations

None on either element.

---

### A.4 Why two IDs exist — importer identity behavior

Legacy import (`import_legacy_content` → `import_list` for experience) assigns:

```text
element_id = "experience.{slugify(company)}-{element_date_slug(item)}"
```

Source: `crates/store/src/import.rs` (lines 76–91), with `slugify` lowercasing and replacing non-alphanumeric runs with `-` (lines 259–272).

**Worked examples (actual payloads):**

| Import row `company` | `element_date_slug` | Resulting element_id |
|----------------------|---------------------|----------------------|
| `Affinity DB \| Leesburg, VA` | `6-2024-8-2024` (from `date_range`) | `experience.affinity-db-leesburg-va-jun-2024-aug-2024` |
| `Affinity DB` + separate `location` | same dates | `experience.affinity-db-jun-2024-aug-2024` |
| `Safeway \| Great Falls, VA` | `9-2020-6-2024` | `experience.safeway-great-falls-va-sep-2020-jun-2024` |
| `Safeway` + `location` | same dates | `experience.safeway-sep-2020-jun-2024` |

**Behavior when ID already exists:** `import_element` skips unless `ImportOptions.force` (then **appends revision** to the **same** ID). When ID differs, it **INSERT**s a **new** element (`import.rs` lines 164–207).

**Observed timeline:** October 2024 imports created pipe-form company IDs. **2026-11-12** creation timestamps on the shorter IDs align with re-import of repo `content/experience.yaml`, which now uses clean `company` + structured `location` (`content/experience.yaml` lines 2–23) — generating **new** slugs without touching old rows.

**Identity stability:** Catalog element IDs are **owner-facing primary keys** (`create_element` rejects duplicate IDs; compositions pin `element_id` + `revision_id`). Importer IDs are **derived from mutable display fields** (`company` string), so they are **not stable** under formatting improvements. This is an **implementation contract gap**, not evidence that the roles differ.

**Not proposed here:** location-derived IDs as the long-term fix, or automatic fuzzy merge on import.

---

### A.5 Canonical identity recommendation

| Pair | Recommended canonical ID | Rationale |
|------|--------------------------|-----------|
| Affinity | `experience.affinity-db-leesburg-va-jun-2024-aug-2024` | **Only** copy with asserted skill evidence; longer revision history (rev 2); older catalog anchor (2026-10-04). Duplicates’ payload improvements merge **into** this ID via a **new revision** (see below). |
| Safeway | `experience.safeway-great-falls-va-sep-2020-jun-2024` | Older anchor + rev 2 history; no skill-link asymmetry. Merge structured `location` + clean `company` from duplicate head payload. |

**Reject as canonical:** the 2026-11-12 IDs — they add no unique bullets; Affinity duplicate would **drop** Ghidra/Programming links if kept alone.

Matching titles/dates alone are **insufficient** to justify delete; here, **revision diff + skill links + import provenance** support retire-after-merge.

---

### A.6 Execution procedure

The approved mutation steps, backup/archive gates, import guard, and post-repair assertions are defined in **§D — Final execution preconditions and authorized procedure**. Do not run §D until the owner explicitly authorizes reconciliation.

**Scope boundary:** Employment duplicate repair only. Evidence enrichment and project interviews (§B) remain **separate** and **out of scope** for §D execution.

---

### A.7 Lossless reconciliation — missing operations (minimum contract)

Today **no** store/API operation:

| Missing capability | Minimum contract | Class |
|--------------------|------------------|--------|
| **Element ID rename / merge** | `merge_elements(canonical_id, duplicate_id) →` move skill links, optionally squash revision history, delete duplicate atomically | **Future enhancement** (not required if owner accepts delete-after-merge above) |
| **Stable import identity** | Import by explicit `element_id` in YAML **or** map file independent of `company` display string | **Future enhancement** |
| **Implicit composition usage** | `delete_element_if_unused` could warn when element appears in “default catalog composition” | **Future enhancement** |

Reconciliation is **lossless for narrative** with current tools; **revision IDs on duplicate elements are discarded** on delete unless archived first (§D.2).

---

## §D — Final execution preconditions and authorized procedure

**Authorization:** None of §D mutates production data unless the owner sends explicit approval to execute. Until then, treat this section as a runbook only.

### D.0 Audited baseline fingerprint (2026-11-12)

Reconciliation plans are valid only if **pre-mutation live state** matches this fingerprint. If any field differs, **stop** and refresh the audit/plan before proceeding.

| Signal | Expected value |
|--------|----------------|
| `schema_meta` | **17** |
| Experience element count | **5** |
| Experience IDs (sorted) | `experience.affinity-db-jun-2024-aug-2024`, `experience.affinity-db-leesburg-va-jun-2024-aug-2024`, `experience.knowledge-in-motion-2024-11`, `experience.safeway-great-falls-va-sep-2020-jun-2024`, `experience.safeway-sep-2020-jun-2024` |
| Affinity skill links (canonical ID only) | `tool.ghidra`, `competency.onet.2-b-3-e` on `…-leesburg-va-…` |
| Safeway skill links | **none** |
| `applications` / `presets` / `submissions` counts | **0** / **0** / **0** |
| Duplicate IDs to delete (after merge) | `experience.affinity-db-jun-2024-aug-2024`, `experience.safeway-sep-2020-jun-2024` |
| Canonical IDs to keep | `experience.affinity-db-leesburg-va-jun-2024-aug-2024`, `experience.safeway-great-falls-va-sep-2020-jun-2024` |

**Pre-mutation verification script (read-only):**

```bash
DB=workspace/jobdesk.db
sqlite3 -readonly "$DB" "SELECT value FROM schema_meta;"
sqlite3 -readonly "$DB" "SELECT id FROM elements WHERE kind='experience' ORDER BY id;"
sqlite3 -readonly "$DB" "SELECT element_id, skill_id FROM element_skill_links WHERE element_id LIKE 'experience.%';"
sqlite3 -readonly "$DB" "SELECT COUNT(*) FROM applications; SELECT COUNT(*) FROM presets; SELECT COUNT(*) FROM submissions;"
```

Compare output to the table above. Optionally export full duplicate revision rows to JSON and diff against [`PORTFOLIO_RECONCILIATION_PLAN.md`](PORTFOLIO_RECONCILIATION_PLAN.md) §A.2–A.3.

---

### D.1 SQLite-consistent backup (replaces raw `cp`)

Do **not** use plain file copy as the sole backup; SQLite may be mid-transaction. Use the **backup API** via the CLI:

```bash
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
SRC=workspace/jobdesk.db
BAK="workspace/jobdesk.db.bak-reconcile-${STAMP}"
sqlite3 "$SRC" ".backup '${BAK}'"
```

**Verify backup:**

```bash
sqlite3 "$BAK" "PRAGMA integrity_check;"          # must return: ok
sqlite3 -readonly "$BAK" "SELECT value FROM schema_meta;"
sqlite3 -readonly "$BAK" "SELECT COUNT(*) FROM elements WHERE kind='experience';"
sqlite3 -readonly "$BAK" "SELECT element_id, skill_id FROM element_skill_links
  WHERE element_id='experience.affinity-db-leesburg-va-jun-2024-aug-2024';"
```

Expect: `integrity_check` = **ok**, schema **17**, experience count **5**, Affinity canonical with **2** skill links.

**Rollback:** Stop JobDesk; replace live DB only from verified backup: `sqlite3 workspace/jobdesk.db ".restore '${BAK}'"` (or rename backup over live after process exit, then re-run `integrity_check` on live).

---

### D.2 Historical archive of duplicate elements (before delete)

Deleting duplicate IDs removes their `element_revisions` rows from the live catalog (`DELETE` element cascades revisions). Canonical elements **retain** merged facts via a **new** revision; they do **not** retain duplicate-specific revision UUIDs or duplicate-only timestamps.

**Before any delete**, write a recoverable archive (owner workspace, not repo):

```bash
ARCH="workspace/artifacts/portfolio-reconcile-duplicate-archive-${STAMP}.json"
mkdir -p workspace/artifacts
sqlite3 -readonly "$SRC" -json "
SELECT e.id AS element_id, e.kind, e.created_at, e.sort_order,
  r.id AS revision_id, r.rev_number, r.created_at AS revision_created_at, r.payload_json,
  (SELECT json_group_array(json_object('skill_id', skill_id, 'source', source))
   FROM element_skill_links l WHERE l.element_id = e.id) AS skill_links
FROM elements e
JOIN element_revisions r ON r.element_id = e.id
WHERE e.id IN (
  'experience.affinity-db-jun-2024-aug-2024',
  'experience.safeway-sep-2020-jun-2024'
)
ORDER BY e.id, r.rev_number;
" > "$ARCH"
```

Retain `ARCH` and the SQLite backup **together**. Restoration of duplicate revision history requires manual re-insert from archive or full DB restore — there is no merge API.

Record in the execution log: archive path, backup path, operator, UTC timestamp.

---

### D.3 Pre-mutation reference confirmation

Immediately before Step 1 (§D.5), re-run:

1. §D.0 fingerprint queries on **live** `workspace/jobdesk.db`.
2. `GET /api/catalog/elements/{duplicate_id}/usage` for both duplicate IDs → **empty** applications and presets.
3. Snapshot **unchanging** entities for later diff:
   - `SELECT COUNT(*), GROUP_CONCAT(id) FROM applications` (expect 0).
   - Same for `presets`, `submissions`.
4. Export head payloads for both **canonical** and **duplicate** IDs via API or SQL; confirm merge fields still match §A.2–A.3.

If live state ≠ D.0, **abort** (stale plan).

---

### D.4 Import guard (prevent re-duplication after repair)

**Problem:** `jobdesk import` from `content/experience.yaml` still derives IDs from `slugify(company)` + date slug. Current seed YAML (`content/experience.yaml`) would create **`experience.affinity-db-…`** and **`experience.safeway-…`**, **not** the canonical pipe-slug IDs — a blind import would **recreate duplicates** (`crates/store/src/import.rs`).

**Operational guard (required until explicit-ID import is designed separately):**

| Rule | Rationale |
|------|-----------|
| **Do not run** `cargo run -p jobdesk -- import` (with or without `--force`) on owner `workspace/jobdesk.db` after reconciliation without owner-reviewed ID impact | Only safe path today is UI/API catalog edits |
| Treat repo `content/` as **legacy seed**, not a sync source for owner DB | Export authoritative state via `jobdesk export` → `workspace/staging/content/` |
| After repair, if `content/experience.yaml` is edited, **do not import** it until an explicit-ID importer exists | No fuzzy merge; no location-derived IDs |
| Document in execution log: “import prohibited on owner DB pending stable IDs” | Process guard only — no new catalog architecture in this assignment |

Optional **read-only** dry-check before any future import (manual): compute `experience.{slugify(company)}-{date_slug}` for each YAML row and confirm IDs match existing elements (see §A.4). Mismatch → stop.

**Durable explicit-ID solution:** tracked separately; **not** part of this repair.

---

### D.5 Authorized mutation steps (owner approval required)

**Step 1 — Merge head on canonical only**

For Affinity then Safeway, on **canonical** ID:

- `POST /api/catalog/elements/{canonical_id}/revisions` with merged payload:
  - `position`, `highlights`, `date_range`: unchanged from current heads (already identical across pair).
  - `company`: `Affinity DB` / `Safeway`.
  - `location`: `{ city, state }` from duplicate head (`Leesburg`/`VA`, `Great Falls`/`VA`).

Affinity canonical should advance to **rev 3**; Safeway canonical to **rev 3**. **Do not** delete or overwrite revisions 1–2 on canonical elements.

**Step 2 — Delete duplicate elements**

For `D` ∈ `{ experience.affinity-db-jun-2024-aug-2024, experience.safeway-sep-2020-jun-2024 }`:

1. Confirm §D.2 archive exists.
2. `GET …/usage` → unused.
3. `DELETE /api/catalog/elements/{D}`.

**Step 3 — Export staging only**

- `jobdesk export` → refresh `workspace/staging/content/` (expect **3** experience entries).
- Do **not** mirror to repo `content/` unless owner explicitly wants seed update **without** import.

---

### D.6 Post-repair assertions

Run on **live** DB after Steps 1–3. **Fail** any row → stop and consider restore from §D.1 backup.

| # | Assertion | How to verify | Expected |
|---|-----------|---------------|----------|
| 1 | Experience count | `SELECT COUNT(*) FROM elements WHERE kind='experience'` | **3** |
| 2 | Remaining experience IDs | sorted list | KIM + **canonical** Affinity + **canonical** Safeway only; **no** `…-jun-2024-…` or `…-sep-2020-…` |
| 3 | Canonical accomplishments & dates | JSON head payloads | Same bullets and `date_range` as pre-merge; `company` + `location` per §D.5 |
| 4 | Affinity skill links | `element_skill_links` for canonical Affinity | **2** rows: `tool.ghidra`, `competency.onet.2-b-3-e` |
| 5 | Prior canonical revisions preserved | `SELECT rev_number FROM element_revisions WHERE element_id=…` | Affinity: rev **1, 2, 3**; Safeway: **1, 2, 3** |
| 6 | No duplicate employment in default composition | `default_composition` experience item count (CLI/build or API) | **3** items; each `element_id` unique |
| 7 | Export experience rows | `grep -c '^- company:' workspace/staging/content/experience.yaml` (or YAML list length) | **3** |
| 8 | Applications unchanged | count + optional hash of table | **0** (same as pre-mutation) |
| 9 | Presets unchanged | count | **0** |
| 10 | Submissions unchanged | count | **0** |

**Execution report template (fill after authorized run):**

- Pre-mutation fingerprint: pass/fail  
- Backup path + `integrity_check`:  
- Archive path:  
- New revision IDs (Affinity rev 3, Safeway rev 3):  
- Deleted element IDs:  
- Post-repair assertion table: all pass/fail  
- Import guard acknowledged: yes  

---

### D.7 Rollback summary

| Situation | Action |
|-----------|--------|
| Bad merge revision | Append rev 4 restoring payload from canonical rev 2 (Affinity/Safeway) or restore backup |
| Wrong delete / lost links | Restore from §D.1 backup |
| Need duplicate revision UUIDs | Use §D.2 archive + full restore; no automatic un-delete |

---

## Assignment B — Owner-guided evidence enumeration

Structured worksheets for three projects. Sources: head revision payloads only (`element_revisions` rev 1). No `portfolio_narrative` present. Skill links: lab has `competency.onet.2-b-3-e` (Programming) only.

---

### B.1 `project.cybersecurity-lab`

**Dates (catalog):** Feb 2023 – Present

#### Explicitly recorded accomplishments (résumé bullets)

1. Declarative IaC NixOS monorepo (flake) as SSOT.
2. SOC-style stack: OPNsense/Suricata NIPS; Wazuh SIEM on all hosts; Grafana Alloy → Prometheus/Loki.
3. Self-hosted Git forge; CI with Trivy SBOM and `nix audit`; secrets via sops-nix/Age.
4. Zero-trust: Tailscale + Keycloak OIDC/MFA; used for HackTheBox and malware RE.

#### Technologies / methods mentioned (not skill-linked)

OPNsense, Suricata, Wazuh, Grafana Alloy, Prometheus, Loki, Nix/NixOS, flake, Git forge, CI, Trivy, nix audit, sops-nix, Age, Tailscale, Keycloak, OIDC, MFA, HackTheBox, malware reverse engineering.

#### Supported only by description (needs owner verification)

| Claim | Verification need |
|-------|-------------------|
| “SOC-style” / production-like detection | Scale, alert volume, tuned rules?, false-positive handling |
| Wazuh on **all** hosts | Host count, agent versions, centralized rules/decoders |
| Suricata NIPS at perimeter | Inline vs passive, rule sources, blocking vs alerting |
| Supply-chain CI (Trivy SBOM, nix audit) | Pipeline location, frequency, blocking policy |
| Zero-trust access architecture | Who uses it, MFA enforcement scope |
| HackTheBox + malware RE **on this infra** | Typical workflows, isolation, samples handling |

#### Missing for security-engineering opportunities (e.g. detection engineer)

- Custom detection content authored (rules, sigma, YARA, Wazuh rulesets).
- Incident/triage narratives or metrics.
- Log sources beyond homelab (cloud SaaS, EDR, third-party SIEM).
- Team/on-call or customer-facing SOC experience.

#### Owner questions (priority order)

1. What did **you** personally build vs deploy from guides/templates for each layer (OPNsense, Wazuh, Alloy)?
2. How many systems/agents feed Wazuh today? Any example **detection** you wrote that fired on real activity?
3. Is Suricata blocking traffic or alerting only? Who maintains rules?
4. Where does the Git forge run, and can you point to a **concrete** CI job that blocked a bad change?
5. How do you isolate malware RE from production homelab networks?
6. What artifacts can you share later (redacted configs, rule files, dashboard screenshots, repo links) for evidence?
7. Should any tools above become **asserted skill links** only after you confirm hands-on depth?

---

### B.2 `project.machine-learning-malware-detector`

**Dates:** Aug 2024 – Present

#### Explicitly recorded accomplishments

1. CI/CD pipeline for ingesting malware samples for supervised ML detection.
2. Ingested **201,549** Windows malware samples; label encoder + padding.
3. **94%** accuracy with k-fold CV vs train-test split baseline.

#### Technologies / methods mentioned

CI/CD, supervised ML, malware samples, label encoder, padding, k-fold cross-validation, train-test split (Windows-focused).

#### Supported only by description

| Claim | Verification need |
|-------|-------------------|
| Sample count 201,549 | Dataset source, licensing, duplicate handling |
| 94% accuracy | Model family, features, class balance, metric definition, holdout leakage |
| CI/CD for ingestion | Runtime environment, automation tool, who operates it |
| Production/use vs class project | Deployment, inference on live samples |

#### Missing for detection-engineering / ML security roles

- Model type, feature engineering, adversarial/evasion testing.
- Integration with detection pipeline (rules + ML).
- Comparison to public baselines; reproducibility.

#### Owner questions

1. What is the **dataset provenance** (repository, timeframe, label quality)?
2. Which **model and features** produced 94%? Can you reproduce the eval notebook/pipeline?
3. Did **you** build the CI/CD alone? What does a pipeline run do end-to-end?
4. Is this still active, academic, or portfolio-only? Any ethical/legal review for sample storage?
5. What artifacts exist (repo, config, eval report, sample metrics table)?
6. For job materials: should the bullet quote 94% as-is, or soften until validated?

---

### B.3 `project.ghostrace-cve-research`

**Dates:** Jan 2024 – May 2024

#### Explicitly recorded accomplishments

1. PoC exploit for **CVE-2024-2193 (GhostRace)**.
2. Analysis tied to Spectre/Meltdown branch-prediction / speculative execution themes.

#### Technologies / methods mentioned

CVE-2024-2193, GhostRace, PoC exploit, Spectre, Meltdown, branch misprediction, speculative execution.

#### Supported only by description

| Claim | Verification need |
|-------|-------------------|
| Working PoC | Target CPU/OS, build steps, demonstrated impact |
| CVE numbering alignment | GhostRace vs published CVE-2024-2193 scope |
| Independent research depth | Literature review vs reproduction of public work |

#### Missing for vuln research / detection roles

- Disclosure timeline, vendor coordination, publication.
- Detection implications (if any) for affected platforms.
- Link to write-up, commit, or advisory.

#### Owner questions

1. Did you **independently discover** or **reproduce** published GhostRace research? What was novel in your PoC?
2. On what **hardware/software** did the PoC run? Is it releasable (even privately to reviewers)?
3. Is there a report, blog draft, or course submission documenting methodology?
4. How does this connect to **defensive** work (detections, mitigations) you could discuss in interviews?
5. Should résumé wording name GhostRace, generic “speculative execution research,” or await your confirmation of CVE attribution?

---

### B.4 Cross-project owner session (suggested agenda)

1. Confirm duplicate experience merge plan (§A).  
2. Walk through lab → ML → GhostRace worksheets (30–45 min).  
3. Agree which claims stay on résumé bullets vs move to future `portfolio_narrative`.  
4. Defer **all** new skill links until owner confirms hands-on evidence.

---

## Assignment C — Implementation decisions

### C.1 Decision table

| Finding | Category | Immediate action | Code required? |
|---------|----------|------------------|----------------|
| Duplicate experience elements | **Data repair** | Merge payload → canonical rev; delete duplicate ID (§A.6) | **No** (existing revision + delete APIs) |
| Sparse skill links | **Data enrichment** | Owner adds links after verification | **No** |
| Stale `content/experience.yaml` causes re-duplication on import | **Data repair** + process | Stop blind re-import; fix YAML rows; use canonical IDs | **No** (process); optional stable-import **enhancement** |
| Importer IDs depend on `company` string | **Implementation defect** (identity contract) | Document; avoid re-import without `force` + same ID | **Yes** for durable fix (explicit ID in YAML or stable id map) — **not** for immediate reconcile |
| `default_composition` lists all elements | **Architectural limitation** | Fixed when duplicates removed | Optional warn-on-delete **enhancement** |
| No `merge_elements` API | **Future enhancement** | Manual merge + delete acceptable now | Optional later |
| `portfolio_narrative` unused | **Data enrichment** | After owner Q&A | **No** |
| `analyze_application` needs application row | **Future enhancement** | Out of scope for duplicate fix | Optional later |
| Agent export file stale | **Documentation** | Regenerate after reconcile | **No** |

### C.2 Implementation defects (demonstrable, not enhancements)

| Defect | Source | Consequence |
|--------|--------|-------------|
| Import constructs **new** element IDs when **display** `company` changes, without dedup | `crates/store/src/import.rs` `id_for` closure | Duplicate catalog rows; duplicate résumé lines |
| Delete guard ignores **default composition** | `element_usage` only scans apps/presets (`catalog.rs` 203–257) | Owner may delete an ID still shown on “full” résumé until duplicates gone — low risk here because duplicates are unwanted |

### C.3 Explicitly deferred (per assignment)

- Automatic skill inference from bullets.
- Evidence graph / global link index.
- New element lifecycle states (`merged`, `retired`).
- Fuzzy or location-based auto-merge on import.

### C.4 Minimum future contract (if owner wants import safety)

**Stable catalog import row:**

```yaml
# illustrative — not implemented
id: experience.affinity-db-leesburg-va-jun-2024-aug-2024  # required stable key
company: Affinity DB
location: { city: Leesburg, state: VA }
# ...
```

Importer: if `id` present, upsert element with that ID; else keep legacy slug rule for backward compatibility.

---

## Owner review checklist

- [ ] Approve canonical IDs (Affinity leesburg-va slug; Safeway great-falls-va slug).  
- [ ] Approve merged head fields (`company` + `location` shape).  
- [ ] Approve delete of `experience.affinity-db-jun-2024-aug-2024` and `experience.safeway-sep-2020-jun-2024` after merge revision.  
- [ ] Approve §D execution runbook (SQLite `.backup`, archive, import guard, assertions).  
- [ ] **Explicit authorization to mutate** `workspace/jobdesk.db` (separate message).  
- [ ] Complete project worksheets (§B) **later** — not part of employment repair.  
- [ ] Decide whether to schedule explicit **import ID** product work (separate design).

**Stop here.** No production DB writes until owner signs checklist **and** sends execution authorization.
