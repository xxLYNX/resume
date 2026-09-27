# Application lifecycle (phase + workflow)

## Why two layers?

Earlier versions used one **status** enum (`draft`, `submitted`, `interview`, …). That mixed:

- **Whether you have sent a resume** (immutable manifest), and  
- **Where the job is in your pipeline** (screening, interview, offer, …).

We split that into:

| Layer | Field (API/DB) | Meaning |
|-------|----------------|---------|
| **Super-status (phase)** | `phase` | `draft` = still preparing; `submitted` = you applied at least once (manifest exists or will on Submit). |
| **Pipeline (workflow)** | `status` | When `phase = draft`, only `draft`. When `phase = submitted`, one of the workflow values below. |

## Workflow values (`phase = submitted`)

- `submitted` — sent; no finer pipeline label yet  
- `screening`, `interview`, `offer`, `rejected`, `withdrawn`

## Rules

1. **New application** → `phase = draft`, `status = draft`.  
2. **Submit (freeze manifest)** → writes manifest, sets `phase = submitted`, `status = submitted`. Does not lock draft composition JSON (you may still edit; preview warns if draft ≠ last send).  
3. **Save** (tracker fields) may change `phase` / pipeline `status` only in valid combinations.  
4. **Cannot set `phase = draft`** after any submission exists — use workflow labels to track the job instead.  
5. Setting workflow to `interview` **without Submit** does **not** create a manifest; only **Submit** freezes what you sent.

## Migration (schema v4)

Existing rows: `status = draft` → `phase = draft`; any other status → `phase = submitted` with the same `status` as workflow.

## Filters

- `GET /api/applications?phase=submitted` — jobs you’ve moved past pure draft.  
- `GET /api/applications?status=interview` — workflow filter (works across submitted-phase rows).

See [`JOBDESK.md`](../JOBDESK.md) for CLI/UI.

## Revisit planned

**Milestone J** will revisit phase vs pipeline behavior in the UI and API. The rules above are the target model; current `serve` + Save paths have **known gaps** (e.g. pipeline/phase labels vs whether a canonical submission exists, filters that disagree with preview mode). Not critical for library/editor work; do not block J CRUD on a full lifecycle redesign.

Track issues and fixes in [`MILESTONE_J_RESUME_LIBRARY.md`](MILESTONE_J_RESUME_LIBRARY.md#phase--pipeline-revisit).
