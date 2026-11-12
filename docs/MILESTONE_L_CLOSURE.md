# Milestone L — Closure under limited acceptance contract

**Status:** **Closed** — committed under limited acceptance contract (2026-11-12).
**Date:** 2026-11-12
**Evidence:** Browser regression suite, acceptance guards, and reviews under `docs/reviews/MILESTONE_L_*`.

**Fix after initial browser QA (2026-11-12):** Re-fetch now drives the detail panel from the ingest `PostingDetail` (so `prior_reviewed` is applied immediately), with a visible prior-review banner and list hint `prior_reviewed_at`. Store prior-review lookup excludes the current interpretation id.

Milestone L delivers **source capture + interpretation lifecycle** for saved job URLs. It does **not** promise ATS-grade structured extraction for arbitrary career sites.

---

## Limited acceptance contract (in scope)

JobDesk **commits to** the following for v16 opportunity postings:

| # | Commitment | Evidence |
|---|------------|----------|
| 1 | **Securely fetch** publicly accessible HTTP(S) posting URLs (SSRF checks, pinned resolution, timeouts, size/type limits, no silent truncation of raw body) | `domain::posting_fetch`, `server::posting_http`, tests |
| 2 | **Preserve** retrieved sources as **immutable snapshots** (re-fetch appends; prior bodies retained) | `posting_snapshots`, `GET …/snapshots/{id}/source` |
| 3 | **Independent posting records** with normalized URL identity, separate from applications | `postings` table; ingest does not touch `applications` |
| 4 | **Machine interpretations separate** from user corrections | `posting_interpretations.machine_json` vs `user_json`; re-fetch creates new interpretation with empty `user_json` |
| 5 | **Accurate extraction reporting** via `extraction_outcome` (`recognized` / `partial` / `source_unavailable` / `extraction_failed`), diagnostics, and list labels | extractor v2, API + UI fields |
| 6 | **Never create an application** as an ingest side effect | handler guard + operational Case A/B (`applications` count 0) |
| 7 | **Do not claim** complete structured extraction where the generic HTML/JSON-LD path does not support it | UI labels, docs, and outcome taxonomy |

---

## Verified capabilities (Milestone L)

- Ingest by URL → posting row + snapshot + machine interpretation in one transaction (`BEGIN IMMEDIATE`).
- Unavailable sources (e.g. Google Careers dead listing): `source_unavailable`, cleared structured fields, diagnostics, large raw HTML preserved (Case A).
- Generic extraction: JSON-LD `JobPosting` when present; otherwise cautious HTML visible-text fallback with dump heuristics and explicit truncation diagnostics.
- Scoped **unavailable** detection (no false `source_unavailable` from incidental footer “page not found” when primary content is a real posting).
- Opportunities API and serve UI wiring (API and automated tests verified; **browser not verified** in CI/agent runs).

---

## Explicitly out of scope (deferred)

| Deferred | Owner direction | Next home |
|----------|-----------------|-----------|
| Greenhouse / ATS **DOM** scraping or page-specific HTML heuristics | **Not** the next correction | — |
| **Source-aware** adapters using provider **public APIs** | Preferred path | **Milestone M** — [`docs/MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md`](MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md) |
| Structured title/employer/location for Greenhouse job-board **HTML-only** pages | Coverage limitation, not a Milestone L defect | Milestone M |
| `applications.posting_id` / “start application from posting” | Deferred | Later milestone |
| Portfolio-wide analyze from posting content | Deferred | See `MILESTONE_L_OPPORTUNITY_POSTINGS.md` boundary |

---

## Documented coverage limitation — Flexport (Case B)

**Observed during operational acceptance** (not Fixture Labs):

- **URL:** `https://job-boards.greenhouse.io/flexport/jobs/8009198`
- **Ground truth:** Greenhouse Job Board API `GET https://boards-api.greenhouse.io/v1/boards/flexport/jobs/8009198?content=true` (title, `company_name`, location, HTML `content`).
- **JobDesk HTML ingest outcome:** `extraction_outcome: partial` — **not** a successful complete structured extraction.
- **Structured fields:** `title`, `employer`, `location` remained **null** (page has no `JobPosting` JSON-LD).
- **Fallback `description_plain`:** contains substantive role/qualification text but also **UI chrome** (apply form, surveys).

This is **expected under the limited contract**: generic HTML capture succeeded; machine interpretation honestly reported **partial**; full structured fields are **deferred** to Milestone M via the official API.

---

## Post-closure hardening note (out of L scope)

Acceptance mode today is gated by the `acceptance_testing` serve flag, registration of `POST /api/_acceptance/reset-opportunities` only when that flag is set, and a **workspace directory name** guard (`e2e-opportunities-workspace` or `e2e-opportunities-*`). That prevents accidental fixture fetch on `./workspace` and blocks reset on typical production workspace paths when the flag is on.

**Future hardening (no general security refactor implied):** tighten **path isolation**—e.g. allowlist an absolute e2e data root or require a dedicated subdirectory under `target/`, so acceptance cannot target arbitrary disks that happen to match the name pattern. E2E already uses `target/e2e-opportunities-workspace/` via `scripts/jobdesk-e2e-server.sh`.

---

## References

- Feature overview: [`MILESTONE_L_OPPORTUNITY_POSTINGS.md`](MILESTONE_L_OPPORTUNITY_POSTINGS.md)
- Operational acceptance: [`reviews/MILESTONE_L_FINAL_OPERATIONAL_ACCEPTANCE.md`](reviews/MILESTONE_L_FINAL_OPERATIONAL_ACCEPTANCE.md)
- Next milestone spec (no implementation until L committed): [`MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md`](MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md)
