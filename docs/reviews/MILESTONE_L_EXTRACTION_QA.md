# Milestone L — Extraction QA and correction report

**Date:** 2026-11-11 (UTC)
**Trigger:** Manual UI ingest of Google Careers URL (`…/317042454704059940-…`) showed empty structured fields and ~158 KB description containing scripts/app state.

---

## 1. Root cause

### Retrieval vs extraction vs interpretation

| Layer | What happened |
|-------|----------------|
| **Retrieval** | HTTP **200** — page successfully fetched and stored as immutable HTML (correct; not a transport failure). |
| **Target identification** | Final URL pointed at a **removed posting**; HTML contained “Job not found” / “taken down” messaging but HTTP did not return 404. |
| **Extraction** | **No** `JobPosting` JSON-LD for the requested role. A **search-results / shell** page still had incidental JSON-LD and massive inline JS. |
| **Legacy fallback** (`visible_text_fallback` + `strip_html_tags`) | Treated almost the entire `<body>` as text, including `<script>` contents and unrelated listings — **~158 KB** into `description_plain`. |
| **Interpretation** | Machine row stored junk description; UI merged it into the editable textarea. List showed generic **“Posting”** when title was empty. |

**Code path (before fix):** `extract_from_html` → optional JSON-LD → `visible_text_fallback` → `clamp_chars(..., 500_000)` — `crates/domain/src/posting_fetch.rs` (v1 extractor).

### Contributory factors

1. **First `JobPosting` JSON-LD on a listing page** could describe a **different** job than the URL.
2. **No “unavailable” detector** — HTTP success implied extraction success.
3. **Regex tag-stripping** does not remove script bodies reliably when structured like a SPA dump.

---

## 2. Corrective implementation

| Change | Files |
|--------|--------|
| **`scraper` / html5ever** visible text; skip `script`, `style`, `noscript`, `nav`, etc. | `crates/domain/src/posting_fetch.rs` |
| **`detect_source_unavailable`** — skip JSON-LD and fallback when page signals missing job | same |
| **Ignore extra `JobPosting` blocks** on multi-listing pages | same |
| **`looks_like_embedded_app_dump`** — reject fallback / clear description | same |
| **Fallback cap `MAX_FALLBACK_DESCRIPTION_CHARS` = 24_000** | same |
| **Explicit `extraction_outcome:*` diagnostics** | same |
| **Extractor version `POSTING_EXTRACTOR_VERSION` = `"2"`** | same |
| API: `extraction_outcome`, `extraction_diagnostics`, `list_label` | `crates/store/src/postings.rs` |
| UI: outcome line, diagnostic list, list labels, empty description on failure | `86-opportunities.js`, `index.html` |
| Regression fixtures | `crates/domain/tests/fixtures/*.html` |

**Unchanged:** Full raw HTML still stored in `posting_snapshots.raw_body_html`.

---

## 3. Before / after (failure fixture)

**Fixture:** `crates/domain/tests/fixtures/careers_search_unavailable.html` (sanitized pattern from owner observation).

| Field | Before (v1 behavior) | After (v2) |
|-------|----------------------|------------|
| `title` | Could be “Other Role” from wrong JSON-LD | `None` |
| `employer` | Could be “Example Co” | `None` |
| `description_plain` | Large blob incl. `AF_initDataCallback`, unrelated jobs | `None` |
| `extraction_outcome` | (none) | `source_unavailable` |

---

## 4. Tests and results

```bash
cargo test --workspace   # exit 0
```

**New / updated domain tests:**

- `careers_unavailable_fixture_rejects_junk_and_other_jobs`
- `greenhouse_fixture_extracts_title_employer_and_qualifications`
- `script_body_does_not_enter_fallback_description`
- `outcome_partial_when_only_title`

**Store/application isolation:** existing `posting_ingest_does_not_create_application` unchanged (exit 0).

---

## 5. Positive extraction demonstration

**Fixture (checked in, deterministic):** `greenhouse_job_posting.html`

- **Title:** Platform Engineer
- **Employer:** Fixture Labs
- **Description:** contains “Qualifications: 5+ years systems experience…”
- **Outcome:** `recognized`

**Not used as positive case:** unavailable Google Careers URL from prior report.

**Optional live JSON-LD:** prior `live_fetch_job_posting_json_ld` transport test remains `#[ignore]` — it validates **fetch**, not extraction quality.

---

## 6. Remaining limitations

| Limitation | Severity |
|------------|----------|
| SPAs with no JSON-LD and little static text → `partial` / `extraction_failed` | Medium |
| Multi-listing pages with one JSON-LD block for another role (if no “unavailable” copy) | Medium |
| Sites requiring auth (LinkedIn, etc.) | Expected unsupported |
| Firefox `file:///` error | **Uninvestigated** — no `file://` ingest in code; likely unrelated extension/profile |

---

## 7. UI acceptance steps (owner)

1. `cargo run -p jobdesk -- --workspace target/milestone-l-op-accept import` (or your workspace).
2. `cargo run -p jobdesk -- serve --open`
3. **Opportunities** → paste a **valid** public posting URL with JSON-LD where possible. Greenhouse board URLs without JSON-LD are expected to show **Partial extraction** until Milestone M (API adapter); do not treat them as complete structured extraction.
4. **Fetch & save** — confirm status hint (`partial` / `recognized` / unavailable message).
5. Confirm **list label** is not generic “Posting” when title missing (e.g. “Unavailable at source”).
6. Confirm **description** textarea empty with placeholder when extraction failed; **diagnostics** list visible.
7. Open **Raw HTML snapshot** — full source still present.
8. Edit title → **Save** → **Mark reviewed** → restart serve → confirm persistence.
9. Re-fetch same URL → second snapshot; new interpretation unreviewed; `prior_reviewed` if applicable.
10. Confirm **Applications** count unchanged.

**Re-test Google URL:** expect **`source_unavailable`**, empty fields, no script dump in description.

---

## 8. Firefox / `file:///`

JobDesk Opportunities ingest uses **https** URLs only (`validate_public_fetch_url`). The observed `file:///` warning is **not reproduced** from this codebase; treat as **environment/browser extension** until correlated.

**No commit or push** per assignment.
