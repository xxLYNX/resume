# Connector development playbook

**Purpose:** Repeatable procedure for adding **bounded** source adapters when a real job search surfaces a site JobDesk does not yet handle well.
**Not in scope:** Generic plugin framework, comprehensive ATS coverage, or authorization to implement new connectors from this document alone.

**Reference implementations:** Greenhouse Job Board API (Milestone M), Workday CXS job-detail (Milestone N).

---

## 1. Source discovery

Before writing code, document:

| Question | Greenhouse example | Where to record |
|----------|-------------------|-----------------|
| Who publishes the job? | Employer via Greenhouse boards | Issue / spec section |
| Public job-detail URL pattern? | `job-boards.greenhouse.io/{board}/jobs/{id}` | `domain::parse_greenhouse_job_detail_url` |
| Structured API? | `GET boards-api.greenhouse.io/v1/boards/{token}/jobs/{id}` | [Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html) |
| Stable provider IDs? | Post `id`, `internal_job_id`, `requisition_id` | `provider_provenance_json` |
| Terms / robots / rate limits | Employer + Greenhouse public API | Operational notes |

Deliverable: short **source brief** (one page): URL examples, ID glossary, and what “same job” means **on that provider** (not cross-posting merge).

---

## 2. Retrieval strategy (preference order)

1. **Documented public API** with stable schemas (preferred — M path).
2. **Structured metadata** in page (JSON-LD `JobPosting` — Milestone L default).
3. **Generic HTML** fetch + cautious visible-text fallback (`domain::posting_fetch`).
4. **Specialized DOM/heuristic extraction** only when justified and maintenance cost is accepted (Milestone M **defers** Greenhouse HTML scraping).

**Rule:** Do not silently substitute a weaker path when a stronger path was selected and failed (Greenhouse: no HTML fallback after API path — see M spec).

---

## 3. Identity and provenance

Persist separately and never conflate:

| Concept | JobDesk storage |
|---------|-----------------|
| User ingest URL | `postings.normalized_url` |
| Request URL actually fetched | Snapshot / `posting_fetch_attempts.request_url` |
| Provider canonical link | `provider_provenance_json` (e.g. API `absolute_url`) |
| Archived body bytes | `posting_snapshots.raw_body_html` + `body_sha256` |
| Retrieval path | `retrieval_kind` (`http_html` \| `greenhouse_job_board_api`) |

**Cross-posting:** Provider IDs are **metadata for this posting row only**. JobDesk does **not** auto-link postings sharing requisition or internal IDs (`ARCHITECTURE.md`, M spec).

---

## 4. Transport and security (reuse — do not fork)

| Control | Location |
|---------|----------|
| SSRF host allowlist / blocklist | `domain::posting_fetch` |
| Body size cap (`2_000_000` bytes) | `MAX_POSTING_BODY_BYTES` |
| Redirect limit | `MAX_POSTING_REDIRECTS` |
| Timeout | `posting_http` fetcher |
| Manual redirect handling | `crates/server/src/posting_http.rs` |

New connectors must call the existing fetch layer (`AppState::posting_fetcher`), not ad-hoc HTTP clients.

---

## 5. Extraction

| Principle | Implementation pattern |
|-----------|-------------------------|
| Source fidelity | Store bytes exactly received; hash stored bytes |
| Explicit vs inferred | Machine JSON separate from `user_json` overrides |
| Honest incompleteness | `extraction_outcome`: `recognized`, `partial`, `extraction_failed`, `source_unavailable` |
| Diagnostics | Human-readable `diagnostics_json` on interpretations and attempts |

**Greenhouse:** `domain::interpret_greenhouse_job_json`, `greenhouse_content` for description HTML → plain text.
**Generic:** `domain::extract_posting_from_html` (JSON-LD + fallback).

User-edited fields in the Opportunities UI are **corrections**, not re-labelled as machine-verified employer text unless the user explicitly treats them as such.

---

## 6. Failure semantics

Differentiate and surface separately:

| Class | User-visible signal | Persistence |
|-------|---------------------|-------------|
| Transport failure | HTTP 0, timeout, SSRF block | `posting_fetch_attempts`, outcome + diagnostics; Greenhouse may have **no** snapshot |
| Provider unavailable | 404/410, empty listing | `source_unavailable` interpretation when applicable |
| Malformed content | Unparseable JSON/HTML | `extraction_failed` + archived body when bytes exist |
| Incomplete extraction | Missing title/location but usable description | `partial` + diagnostics |

Never fabricate snapshot bodies. Attempt rows (v17) preserve failed retrieval audit trail for Greenhouse.

---

## 7. Acceptance checklist

Before declaring a connector **operationally ready**:

- [ ] **Fixtures** — Golden files under `crates/domain/tests/fixtures/` (HTML/JSON), deterministic unit tests.
- [ ] **Negative tests** — ID mismatch, EOE-only boilerplate, oversized body, SSRF blocked host.
- [ ] **Store integration** — Migration if new `retrieval_kind` or provenance shape; backfill strategy documented.
- [ ] **HTTP tests** — `crates/server/src/opportunities.rs` ingest tests with mock fetcher.
- [ ] **Browser workflow** — Playwright under `e2e/opportunities/` (see `scripts/run-opportunities-e2e.sh`).
- [ ] **Real smoke** — One live URL documented in `docs/reviews/*_LIVE_ACCEPTANCE.md` with `body_sha256` / outcome recorded.
- [ ] **No application side effect** — `count_applications` unchanged across ingest (asserted in tests).

