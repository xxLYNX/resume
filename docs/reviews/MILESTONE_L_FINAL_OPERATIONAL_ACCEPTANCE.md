# Milestone L — Final operational acceptance (pre-commit)

**Date:** 2026-11-12
**Baseline commit (parent):** `97fb46d2428f887327b5d2ad070f32536eddeaad`
**Workspace DB used:** `target/milestone-l-final-accept2/jobdesk.db`
**No commit / no push** per owner request.

## Correction applied during this pass

`detect_source_unavailable` previously scanned raw HTML (including footer copy containing “page not found”), which falsely marked valid postings unavailable. It now probes **title + primary content** (`main` / `article` / `body`) and treats **“page not found”** only when it appears in **title or primary `h1`**. Strong phrases (e.g. “job not found”, “may have been taken down”) still apply to that probe text. Regression: `valid_page_with_footer_error_text_not_unavailable`, `careers_unavailable_fixture_rejects_junk_and_other_jobs`.

---

## Commands (exact)

### Build and workspace

```bash
cd /home/voyager/Projects/resume
cargo build -p jobdesk -q
OP_WS="/home/voyager/Projects/resume/target/milestone-l-final-accept2"
rm -rf "$OP_WS" && mkdir -p "$OP_WS/preview"
cargo run -p jobdesk -q -- --workspace "$OP_WS" import
```

### Serve (background for API calls)

```bash
PORT=3861
cargo run -p jobdesk -q -- --workspace "$OP_WS" serve --port $PORT --host 127.0.0.1 &
SRV=$!
sleep 4
```

### Case A — Google Careers (previously unavailable posting)

```bash
GOOGLE='https://www.google.com/about/careers/applications/jobs/results/317042454704059940-software-engineer-iii-geo-maps/'
curl -sS -o /tmp/case_a2.json -w "HTTP:%{http_code}\n" \
  -X POST "http://127.0.0.1:$PORT/api/opportunities/ingest" \
  -H 'Content-Type: application/json' \
  -d "{\"url\":\"$GOOGLE\"}"
sqlite3 "$OP_WS/jobdesk.db" \
  "SELECT diagnostics_json, length(raw_body_html) FROM posting_snapshots WHERE request_url LIKE '%google%';"
sqlite3 "$OP_WS/jobdesk.db" "SELECT COUNT(*) FROM applications;"
kill $SRV 2>/dev/null; wait $SRV 2>/dev/null || true
```

**Observed:** `HTTP:201`; API `extraction_outcome` = `source_unavailable`; `list_label` = `Unavailable at source`; merged `title`/`employer`/`description_plain` = `null`; snapshot `diagnostics_json` includes `extraction_outcome:source_unavailable`, `source page signals job unavailable`, `fill fields manually; raw HTML preserved in snapshot`, `http_status:200`; `raw_body_html` length **1,260,849** bytes; `applications` count **0**.

### Case B — live public posting (not Fixture Labs)

**Ground truth (independent of JobDesk):**

```bash
JOB_ID=8009198
curl -sS "https://boards-api.greenhouse.io/v1/boards/flexport/jobs/$JOB_ID?content=true" \
  -o /tmp/flex_job.json
grep -o '"title":"[^"]*"' /tmp/flex_job.json
grep -o '"company_name":"[^"]*"' /tmp/flex_job.json
grep -o '"absolute_url":"[^"]*"' /tmp/flex_job.json
grep -o '"location":{[^}]*}' /tmp/flex_job.json
```

**Recorded URL:** `https://job-boards.greenhouse.io/flexport/jobs/8009198`
**Published (API):** title `Account Executive`; employer `Flexport`; location `Atlanta, Georgia, United States`; HTML `content` includes “About Flexport”, “The opportunity”, “You will”, “You should have” (qualifications).

**Ingest:**

```bash
PORT=3861
cargo run -p jobdesk -q -- --workspace "$OP_WS" serve --port $PORT --host 127.0.0.1 &
SRV=$!
sleep 4
CASEB='https://job-boards.greenhouse.io/flexport/jobs/8009198'
curl -sS -o /tmp/case_b2.json -w "HTTP:%{http_code}\n" \
  -X POST "http://127.0.0.1:$PORT/api/opportunities/ingest" \
  -H 'Content-Type: application/json' \
  -d "{\"url\":\"$CASEB\"}"
kill $SRV 2>/dev/null; wait $SRV 2>/dev/null || true
```

