# Milestone M — live Greenhouse acceptance (Flexport 8009198)

**Date:** 2026-11-12
**URL ingested:** `https://job-boards.greenhouse.io/flexport/jobs/8009198`
**API ground truth:** `GET https://boards-api.greenhouse.io/v1/boards/flexport/jobs/8009198?pay_transparency=true`
**Command:**

```bash
cargo run -p jobdesk -- serve --port 19552 &
curl -sS -X POST http://127.0.0.1:19552/api/opportunities/ingest \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://job-boards.greenhouse.io/flexport/jobs/8009198"}'
# HTTP 201
```

## Field comparison

| Field | API | JobDesk ingest | Result |
|-------|-----|----------------|--------|
| Post ID | `8009198` | Strict match on JSON `id` | **Correct** |
| Title | `Account Executive` (trimmed) | `Account Executive` | **Correct** |
| Employer | `Flexport` | `Flexport` | **Correct** |
| Location | `Atlanta, Georgia, United States` | Same | **Correct** |
| `first_published` | `2026-06-25T10:50:08-04:00` | Present in `machine_suggested` | **Correct** |
| `updated_at` | `2026-11-04T11:38:51-05:00` | `posting_updated_at` in machine JSON | **Correct** |
| `application_deadline` | `null` | Omitted | **Correct** (no fabrication) |
| `pay_input_ranges` | `[]` | `compensation_ranges: []`, `gap:pay_input_ranges` diagnostic | **Correct** |
| `requisition_id` | `P-11390` | In snapshot provenance JSON | **Captured in provenance** |
| `internal_job_id` | `3472585` | In provenance | **Captured** |
| Description | HTML `content` (intro + role + lists) | `description_plain` with headings/lists; includes board intro | **Substantive; not stripped** |
| Outcome | N/A | `recognized` | **Correct** |

## Provenance / storage

- `retrieval_kind`: `greenhouse_job_board_api`
- Snapshot `content_type`: `application/json`
- `request_url`: boards-api GET with `pay_transparency=true`
- `body_sha256`: hash of stored JSON bytes (`51d55efbe659e46fef5560f88ad6033639fe8b43ade8fe77a7e3689acae5762d` at acceptance time)
- `posting_fetch_attempts` row linked via `snapshot_id` (body not duplicated on attempt when snapshot exists)

## Applications

`applications` count unchanged after ingest (verified in integration tests).

## Known live limitations

- `pay_input_ranges` empty on this posting at acceptance time (not a JobDesk defect).
- Description includes Flexport board introduction per Greenhouse API contract (not job-only prose).

---

## Post-correction live smoke (HTTP transport fixes)

**Date:** 2026-11-12 (after correction pass)
**Fetcher:** `SecurePostingHttpFetcher` / `fetch_posting_document` (production path; **not** fixture manifest)
**Precondition:** `jobdesk import` on an isolated workspace so `jobdesk.db` exists before `serve`.

| Step | Value |
|------|--------|
| Ingest URL | `https://job-boards.greenhouse.io/flexport/jobs/8009198` |
| Provider GET | `https://boards-api.greenhouse.io/v1/boards/flexport/jobs/8009198?pay_transparency=true` |
| API HTTP status | **200** |
| Ingest HTTP status | **201** |
| Extraction outcome | **recognized** |
| Title | `Account Executive` |
| Employer | `Flexport` |
| Location | `Atlanta, Georgia, United States` |
| Latest attempt | HTTP **200**, outcome **recognized** |

### Snapshot integrity (same wire bytes as independent `curl` of API URL)

| Check | Result |
|-------|--------|
| `wire_body_sha256` (curl API body) | `51d55efbe659e46fef5560f88ad6033639fe8b43ade8fe77a7e3689acae5762d` |
| `posting_snapshots.body_sha256` | **Same** |
| Stored encoding | **utf8** (valid JSON; no `base64:` prefix) |
| `raw_body_html` bytes identical to wire | **Yes** |
| Overall | **PASS** |

Evidence captured at `/tmp/milestone-m-live-evidence.txt` on the acceptance host.
