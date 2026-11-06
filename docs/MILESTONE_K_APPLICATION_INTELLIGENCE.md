# Milestone K — Application intelligence

**Status:** complete on `master`.

Decision support and evidence planning for a target posting — not automated apply, not a cloud LLM product. Generative coaching runs in your editor via **MCP**; jobdesk supplies grounded facts from SQLite.

---

## Coaching loop (from real application workflows)

1. Paste or store **posting text** on an application.
2. **Analyze** — deterministic report: skill alignment, composition gaps, positioning notes.
3. Optional **MCP session** in Cursor — tools read the same DB; you add fit thesis, gap-closure plans, Q&A drafts.
4. **Portfolio / Resume** — add evidence, pin composition, preview PDF.
5. **Submit** — immutable manifest.

---

## Inputs

| Input | Source |
|-------|--------|
| `application_id` | Applications row |
| `posting_text` | Application field (fallback: title + company + tags) |
| Draft **composition** | Resume draft for that application |
| **Skill catalog** + **element_skill_links** | Portfolio evidence |
| Optional `coaching_json` | Future: user/LLM-filled thesis and notes (not required for K.1) |

---

## Analyze report (`ApplicationAnalyzeReport`)

JSON shape produced by `build_application_analyze` and `GET /api/applications/{id}/analyze`.

| Field | Meaning |
|-------|---------|
| `application_id`, `company`, `title` | Context |
| `posting_token_count` | Significant tokens extracted from posting |
| `posting_hints` | Same rows as F.3b hints API |
| `skill_gaps` | In posting, catalog match, but not on draft evidence |
| `skill_orphans` | On draft evidence, not mentioned in posting (trim list) |
| `composition_gaps` | Elements with posting-matched skills but excluded from draft |
| `suggestions` | Actionable strings (link evidence, include element, fix objective tone) |
| `objective_alignment` | `ok` \| `warn` \| `unknown` + detail when objective text conflicts with title tokens |

**Requirement status (per skill row in hints):**

- **evidenced** — in posting and on draft composition evidence
- **partial** — in posting, evidence exists in catalog but not on draft pins
- **gap** — in posting, no asserted evidence in catalog
- **extra** — on draft, not in posting (informational)

---

## K.1 — Deterministic layer

- Reuses [`posting_hints`](../crates/domain/src/posting_hints.rs) and store `skill_ids_on_composition`.
- CLI: `jobdesk application analyze <id> [--json]`
- API: `GET /api/applications/{id}/analyze`

No outbound network from `jobdesk serve`.

---

## K.2 — MCP server

```bash
cargo run -p jobdesk -- mcp serve --workspace workspace/
```

Stdio JSON-RPC ([Model Context Protocol](https://modelcontextprotocol.io/)). Tools (scoped, capped payloads):

| Tool | Purpose |
|------|---------|
| `list_applications` | Tracker list |
| `get_application` | One application + draft composition summary |
| `get_catalog_summary` | Element ids, kinds, latest revision snippets |
| `get_catalog_element` | Full element detail (all revisions + latest payload; not truncated) |
| `get_catalog_revision` | One revision payload by id |
| `get_element_usage` | Applications/presets referencing an element |
| `get_skill_evidence` | Links for an element id |
| `posting_hints` | Hint rows for an application |
| `analyze_application` | Full K.1 report |
| `get_composition` | Draft composition JSON for an application |

Configure in Cursor: MCP server command pointing at `jobdesk mcp serve` with `--root` set to this repo.

**K.4 posting fetch:** not in core Rust; paste posting text or use Cursor `WebFetch` / browser tools in the agent.

---

## K.3 — UI

Applications tab: **Analyze posting** expands to structured sections (gaps, suggestions, hints table). Link to **Resume** tab with application target pre-selected when possible.

---

## Privacy

- Default: no network from analyze or MCP (local DB only).
- Summary tools cap text length; `get_catalog_element` / `get_catalog_revision` return full JSON up to 512KB or an explicit error (no silent truncation).
- Local MCP does not imply an external LLM keeps data private — do not feed classified/NDA material into chats.

---

## Non-goals (unchanged)

- Auto-submit to job boards
- Required cloud LLM in jobdesk binary
- Workday scraper in Rust core

Optional local storage of user-reviewed Q&A drafts may be added later as `coaching_json` on applications without changing the above.
