# Milestone L — Opportunity posting ingestion (v1)

**Closure:** [`MILESTONE_L_CLOSURE.md`](MILESTONE_L_CLOSURE.md) — limited acceptance contract, verified vs deferred.
**Next (spec only):** [`MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md`](MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md).

## Scope (this increment)

- Persist **postings**, **immutable snapshots**, and **interpretations** (generic machine extraction from JSON-LD + HTML fallback — **not** ATS-specific adapters).
- **No** application rows from ingestion; `applications.posting_id` is **deferred**.
- Server **HTTP fetch** with SSRF URL checks, size cap, content-type gate, timeout (see `domain::posting_fetch` + `server::posting_http`).
- UI: **Opportunities** tab — fetch & save, list, edit interpretation, view raw snapshot.

## Limited acceptance contract

Milestone L is **accepted on** secure public fetch, immutable source history, separated machine/user interpretation, honest `extraction_outcome` reporting, and **zero** application side effects. It is **not** accepted on complete structured extraction for every career site. Sites without `JobPosting` JSON-LD may yield **`partial`** or **`extraction_failed`** only; see Flexport in the closure doc.

## Verified vs deferred

| Verified in L | Deferred (not L) |
|---------------|------------------|
| Posting + snapshot + interpretation ingest | Greenhouse Job Board **API** adapter (Milestone M) |
| JSON-LD `JobPosting` when present in HTML | DOM/heuristic scraping for Greenhouse HTML |
| Cautious HTML visible-text fallback + diagnostics | Full structured fields for HTML-only Greenhouse pages |
| `source_unavailable` for dead listings (e.g. Google Careers) | `applications.posting_id` workflow |
| Outcomes: recognized / partial / unavailable / failed | Portfolio-wide analyze from posting |

## API

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/api/opportunities/ingest` | Body `{ "url": "https://..." }` — fetch, append snapshot, new interpretation |
| GET | `/api/opportunities` | List postings |
| GET | `/api/opportunities/{id}` | Detail + snapshots + current interpretation |
| GET | `/api/opportunities/snapshots/{id}/source` | Raw stored retrieval body (HTML today) |
| PATCH | `/api/opportunities/{id}/interpretation` | Body `{ "user_overrides": { ... }, "mark_reviewed": true }` |

## Review semantics (v1)

- **Current interpretation** = row with greatest `(created_at, id)` for the posting.
- Each **re-fetch** inserts a new snapshot and a **new interpretation** with **empty** `user_json` (prior rows, including user edits and `reviewed_at`, are preserved).
- **`prior_reviewed`** on `GET /api/opportunities/{id}` surfaces the latest reviewed row when the current row is unreviewed (e.g. after re-fetch).
- **PATCH** with `user_overrides` on a reviewed interpretation **clears** `reviewed_at` unless `mark_reviewed: true` is sent in the same request.

## Schema

**`schema_version` 16** — tables `postings`, `posting_snapshots`, `posting_interpretations` (see `store::migrate::migrate_opportunity_postings_v16`).

## Coverage note — Flexport Greenhouse HTML

Ingesting `https://job-boards.greenhouse.io/flexport/jobs/8009198` via the **generic HTML path** produced **`partial`** extraction: substantive text in `description_plain` but **no** structured title/employer/location and UI noise in fallback text. This is a **documented limitation**, not complete extraction success. Structured Flexport fields belong to Milestone M via `boards-api.greenhouse.io`.

## Next increment — portfolio-wide alignment (boundary)

Application analyze today (`domain::build_application_analyze`) takes a **composition** (draft). For posting-level work:

- Add a separate entry point that supplies **portfolio-wide** inputs: e.g. all catalog element IDs or a dedicated “evidence projection,” not a default résumé composition.
- Reuse **token / hint** primitives from `posting_hints` and `application_analyze` without copying; keep **posting** and **application** reports as distinct result types.
- Do **not** treat absent taxonomy links as “proof of gap”; keep uncertainty explicit.
