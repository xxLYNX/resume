# Milestone F — Skills catalog & framework refs

**E** is the next implementation focus (composition UI, themes, catalog forms, presets). **F** is the next **unused letter that stays purely on the roadmap** until E lands — dedicated to skills, not UniFFI.

UniFFI and portfolio move to **G** and **H** (see [`ARCHITECTURE.md`](../ARCHITECTURE.md)).

---

## Problem today

- One catalog element `skills.main` holds all skills text for LaTeX.
- Application **tags** are job-tracker labels, not canonical skill ids.
- CHECKLIST [`framework_refs`](../CHECKLIST.md) (O\*NET / ONET, NICE KSAs, ESCO, etc.) were never modeled. **Deliberately deferred until E is complete** — frameworks are not composition toggles.

---

## Target model

### `skills` table (catalog)

| Column | Purpose |
|--------|---------|
| `id` | Stable canonical id (e.g. `skill.pentest`) |
| `label` | Default display string |
| `aliases_json` | Posting-friendly variants (`penetration testing`, `PT`) |
| `tags_json` | Buckets (`offensive`, `cloud`, `compliance`) |
| `framework_refs_json` | External mappings (see below) |

Elements/revisions may still hold rendered skill **lines** for backward compatibility during migration; resolver eventually picks labels/aliases from catalog ids referenced in **composition** (or a dedicated composition section for skills).

### `framework_refs` (on skill row)

Structured optional refs, e.g.:

```json
{
  "onet": { "element_id": "...", "occupation_code": "15-1212.00" },
  "nice": { "work_role": "...", "ksa_ids": ["K0001", "S0002"] },
  "esco": { "uri": "http://data.europa.eu/esco/skill/..." }
}
```

Store **refs only** in v1 (lookup / enrichment / future JD matching). Same pattern for other taxonomies the user adds via import files.

**v1 F scope:** store and edit refs; **import** from bundled or user-supplied mapping files (CSV/JSON). **Not** required to auto-rewrite resume bullets from NICE text.

**Later:** JD keyword suggest (“this posting mentions KSA K0044 → skills you have”), portfolio filters, compliance reporting.

---

## Composition & resolve

- Composition pins **skill ids** (+ optional chosen **alias** per id for this application/preset).
- Resolver expands to the shape `themes/classic` skills template expects (or a v2 template with explicit skill list).
- De-dupe rule (CHECKLIST): one canonical id; aliases are display only.

---

## Milestone F checklist (draft)

- [ ] Migration: `skills` table; optional link table `composition_skills` or skills section in composition JSON schema v2
- [ ] Import/migrate from current `skills.main` revision blob → catalog rows
- [ ] CRUD API + minimal UI (list/edit skill, aliases, tags, framework refs)
- [ ] Resolver: composition → render model with chosen aliases
- [ ] Optional: import enrichment files (O\*NET, NICE, ESCO, …) into `framework_refs`
- [ ] Docs: how refs relate to job tags vs skill tags

---

## Out of scope for F (other milestones)

| Item | Milestone |
|------|-----------|
| Application content picker (section + element inclusion) | E — list collections first; per-skill rows when F splits `skills.main` |
| Theme bundles / `theme_files_hash` | E |
| Presets (full composition, no job) | E |
| Portfolio / public site | G |
| UniFFI | H |
