# Milestone M — Greenhouse Job Board API adapter (revised specification)

**Status:** Implemented on branch (pre-commit). Baseline spec with kickoff clarifications below applied in code.
**Prerequisite:** Milestone L closed — [`MILESTONE_L_CLOSURE.md`](MILESTONE_L_CLOSURE.md).
**This document is spec-only.** Do not implement, commit, or push from this revision alone.

**Primary reference:** [Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html) — especially **Retrieve a job** (`GET /v1/boards/{board_token}/jobs/{job_id}`).

---

## Goal

When the user ingests a **recognized Greenhouse job-board job-detail URL**, JobDesk retrieves structured job content from Greenhouse’s **public Job Board API**, persists an **immutable JSON snapshot** with explicit **provenance**, and produces a **machine interpretation** with honest outcomes. Milestone L’s posting / snapshot / interpretation lifecycle is unchanged in spirit; Milestone M adds one **bounded** provider path.

**Non-goals (Milestone M v1):**

- Greenhouse HTML DOM scraping or employer-specific selectors.
- `questions=true` (application form schema) unless a **separately approved** requirement exists.
- Non-Greenhouse ATS adapters, plugin loading, automatic cross-source deduplication.
- `applications.posting_id`, ingest-time application creation, portfolio-wide analyze from postings.
- Custom HTTP stack (reuse L transport + SSRF policy).
- Silent fallback to generic HTML when the Greenhouse adapter path is selected and API retrieval fails.

---

## Conceptual model (unchanged from L, tightened for M)

| Layer | Meaning |
|-------|---------|
| **Posting** (`postings`) | One row per **normalized user URL** the user saved — not a canonical vacancy. |
| **Snapshot** | Immutable **successfully captured** source body (HTML or job JSON) + HTTP metadata + provenance. **No** empty or fabricated bodies. |
| **Fetch attempt** | Record of one ingest-time retrieval (success or failure), with truthful HTTP metadata; optional link to a snapshot. |
| **Interpretation** | Machine + user view for **one** snapshot; created only when a snapshot exists. |

**Cross-posting rule:** Greenhouse `internal_job_id`, `requisition_id`, and `absolute_url` are **metadata for this posting only**. They do **not** imply merge/link with other saved URLs or postings. Automatic deduplication remains **out of scope**.

---

## 1. Source and provider identity

