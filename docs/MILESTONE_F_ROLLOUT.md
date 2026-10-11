# Milestone F — execution roadmap (v10 competency model)

**Branch:** `dev/milestone-f`  
**Policy:** Not production. Breaking changes OK. **Single build path:** SQLite catalog → resolve → LaTeX/PDF (no parallel file-based content build).

**Legacy re-link checklist (human):** [`LEGACY_SKILLS_INVENTORY.md`](LEGACY_SKILLS_INVENTORY.md) — reads `content/skills.yaml` + element fields; not source of truth.

---

## Competency model v10

| Concept | Rule |
|--------|------|
| **Catalog** | `skill_nodes`: `tool` \| `competency`. |
| **Evidence** | `element_skill_links` on **elements**. |
| **Tool → competency** | `tool_competency_edges` + `GET /api/skills/suggestions`. |
| **Where** | experience, project, education, course, certification, award. |
| **PDF skills** | Evidence only — aggregate from pinned entries; theme `skills` section is render-only. |
| **No skills element kind** | `skills.main` / blob element removed (import, export, v11 migration). |

---

## Phase map

| Phase | Status | Outcome |
|-------|--------|---------|
| **F.0** | Done | Branch + plan |
| **F.2a** | Done | v10 schema, element links, evidence resolver, portfolio grouping v1 |
| **F.2-cleanup** | Done | Modular UI; `skills.main` retired; v11; file-based build removed |
| **F.2b** | Next | Skill discovery UI |
| **F.2c** | Planned | Relationships pane |
| **F.2d** | Planned | Cert/award/course on PDF/composition |
| **F.3** | Planned | Framework import; tailoring hints |

---

## F.2b — discovery (next)

- [ ] Typeahead pickers, payload scan (technologies + highlights), create tools for unmatched tokens.
- [ ] Competency suggestions apply button; applications filter autocomplete off.

---

## F.2c — relationships pane

- [ ] Store queries: skills for element; elements for skill id.
- [ ] Portfolio UI table/filters (same tables as resolver).

---

## F.2d — portfolio & PDF completeness

- [ ] Composition/resolver/theme for certification, award, course on résumé.
- [ ] Portfolio UX polish (add class from program context, etc.).

---

## F.3 — frameworks & tailoring

- [ ] Cache/import framework mappings onto `skill_nodes.framework_refs_json`.
- [ ] Read-only application hints (posting keyword overlap).

---

## Operations

- **DB:** v11 runs on open (drops legacy skills elements + composition sections). No wipe required.
- **Import:** `jobdesk import` seeds header/objective/education/experience/projects from `content/` — not `skills.yaml`.
- **Build config:** `settings/jobdesk.yaml` lists **section templates only**; content always from DB resolve.
