# Milestone I — Application ledger & PDF export

**Status:** complete on `master` (branch closed).

Track job postings with enough metadata for search and a **single PDF ledger** of all applications, written to the user **Downloads** folder (e.g. `~/Downloads/jobdesk-applications.pdf` on Linux).

Not in scope: skills frameworks (F), resume composition (E), submission history UI (deferred post-E).

---

## Goals

1. **Richer application records** — compensation, work arrangement, employment type, location when relevant.
2. **Ledger PDF** — tabular (or compact list) export of all applications from SQLite, sorted for review (default: `updated_at` desc).
3. **Same pipeline as resumes** — generate `.tex` (or HTML) → existing PDF engine when available.

---

## Application fields (new)

| Field | Storage | Notes |
|-------|---------|--------|
| **Employment type** | `employment_type` | `full_time`, `part_time`, `contract`, `internship` |
| **Work mode** | `work_mode` | `remote`, `hybrid`, `on_site` |
| **Location** | `location` | City/region string; **required** when `work_mode` is `hybrid` or `on_site`; optional for `remote` |
| **Compensation** | `compensation_json` | See below |

### Compensation model (`compensation_json`)

User may enter a **single value** or **min–max range**. Pay may be **hourly** or **annual**.

```json
{
  "unit": "hourly",
  "min": 45,
  "max": 55
}
```

- Omitted `max` or `max == min` → display as single value.
- **`unit: "hourly"`** → ledger and UI also show **computed annual equivalent** for comparison:
  - **Full-time:** `hourly × 2080` (40 h × 52 w).
  - **Part-time:** `hourly × 1040` (20 h × 52 w) unless we add explicit `hours_per_week` later.
  - **Contract / internship:** use full-time hours unless `hours_per_week` is set (future); default 2080 for contract, 1040 for internship in v1.
- **`unit: "annual"`** → store and display as given (no conversion).

Validation lives in `domain` (API returns 400 on bad combinations).

---

## Ledger PDF

**Trigger**

- CLI: `jobdesk application export-ledger [--pdf]` (default: write PDF if engine present).
- UI: Applications tab → “Export ledger PDF” (writes default Downloads path via API).

**Output path**

- Default: `{user_home}/Downloads/jobdesk-applications.pdf` (create parent dirs if needed).
- Override: `--output /path/to/file.pdf`.

**Content (v1 columns)**

Company, title, phase, pipeline status, employment type, work mode, location, compensation (formatted + annual equiv when hourly), posting URL (shortened or footnote), last updated, submission count.

**Implementation sketch**

- `store::list_applications` (unfiltered or respect current filters via CLI flags later).
- `domain::format_compensation` + ledger row type.
- Minimal LaTeX template under `templates/ledger/` or inline in crate `application_ledger`.
- Reuse `render_pdf` compile step.

---

## Completion checklist

- [x] Schema migration (v6) + store/API types
- [x] Domain validation (work mode ↔ location, compensation shape)
- [x] Applications UI: fields on create/detail + unified Save
- [x] CLI + API: export ledger PDF to Downloads
- [x] Docs: `JOBDESK.md` command; ARCHITECTURE Milestone I checked

---

## Relationship to other milestones

| Milestone | Overlap |
|-----------|---------|
| **F (skills)** | None — tags stay tracker labels until F links skills |
| **E deferred** | Submission history could add “sent on” column to ledger later |
| **G/H** | Independent |

Merge **I** to `master` when checklist is done; does not block F.