Greenhouse exposes three distinct identifiers in Job Board JSON ([official list jobs docs](https://docs.greenhouse.io/job-board.html)): the response `id` is the **published job post ID**; `internal_job_id` is Greenhouse’s internal **job** ID; `requisition_id` is the employer’s requisition when present.

| Field (API) | Role in JobDesk | Rules |
|-------------|-----------------|-------|
| **`id` (job post ID)** | `greenhouse_post_id` in provenance + validation | Must match post ID from v1 URL path. **Strict validation:** JSON `id` MUST equal requested post ID (normalized). Mismatch → **`extraction_failed`** interpretation on **archived** JSON snapshot (received bytes stored). |
| **`internal_job_id`** | `greenhouse_internal_job_id` | Store when non-null; nullable (prospect posts). Never substitute for post ID in API requests. |
| **`requisition_id`** | `employer_requisition_id` | Store as opaque string when present; optional; never used as primary key. |

**Three URLs (all preserved, none required to be equal):**

| Stored concept | Source | Notes |
|----------------|--------|-------|
| **Ingest URL** | User-supplied URL (normalized on `postings.normalized_url`) | Authoritative identity of the saved posting row. |
| **Provider retrieval URL** | Actual GET issued to `boards-api.greenhouse.io` (including query string) | e.g. `…/jobs/8009198?pay_transparency=true`. Stored on snapshot and/or fetch attempt `request_url`. |
| **API `absolute_url`** | Field from JSON when present | Public/canonical link Greenhouse reports; may be employer careers site (`https://employer.com/careers?gh_jid=…`). **Do not** require literal equality with ingest URL. Surface in provenance + optional UI hint only. |

**Linking:** JobDesk does **not** create edges between postings that share `internal_job_id` or `requisition_id`. Future user-confirmed `posting_links` remain deferred (see L closure).

---

## 2. Persistence — schema v17 (resolved)

Milestone L **v16** stores snapshots in `posting_snapshots` ([`migrate_opportunity_postings_v16`](../../crates/store/src/migrate.rs)). **v17** adds provider metadata, **fetch attempts**, and tightens snapshot eligibility.

### 2.0 Snapshot eligibility (persistence contract)

A **`posting_snapshots` row is created only when** the ingest path has **actual source bytes** to archive:

| Path | Create snapshot when |
|------|----------------------|
| L HTML | HTTP response body stored per L (non-truncated policy); same as today. |
| Greenhouse API | Response body is **non-empty** and stored **exactly as received** (typically `application/json` job post on HTTP 200). |

**Never create a snapshot for:**

- Transport failures (timeout, DNS, SSRF block, no response bytes).
- Fabricated placeholders, empty `raw_body_html`, or invented JSON error envelopes.

**Always create a `posting_fetch_attempts` row** for every ingest/re-fetch on the Greenhouse adapter path (and optionally unify HTML path later; **v1: Greenhouse adapter attempts required, HTML may remain snapshot-only until aligned**).

When a snapshot **is** created, link the attempt via `snapshot_id`. When retrieval fails before archivable body, `snapshot_id` is **NULL**; the last successful snapshot and its interpretation(s) remain unchanged.

**`body_sha256`:** SHA-256 of the **exact UTF-8 bytes stored** in `raw_body_html` after any defined normalization for storage (store decoded body as received from fetch layer; hash **stored** bytes, not gzip wire format).

### 2.1 Migration v17 — `posting_snapshots` (additive columns)

| Column | Type | Purpose |
|--------|------|---------|
| `retrieval_kind` | `TEXT NOT NULL DEFAULT 'http_html'` | `http_html` \| `greenhouse_job_board_api` |
| `provider_provenance_json` | `TEXT NOT NULL DEFAULT '{}'` | Structured provenance (§2.2). |

**Unchanged column semantics (L-compatible):**

- `raw_body_html` — opaque archived source (HTML or JSON as stored).
- `content_type` — **stored** response MIME (`text/html`, `application/json`, etc.).
- `body_sha256` — hash of stored `raw_body_html` bytes (§2.0).
- Append-only: no UPDATE of prior snapshot bodies.

**Backfill:** existing rows → `retrieval_kind = 'http_html'`, `provider_provenance_json = '{}'`.

### 2.1b Migration v17 — `posting_fetch_attempts` (new table)

Minimal attempt history (Greenhouse path **required** in M v1):

```sql
CREATE TABLE posting_fetch_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    posting_id TEXT NOT NULL REFERENCES postings(id) ON DELETE CASCADE,
    attempted_at TEXT NOT NULL,
    retrieval_kind TEXT NOT NULL,
    request_url TEXT NOT NULL,
    final_url TEXT NOT NULL,
    redirect_chain_json TEXT NOT NULL DEFAULT '[]',
    http_status INTEGER NOT NULL,
    response_content_type TEXT,
    response_body TEXT,
    outcome TEXT NOT NULL,
    diagnostics_json TEXT NOT NULL DEFAULT '[]',
    snapshot_id TEXT REFERENCES posting_snapshots(id) ON DELETE SET NULL
);
CREATE INDEX idx_posting_fetch_attempts_posting
    ON posting_fetch_attempts(posting_id, attempted_at);
```

| Column | Notes |
|--------|--------|
| `response_body` | **Optional.** Raw bytes/text **when HTTP delivered a body** (including 404/5xx HTML/JSON error pages). NULL when transport failed before body. **Not** duplicated in `posting_snapshots` when no snapshot is created. |
| `outcome` | Same taxonomy as extraction outcomes where applicable (`recognized`, `partial`, `source_unavailable`, `extraction_failed`) plus retrieval-specific diagnostics. |
| `snapshot_id` | Set when this attempt produced an archivable snapshot + interpretation. |

**Interpretation rule:** New `posting_interpretations` row **only** when `snapshot_id` is set on the successful attempt chain (same transaction as today’s ingest).

### 2.2 UI / API — latest attempt vs usable interpretation

Posting detail (extend L `GET /api/opportunities/{id}`) must expose **both**:

| Field | Meaning |
|-------|---------|
| **`current`** | Latest interpretation row (may be `extraction_failed` after a bad refetch). |
| **`last_usable`** | Latest `recognized` or `partial` interpretation for display/edits when `current` is not usable. |
| **`latest_fetch_attempt`** | Most recent `posting_fetch_attempts` row (timestamp, `outcome`, `http_status`, short diagnostic summary, optional link to `snapshot_id`). |

UI status line should distinguish **“last fetch failed (timeout)”** from **current posting content** still showing the last good interpretation. Prior review banner behavior (L) applies to **interpretations**, not failed attempts.

### 2.3 Provenance object (`provider_provenance_json`)

When `retrieval_kind = greenhouse_job_board_api`, minimally:

```json
{
  "adapter_id": "greenhouse_job_board_api",
  "adapter_version": "1",
  "board_token": "flexport",
  "greenhouse_post_id": "8009198",
  "greenhouse_internal_job_id": 12345678,
  "employer_requisition_id": "REQ-50",
  "ingest_url": "https://job-boards.greenhouse.io/flexport/jobs/8009198",
  "provider_request_url": "https://boards-api.greenhouse.io/v1/boards/flexport/jobs/8009198?pay_transparency=true",
  "api_absolute_url": "https://…"
}
```

Omit keys when API field absent; do not invent values.

### 2.4 Source viewing (v16 HTML + v17 JSON) — owner A2

| `retrieval_kind` | Stored `content_type` | `GET …/snapshots/{id}/source` response |
|------------------|----------------------|----------------------------------------|
| `http_html` | Actual HTML MIME | `text/plain; charset=utf-8`, raw stored HTML (L). |
| `greenhouse_job_board_api` | Actual JSON MIME | **`text/plain; charset=utf-8`**, pretty-printed JSON **for humans only** (parse stored bytes → format; never persist formatted form). |

Snapshot row keeps truthful `content_type` (`application/json`); display encoding is not stored MIME.

### 2.5 Reinterpretation (deferred from M v1)

- **`POST /api/opportunities/{id}/reinterpret`:** **Out of scope for Milestone M v1.** No new review lifecycle in M.
- **In scope for M v1:** `domain` function **`interpret_greenhouse_job_json(bytes) -> MachineInterpretation`** (name TBD), callable from ingest and **unit tests on archived fixture JSON without network**.
- **Deferred (document only):** When reinterpret exists, whether “current” interpretation follows latest reinterpret vs latest snapshot-bound row vs user-pinned historical snapshot — **unresolved**; do not implement selection UX in M.

**Re-fetch (L, unchanged):** network fetch → attempt row → snapshot when eligible → new interpretation.

---

## 3. API behavior

### 3.1 Endpoint (official)

Per [Retrieve a job](https://docs.greenhouse.io/job-board.html):

```http
GET https://boards-api.greenhouse.io/v1/boards/{board_token}/jobs/{job_post_id}
```

**Path `{job_post_id}`:** Greenhouse documents this as the ID of the job post to retrieve; it is the **published post ID** (same as `id` in JSON and the `/jobs/{id}` segment on `job-boards.greenhouse.io`).

**Query parameters (v1):**

| Parameter | v1 usage |
|-----------|----------|
| `pay_transparency=true` | **Required on adapter requests.** Populates `pay_input_ranges` when configured ([docs](https://docs.greenhouse.io/job-board.html), [support article](https://support.greenhouse.io/hc/en-us/articles/10028084062491-Add-pay-transparency-to-a-job-post)). Map into machine JSON / diagnostics when present; absence is **not** failure. |
| `questions=true` | **Not used** in v1 (no application-form ingestion). |
| `content=true` | **Not used** on single-job GET (full post body including `content` is already returned on the single-job endpoint per official example). |

### 3.2 URL recognition (v1, explicit only)

**Adapter activates** only when normalized ingest URL matches:

1. Host: `job-boards.greenhouse.io` (HTTPS).
2. Path: `/ {board_token} /jobs/ {post_id}` with non-empty `board_token` and numeric `post_id`.

**Deferred (A3):** `gh_jid`-only or embed-style URLs without `/jobs/{post_id}` path — **not** in v1 recognition.

**No match** → Milestone L generic HTML fetch (unchanged).

**Match** → **API-only** retrieval for this ingest (see §5.6 — no silent HTML fallback).

### 3.3 Transport and security

- Reuse `SecurePostingHttpFetcher` / L policy: HTTPS only, SSRF checks, pinned public DNS, redirect re-validation, timeouts, max body size, no silent truncation.
- Allowlist host `boards-api.greenhouse.io` for adapter GETs (not employer-specific).
- Store actual `http_status`, `final_url`, redirect chain like HTML ingest.

### 3.4 Compensation

When `pay_input_ranges` is non-empty, copy structured ranges into `machine_json` (e.g. `compensation_ranges`) and/or diagnostics; do not collapse to prose unless display requires a short summary field.

---

## 4. Description handling

### 4.1 API `content` field

- Delivered as **HTML with HTML entities** ([official docs](https://docs.greenhouse.io/job-board.html)); decode entities before structure-aware text conversion.
- **Board-level introduction and conclusion** may be concatenated into `content` ([Board-level Introductions and Conclusions](https://docs.greenhouse.io/job-board.html)). Treat as **possible non-job prose**, not as noise to delete blindly.

### 4.2 Conversion policy (testable)

Implement a dedicated **`greenhouse_content_to_plain`** (name TBD) in `domain`:

1. Decode HTML entities (`&lt;` → `<`, etc.), including **multiply-encoded** sequences (e.g. `&amp;lt;` → `&lt;` → `<`) until stable or depth-capped (fixture-tested).
2. Parse as HTML fragment; walk DOM:
   - Headings (`h1`–`h6`) → plain lines with visible hierarchy (e.g. prefix or extra newline).
   - `li` → bullet lines (`- ` or `• `).
   - `p`, `br` → paragraph breaks.
3. Collapse excessive whitespace; preserve paragraph order.
4. **Do not** run aggressive “compliance stripper” or employer-boilerplate heuristics in v1.
5. Optional diagnostics when content is very short after conversion (see §5).

**Golden tests:**

- Flexport (or redacted) subset.
- Synthetic heading + list samples.
- **Multiply-encoded entity** markup (dedicated fixture files).
- **Short but valid** job description (recognition must not reject solely on length).
- **Board boilerplate–heavy** `content` (expects `partial` or meaningful-description failure per §5, not silent `recognized`).

### 4.3 Machine `machine_json` (adapter path)

Populate from API (not HTML scrape):

| Field | Source |
|-------|--------|
| `title` | `title` |
| `employer` | `company_name` when present — **never fabricate** |
| `location` | `location.name` when present |
| `description_plain` | §4.2 from `content` |
| `compensation_ranges` | `pay_input_ranges` when present |
| `language` | `language` when present |
| `application_deadline` | API when present |
| `first_published` | API when present |
| `updated_at` | API when present |

Omit JSON keys when API omits fields; do not invent timestamps or employer strings.

**Extractor identity:** `extractor_id = greenhouse_job_board_api`, `extractor_version` / adapter version **distinct** from `POSTING_EXTRACTOR_VERSION` (`2`) on HTML path.

---

## 5. Failure semantics

Three layers:

1. **Retrieval** — transport or HTTP without an archivable job JSON snapshot.
2. **Source unavailable** — authoritative “post not available” (e.g. 404).
3. **Extraction** — archivable body stored in snapshot, but machine interpretation cannot meet outcome rules.

Failed fetches **record `posting_fetch_attempts`**; **do not** create snapshots or new interpretations unless §2.0 snapshot eligibility is met. **Prior snapshots, interpretations, and user review state are never deleted or overwritten.**

### 5.1 Retrieval failures (no snapshot)

| Condition | Snapshot | Attempt row | Usable interpretation |
|-----------|----------|-------------|------------------------|
| Timeout, DNS, SSRF, connection reset | **No** | Yes (`http_status` 0, `response_body` NULL, `outcome` extraction_failed, `retrieval:transport` / `retrieval:timeout`) | **Unchanged** (latest prior) |
| HTTP **429** | **No** | Yes; store body if present | Unchanged |
| HTTP **5xx** | **No** | Yes; **store actual response body** when received | Unchanged |

v1: no automatic retry on ingest.

### 5.2 Source unavailable (typically no snapshot)

| Condition | Snapshot | Attempt |
|-----------|----------|---------|
| HTTP **404** / documented empty-unpublish pattern | **No** (unless policy later allows archiving error JSON as snapshot — **v1: no**) | Yes, `outcome = source_unavailable`, truthful `http_status`, body if any |

### 5.3 Extraction on archived JSON (snapshot **yes**)

When HTTP **200** delivers a body stored in `posting_snapshots`:

| Condition | Outcome | Snapshot |
|-----------|---------|----------|
| Body not parseable as JSON | `extraction_failed` | Yes — store **received** bytes |
| JSON parse OK, `id` ≠ requested post ID | `extraction_failed`, `greenhouse:post_id_mismatch` | Yes — store received JSON (truthful archive) |
| JSON parse OK, `id` matches, interpretation rules fail | `partial` or `extraction_failed` per §5.4–5.5 | Yes |

### 5.4 Outcome taxonomy (adapter, unified — kickoff lock)

**`recognized`** — post identity established and **all** required core fields present:

| Requirement | Rule |
|-------------|------|
| Post ID | Parsed URL post ID **equals** JSON `id` (normalized) |
| Title | Non-empty after trim |
| Employer | `company_name` non-empty |
| Description | `description_plain` passes **`content_meaningful()`** (§5.5) |

**Optional metadata** (location, compensation, requisition IDs, dates, `pay_input_ranges`) **does not** downgrade `recognized` → `partial`; emit `gap:*` diagnostics only.

**`partial`** — requested post ID matches, snapshot archivable, but **required** content missing or insufficient (e.g. empty title/employer, non-meaningful description).

**`extraction_failed`** — malformed source or post ID mismatch on archived body; or unparseable JSON.

**`source_unavailable`** — documented provider-unavailable (e.g. HTTP 404); attempt row, typically no snapshot.

**UI/API:** `current` = latest interpretation (may be failed). `last_usable` = latest `recognized` or `partial`. Display and forms prefer `last_usable` when `current` is not usable. `latest_fetch_attempt` always reflects the last try. Ordering: `(timestamp, id)` tie-break for attempts, snapshots, and interpretations.

### 5.5 `content_meaningful()` (owner A5)

**Not** a rigid character-count gate. Domain function evaluated in tests via **fixtures**:

- Combines signals: non-trivial token count after stop-word trim, presence of list/heading structure from §4.2, and rejection of **boilerplate-only** samples (dedicated fixture).
- **Short valid postings** must pass (dedicated fixture — e.g. concise role summary with bullets).
- Implementation documents the rule in code; acceptance uses golden files, not a single `>= 200` constant.

### 5.6 Fallback policy (resolved)

```
ingest(url)
  → validate URL (L)
  → if greenhouse_v1_job_detail_url(url)
       → parse (board_token, post_id)
       → GET Job Board API (§3.1) with L security
       → attempt row always
       → on archivable body: snapshot(JSON) → interpret(adapter)
       → else: attempt outcome per §5.1–5.2 — **stop** (no generic HTML fetch for same ingest)
     else
       → L HTML fetch → snapshot(HTML) → extract_from_html v2
```

**Rationale:** L already documented Flexport HTML as `partial`; M’s value is API truth. HTML fallback would hide adapter failures and reintroduce DOM noise.

**User URL on non-matching Greenhouse pages** (list pages, embeds): generic L path only.

---

## 6. Acceptance criteria (before / during implementation)

Document **expected results** and **failure evidence** (HTTP status, diag lines, snapshot count, `applications` count) for each case.

### 6.1 Offline / unit / integration

| ID | Test | Pass criteria |
|----|------|----------------|
| M-U1 | **Flexport API JSON fixture** (redacted/checked-in) | `recognized`; title, employer, location, dates when present; golden `description_plain` subset; provenance |
| M-U2 | **Post ID mismatch** | Fixture wrong `id` → snapshot with raw JSON, `extraction_failed`, `greenhouse:post_id_mismatch` |
| M-U3 | **Missing optional metadata** | No `requisition_id` / pay / dates → `recognized` if §5.4 core met; `gap:*` diags present |
| M-U4 | **Entity + multiply-encoded HTML** | Golden files for `greenhouse_content_to_plain` |
| M-U4b | **Short valid description** | `content_meaningful` → allows `recognized` with fixture |
| M-U4c | **Boilerplate-heavy content** | → `partial` (or failed meaningful check), not `recognized` |
| M-U5 | **Transport failure** | No new snapshot; attempt row; **prior interpretation unchanged**; snapshot count unchanged |
| M-U5b | **Malformed JSON (200)** | Snapshot with raw body; `extraction_failed` |
| M-U6 | **404 fixture** | Attempt only; `source_unavailable`; no snapshot |
| M-U7 | **Provenance + hash** | `body_sha256` matches SHA-256 of stored `raw_body_html` bytes |
| M-U8 | **Extractor offline** | `interpret_greenhouse_job_json(fixture_bytes)` without HTTP; no `/reinterpret` API |
| M-U9 | **Generic HTML regression** | **HTTPS** fixture URL per L URL policy (e.g. test server serving `greenhouse_job_posting.html`) → HTML path `recognized`; no `file:` or policy-breaking URL |
| M-U10 | **v16 HTML snapshot compat** | Source endpoint unchanged for pre-v17 rows |
| M-U11 | **API detail fields** | `latest_fetch_attempt` vs `current` interpretation on failed re-fetch |

### 6.2 Browser (Playwright, fixture-backed API)

Extend e2e acceptance **fixture manifest** to simulate `boards-api.greenhouse.io` responses (no live Greenhouse in CI).

| ID | Scenario | Verify |
|----|----------|--------|
| M-E1 | Ingest recognized Greenhouse job-board URL | UI shows title, employer, location; outcome **recognized** |
| M-E2 | View source | Pretty JSON visible; matches stored snapshot (modulo formatting) |
| M-E3 | Mark reviewed → re-fetch (fixture v2) | Prior review banner; new snapshot when successful; failed re-fetch shows attempt without losing prior review |
| M-E3b | Re-fetch transport fail (fixture) | UI shows failed attempt; **prior reviewed content still visible** |
| M-E4 | **Applications** | `applications` count **0** after ingest + re-fetch |
| M-E5 | API guard | Reset/provenance unchanged from L acceptance rules |

Determinism: keep `workers: 1`, serial tests (same as L e2e).

### 6.3 Live smoke (owner A4)

| ID | Test | Pass criteria |
|----|------|----------------|
| M-L1 | Live Flexport `8009198` or owner-designated posting | **Required for Milestone M sign-off**, not every standard CI run. Ingest → `recognized`; evidence artifact on failure |

Document runbook under `docs/reviews/` when implementing; CI may omit M-L1 by default.

**Failure evidence required:** HTTP status, response snippet, `extraction_outcome`, diagnostic list, snapshot count, whether post ID validation passed.

### 6.4 Explicit non-criterion

- **Not** a regression gate: Flexport **HTML-only** ingest remaining `partial` (L documented limitation). Adapter path must succeed for the **same** saved URL when pattern matches.

---

## 7. Architectural scope

**In scope:**

- One module: Greenhouse Job Board API adapter (URL parse, GET, validate, interpret).
- Minimal shared types: `retrieval_kind`, provenance JSON, `posting_fetch_attempts`, source view branching.
- Callable `interpret_greenhouse_job_json` (domain); **no** `/reinterpret` HTTP in v1.
- Schema v17 migration + store insert/read paths.
- Domain HTML-to-text helper for API `content`.
- Tests and fixtures listed in §6; e2e manifest extension.

**Out of scope:** plugins, multi-provider registry, deduplication, applications, portfolio analyze, `questions=true`, legacy `boards.greenhouse.io` embed hosts (v2 candidate).

---

## Owner decisions (locked)

| # | Decision |
|---|----------|
| A1 | **No invented source bodies.** Failed retrieval → `posting_fetch_attempts` only; snapshots only for real archived bytes (§2.0). |
| A2 | Human JSON source: **`text/plain; charset=utf-8`** pretty-print; stored `content_type` remains accurate (`application/json`). |
| A3 | **`gh_jid`-only URLs** deferred from v1 recognition. |
| A4 | **M-L1 live smoke required** for Milestone M sign-off; **not** required on every standard CI run. |
| A5 | **`content_meaningful()`** fixture-driven rules; **no** rigid 200-character-only gate. |

## Decisions resolved (architecture)

| # | Topic | Decision |
|---|-------|----------|
| D1 | Post vs internal vs requisition IDs | Stored separately; post ID strict match; no cross-posting merge |
| D2 | URL equality | Ingest, provider GET, and `absolute_url` stored separately; no equality required |
| D3 | Schema v17 | Snapshot columns + **`posting_fetch_attempts`** table; v16 HTML rows compatible |
| D4 | JSON source view | Pretty-print at read time; never mutate stored bytes |
| D5 | Reinterpret API | **Deferred**; domain extractor callable on archived JSON |
| D6 | API query | Single-job GET + `pay_transparency=true`; no `questions=true` |
| D7 | `content=true` | Not on single-job GET |
| D8 | Fallback | No silent HTML fallback when v1 URL matched |
| D9 | Failed refresh | Never delete prior snapshots/interpretations; attempt row always on adapter fetch |
| D10 | Description | Entity decode (incl. multiply-encoded) + structure-preserving plain text |
| D11 | Acceptance | Fixtures per §6; HTTPS-only generic HTML regression |
| D12 | Timestamps | Retain `application_deadline`, `first_published`, `updated_at` when API provides |

---

## Proposed implementation sequence

1. **Domain:** URL parser, post ID validation, DTO, `greenhouse_content_to_plain`, `content_meaningful`, `interpret_greenhouse_job_json`; tests M-U1–M-U8, M-U4b/c.
2. **Store v17:** migrate snapshots + `posting_fetch_attempts`; ingest transaction: attempt → optional snapshot → interpretation; posting detail includes `latest_fetch_attempt`.
3. **Server:** Greenhouse ingest branch, boards-api host allowlist, JSON source pretty-print (`text/plain`), extend opportunities UI for attempt vs current interpretation.
4. **Fixtures:** Flexport API JSON, entity-encoding samples, short/boilerplate content, 404/transport fakes.
5. **Integration:** M-U5, M-U11, M-U10; hash over stored bytes (M-U7).
6. **E2E:** M-E1–M-E5, M-E3b; update browser acceptance doc.
7. **Sign-off:** M-L1 runbook + manual evidence (A4).
8. **Docs:** `MILESTONE_L_OPPORTUNITY_POSTINGS.md` boundary only.

**Stop after specification until implementation kickoff is explicitly authorized.**

---

## Final persistence contract (summary)

```
postings (1 per normalized user URL)
  ├── posting_fetch_attempts*  (append-only; every Greenhouse ingest/re-fetch)
  │     snapshot_id → optional link when snapshot created
  └── posting_snapshots*     (append-only; only real archived bodies)
        └── posting_interpretations*  (append-only; only when snapshot exists)
```

- **Truth:** `raw_body_html` + `body_sha256` over **stored** bytes; `content_type` is stored MIME.
- **Failure:** attempt records HTTP truth (`http_status`, optional `response_body`); **no** empty snapshots.
- **UX/API:** `current` = latest interpretation; `last_usable` = latest recognized/partial; PATCH targets `last_usable` when `current` failed; `latest_fetch_attempt` = most recent try. Chronology tie-break: `(created_at|attempted_at, rowid)` not UUID lexical order.
- **L compatibility:** existing HTML snapshots unchanged; v17 columns backfilled.

---

## Unresolved issues (material review)

| Issue | Impact | M v1 stance |
|-------|--------|-------------|
| **Current interpretation selection after reinterpret** | Future UX when `/reinterpret` or multi-snapshot review exists | **Document only; no API in M** |
| **HTML ingest attempt history** | L path does not yet write `posting_fetch_attempts` | **Greenhouse-only in M**; optional HTML alignment later |
| **404 / error JSON as archivable snapshot** | Could debate storing error pages as snapshots | **v1: attempt row only** for 404; revisit if audit needs error archive |
| **`content_meaningful` edge cases** | Borderline board intro + short role text | **Resolve via fixtures + owner review of goldens at M QA**, not spec char limit |

None of the above block implementation kickoff once this spec revision is accepted.

---

## References

- Milestone L closure: [`MILESTONE_L_CLOSURE.md`](MILESTONE_L_CLOSURE.md)
- Flexport operational observation: [`reviews/MILESTONE_L_FINAL_OPERATIONAL_ACCEPTANCE.md`](reviews/MILESTONE_L_FINAL_OPERATIONAL_ACCEPTANCE.md) Case B
- Greenhouse Job Board API: https://docs.greenhouse.io/job-board.html
- L browser acceptance pattern: [`e2e/OPPORTUNITIES_BROWSER_ACCEPTANCE.md`](e2e/OPPORTUNITIES_BROWSER_ACCEPTANCE.md)