---

## 8. Maintenance

| Event | Response |
|-------|----------|
| Provider changes JSON/HTML | Update adapter version (`extractor_version` / adapter id); add fixture from new shape |
| Ingest regressions | Compare `posting_fetch_attempts` diagnostics + snapshot hash |
| Employer migrates ATS | New connector brief; old URLs may go `source_unavailable` |
| Connector cost > value | Retire adapter path; document manual fallback; keep historical snapshots read-only |

Bump `GREENHOUSE_ADAPTER_VERSION` (or successor) when interpretation logic changes so interpretations remain attributable.

---

## Milestone M: reusable vs Greenhouse-specific

### Reusable (next connectors should copy)

- Posting / snapshot / interpretation lifecycle (v16+).
- `posting_fetch_attempts` + optional `snapshot_id` link.
- `retrieval_kind` + `provider_provenance_json` columns.
- Ingest orchestration in `opportunities::ingest_posting` (route by URL → adapter or generic).
- Transport stack (`posting_http` + `domain::posting_fetch` limits).
- UI: Opportunities list/detail, interpretation PATCH with stale `interpretation_id` → 409, source preview endpoint.
- Source view branching by `retrieval_kind` (HTML vs JSON display).
- Test harness pattern: inject `PostingFetcher` in server tests.

### Workday-specific (Milestone N)

- URL parser `parse_workday_job_detail_url` / `WorkdayJobRef`.
- CXS URL `workday_cxs_job_url` → `GET /wday/cxs/{tenant}/{site}/job/…`
- `try_ingest_workday` in `crates/server/src/workday_ingest.rs`
- JSON interpretation `interpret_workday_cxs_json` (`crates/domain/src/workday.rs`)
- `retrieval_kind`: `workday_cxs_job_detail`
- Spec: [`MILESTONE_N_WORKDAY_CXS_ADAPTER_SPEC.md`](MILESTONE_N_WORKDAY_CXS_ADAPTER_SPEC.md)

### Greenhouse-specific

- URL parser `parse_greenhouse_job_detail_url` / `GreenhouseJobRef`.
- API URL builder `greenhouse_api_job_url`.
- `try_ingest_greenhouse` in `crates/server/src/greenhouse_ingest.rs`.
- JSON interpretation + post-ID strict match + `content_meaningful` EOE guard (`crates/domain/src/greenhouse.rs`, `greenhouse_content.rs`).
- Provenance schema keys (`greenhouse_post_id`, `board_token`, etc.).

---

## Unsupported sources — operational response

When the user hits a URL JobDesk does not specialize:

### Automated path (today)

1. **Opportunities → Fetch & save** with the public job URL.
2. Server uses **generic `http_html`** unless URL matches Greenhouse pattern (`opportunities.rs` + `greenhouse_ingest::parse_gh_url`).
3. Outcomes:
   - **`recognized` / `partial`** — JSON-LD or fallback text; user **reviews** fields and marks reviewed.
   - **`extraction_failed` / `source_unavailable`** — User fills title/employer/description manually in interpretation form; diagnostics instruct to consult raw snapshot when present (`posting_fetch.rs`, `86-opportunities.js`).

### What provenance is preserved

| Case | Snapshots | Meaning |
|------|-----------|---------|
| HTTP success, body stored | Yes (`retrieval_kind=http_html`) | Verified capture **of what that URL returned at fetch time** — not cryptographic proof the employer authored it |
| Greenhouse API success | Yes (`greenhouse_job_board_api`) + provenance JSON | Structured capture with provider IDs |
| Transport failure (GH API) | Attempt row, often **no** new snapshot | Honest failure; prior snapshot unchanged |
| User only pastes text on **application** | No posting row | **Manual prose** — no fetch metadata; analyze can still run on paste |

### What must not be claimed

- Manually typed description on an application or edited opportunity fields without a successful snapshot must **not** be described as a verified employer-site capture.
- Partial extraction must **not** be reported as full structured success (UI shows extraction line + outcome).

### Practical fallback workflow (no new connector yet)

1. Save URL via Opportunities when fetch is allowed (SSRF/public).
2. If extraction is poor: open **View source** (`GET /api/opportunities/snapshots/{id}/source`), copy requirements into interpretation overrides or external notes.
3. Create **application** manually; paste same description into `posting_text` for **analyze/hints** (Milestone K).
4. For sites that block server fetch: owner may need off-tool capture (PDF, print) archived outside JobDesk until a connector or browser-assisted import exists — **not implemented** in v17.

---

## Authorization boundary

This playbook describes **how** to add connectors when prioritized. Each new provider still requires:

- Owner approval of scope
- Spec doc (like `MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md`)
- Acceptance runbook before operational reliance

No obligation to implement adapters preemptively before the job search begins.
