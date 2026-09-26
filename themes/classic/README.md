# Classic theme

Reference bundle for jobdesk. Copy this directory when authoring a new theme.

- **Contract:** [`../CONTRACT.md`](../CONTRACT.md)
- **Authoring guide:** [`../README.md`](../README.md)
- **Macros:** `macros/resume.jinja.tex` (registered as `classic_macros` in `theme.toml`)
- **Build config:** [`../../settings/jobdesk.yaml`](../../settings/jobdesk.yaml)

Golden parity: `cargo test -p render_tex`.
