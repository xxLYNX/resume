# Milestone N — Workday CXS job-detail adapter (bounded)

**Status:** Implemented (pre-acceptance).
**Reference tenant:** CrowdStrike — `https://crowdstrike.wd5.myworkdayjobs.com/…`

## Goal

When the user ingests a recognized **MyWorkdayJobs job-detail URL**, JobDesk fetches structured JSON from the browser-facing **CXS** endpoint, archives bytes immutably, and produces a machine interpretation with honest outcomes — same lifecycle as Milestones L/M.

## Non-goals

- Job search / board scraping (`POST …/jobs` only in tests or requisition resolution, not product UI).
- Application automation, `applications.posting_id`, portfolio analysis.
- Generic Workday tenant discovery beyond URL-parseable `{tenant}` + `{site}`.
- Inferring work authorization from the word “Remote” in location text.

## URL recognition

| Component | Rule |
|-----------|------|
| Host | `https://{tenant}.wd{N}.myworkdayjobs.com` (public DNS; existing SSRF policy) |
| Path | Optional `/{locale}/` then `{site}/job/{…}` where locale matches `xx-XX` |
| CXS GET | `https://{host}/wday/cxs/{tenant}/{site}/job/{tail}` |

Ingest URL identity remains the **normalized user URL** on `postings`.

## Extraction

- Source: `jobPostingInfo` in CXS JSON (`jobDescription` HTML → plain text via shared DOM walk).
- **Employer:** `hiringOrganization.name`.
- **Location:** `jobPostingInfo.location` (opaque string; no eligibility inference).
- **Employment type:** map `timeType` → `full_time` / `part_time` when recognized.
- **Work mode:** set `hybrid` only when title contains `(Hybrid)`; otherwise leave unset (not from “Remote” in location).
- **Requisition:** verify URL suffix `R####` (+ optional `-N`) against `jobReqId` when both present.

## Outcomes

| Case | Outcome |
|------|---------|
| HTTP 404 / CXS `errorCode` not found | `source_unavailable` (body archived when present) |
| JSON parse failure / post-req mismatch | `extraction_failed` |
| Missing title/employer or thin description | `partial` |
| Core fields + meaningful description | `recognized` |

## Persistence

- `retrieval_kind`: `workday_cxs_job_detail`
- `provider_provenance_json`: tenant, site, `workday_job_posting_id`, `job_req_id`, `external_url`, `cxs_request_url`, ingest URL.

Reuses `ingest_adapter_fetch` (shared with Greenhouse).

## Acceptance URLs (CrowdStrike)

| Req | Ingest URL (en-US) |
|-----|---------------------|
| R30561 | …/job/USA---Rancho-Cordova-CA/Data-Center-Technician--Santa-Clara--CA-_R30561 |
| R29756 | …/job/United-Kingdom---Remote/Technical-Support-Engineer---Cloud--Remote--GBR-_R29756 |
| R30634 | …/job/USA---Remote/Detection-Engineer--Falcon-Complete--Remote-_R30634 |
| R30635 | …/job/Canada---Remote-BC/Detection-Engineer--Falcon-Complete--Remote-_R30635 |
| R30636 | …/job/USA---Redmond-WA/Sr-Software-Engineer-II---Sensor--Linux-Guardian--Hybrid-_R30636-1 |
| R29568 | …/job/USA---Remote/Senior-Product-Security-Engineer--Application-Security--Remote-_R29568-1 |

Base: `https://crowdstrike.wd5.myworkdayjobs.com/en-US/crowdstrikecareers`
