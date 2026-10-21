# Themes

jobdesk renders resumes with **theme bundles** under `themes/<id>/`.

## Authoring

1. Copy [`classic/`](classic/) as a starting point.
2. Edit `theme.toml` (id, sections, `optional_fields`, optional `[macros]`).
3. Edit `preamble.tex` and `sections/*.jinja.tex` (MiniJinja).
4. Register the theme in `settings/jobdesk.yaml` (section order + template paths).

Full contract (context shapes, macros, guarantees): [`CONTRACT.md`](CONTRACT.md).

## Classic reference

[`classic/README.md`](classic/README.md) — golden test, macro file layout.

## ATS variant

[`ats-classic/`](ats-classic/) — same layout as classic with visible contact strings for text-extraction scanners.

Regression: `cargo test -p render_tex` compares normalized `.tex` to `tests/fixtures/golden_resume.tex`.
