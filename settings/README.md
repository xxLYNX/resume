# Settings

Configuration for **jobdesk** (Rust). These files are not used by the Go builder.

| File | Purpose |
|------|---------|
| `jobdesk.yaml` | Section order, content paths, and **classic** theme (`themes/classic/`) |

The Go builder uses [`config.yaml`](../config.yaml) and [`templates/`](../templates/) only. Do not point Go at Jinja templates under `themes/`.
