# jobdesk UI static assets

Served at `/` (index) and `/static/*` (see `server` crate `ServeDir`).

| Path | Role |
|------|------|
| `index.html` | Shell markup only |
| `css/jobdesk.css` | Layout and component styles |
| `js/00-dom-state.js` | DOM refs, shared mutable state, preview/rebuild |
| `js/10-preview-themes.js` | Theme catalog and preview theme changes |
| `js/20-portfolio-list.js` | Catalog fetch, grouped portfolio list |
| `js/30-payload-element.js` | Payload forms, dates, element create/detail helpers |
| `js/40-composition.js` | Application composition editor (no `skills` section) |
| `js/50-presets.js` | Presets tab |
| `js/60-workday-catalog.js` | Workday fill scripts, catalog load/select helpers |
| `js/70-skills-element.js` | Skill catalog list, element tool/competency links |
| `js/71-skill-discovery.js` | Typeahead, payload scan, create tools, competency apply |
| `js/72-relationships.js` | Skill ↔ element relationships table and filters |
| `js/90-applications.js` | Applications tab CRUD, posting hints, draft editing |
| `js/95-bootstrap.js` | Tab wiring, catalog save/create, startup |

Scripts load in numeric order and share one global scope (classic scripts, not ES modules).