**Observed:** `HTTP:201`; `extraction_outcome` = `partial`; `list_label` = `Partial extraction`; diagnostics include `description_plain from structured HTML text`, `some fields missing; review before marking reviewed`, `http_status:200`; page has **no** `JobPosting` JSON-LD (confirmed on live HTML: 0 `ld+json` / `jobposting` hits); `applications` count remains **0**.

### Incidental error text (automated)

```bash
cd /home/voyager/Projects/resume
cargo test -p domain footer
cargo test -p domain unavailable_fixture
```

### Fallback truncation reporting (automated)

```bash
cargo test -p domain truncation
```

### Full workspace tests

```bash
cargo test --workspace
```

**Exit code:** 0 (all crate suites passed; 3 live transport tests remain `#[ignore]`).

---

## Expected vs observed (compact)

| Check | Expected | Observed | Pass |
|--------|-----------|----------|------|
| Case A outcome | `source_unavailable` | `source_unavailable` | Yes |
| Case A title/employer/description | empty / null | all `null` | Yes |
| Case A diagnostics visible | yes (API + snapshot) | yes | Yes |
| Case A raw source preserved | large snapshot HTML | 1,260,849 bytes stored | Yes |
| Case A no application | count unchanged | 0 applications | Yes |
| Case B live URL, no auth | fetch 200, store posting | 201, partial extract | Yes |
| Case B title field | `Account Executive` | `null` (title only inside `description_plain` text) | **No** |
| Case B employer field | `Flexport` | `null` (named in description prose) | **No** |
| Case B location field | `Atlanta, Georgia, United States` | `null` (line in description) | **No** |
| Case B job description body | role + responsibilities + qualifications | core copy present; includes apply-form chrome | Partial |
| Footer “page not found” on valid JSON-LD page | not `source_unavailable` | `recognized`/`partial` path; test passes | Yes |
| Long fallback description | diagnostic mentions truncation | `description_plain truncated to 24000 characters…` in unit test | Yes |
| Owner browser QA | Opportunities UI | **Not exercised in this pass** | Unverified |

---

## Case B — field comparison (ground truth vs stored `merged`)

| Field | Ground truth (Greenhouse API / public page) | JobDesk stored (`merged`) | Verdict |
|--------|-----------------------------------------------|---------------------------|---------|
| Title | Account Executive | `null` (visible as plain-text line 2 of description) | **Missing** (structured) |
| Employer | Flexport | `null` (“About Flexport” in description) | **Missing** (structured) |
| Location | Atlanta, Georgia, United States | `null` (same string in description) | **Missing** (structured) |
| Description (role / duties) | “The opportunity”, “You will” bullets | Substantively present in `description_plain` | **Match** (content) |
| Qualifications | “You should have” bullets (e.g. 5+ years…) | Same bullets present in `description_plain` | **Match** (content) |
| Noise | N/A (API `content` is job body only) | “Back to jobs”, apply form fields, demographic survey UI text appended | **Incorrect** (extraction pollution) |

---

## Manual browser acceptance (unverified here)

1. `cargo run -p jobdesk -- --workspace <path> serve` and open **Opportunities**.
2. Ingest Case A URL → confirm list label **Unavailable at source**, empty description in editor, diagnostics panel populated, **View source** shows large HTML.
3. Ingest Case B URL → confirm **Partial extraction**, description shows job prose, diagnostics mention fallback text; verify title/employer fields empty until user edit.
4. Confirm no new **Applications** rows after either ingest.

---

## Milestone L verdict (under limited acceptance contract)

Closure text: [`docs/MILESTONE_L_CLOSURE.md`](../MILESTONE_L_CLOSURE.md).

| Area | Status |
|------|--------|
| Limited contract (fetch, snapshots, interpretations, outcomes, no apps) | **Meets** |
| Unavailable posting (Case A) | **Meets** |
| SSRF-pinned fetch + atomic ingest + no application creation | **Meets** |
| Diagnostics + empty fields on unavailable | **Meets** |
| False unavailable from incidental footer text | **Met after scoped-detection fix** |
| Truncation transparency | **Meets** (unit test) |
| Case B Flexport HTML ingest | **Coverage limitation** — `partial` only; **not** complete structured extraction (see field table above) |
| Greenhouse structured fields | **Deferred** to Milestone M (Job Board API); **no** DOM heuristics in L |
| Opportunities UI | **Unverified** — **remaining owner browser gate** |

**Overall:** Milestone L is **ready for closure** on the explicit limited contract once owner browser acceptance passes. Flexport Case B is recorded as expected **partial** generic-HTML behavior, not a failed Milestone L obligation.

**Do not commit or push** until owner explicitly requests (after browser gate).
