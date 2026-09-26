# Classic theme (jobdesk)

Reference theme for jobdesk. **Go `templates/` is legacy and not maintained.**

- **Contract:** [`../CONTRACT.md`](../CONTRACT.md)
- **Macros:** `macros/resume.jinja.tex` — import as `classic_macros` in section templates
- **Edits:** this directory only (`theme.toml`, preamble, sections, macros)
- Configured from [`settings/jobdesk.yaml`](../../settings/jobdesk.yaml) (section list merges into theme bundle over time)

Golden parity: `cargo test -p render_tex` compares normalized `.tex` to `tests/fixtures/golden_resume.tex`.
