# Showcase résumé (J1)

One Showcase per workspace. It is not an application, a preset, or a second copy of the catalog.

Schema **v18** adds `showcase` and `showcase_publications`. Opening a database creates the singleton draft if it is missing. Existing `elements`, `applications`, and `submissions` rows are not rewritten.

## Draft

`showcase.id` is `default`. The first draft uses theme `classic` and the standard section keys with no items. It is not `default_composition`.

`save_showcase_draft` replaces `theme_id` and `draft_composition_json`. It does not change publication rows or `current_publication_id`.

Deleting a catalog element fails while the draft pins it. An approved publication does not keep that element alive. The publication already copied `resolved`.

## Publication

`approve_showcase` (`render_tex`) captures the draft once. That capture supplies the composition, theme, current publication pointer, and resolved data used both to render and to insert the publication. The database commit re-reads `theme_id`, the exact `draft_composition_json`, `updated_at`, and `current_publication_id` in the same transaction as the insert and the pointer update. If any of those differ, including a restore that moved the pointer, the attempt is a conflict: the new artifact directory is removed and the restored pointer stays. The early identical-approval return also requires the captured pointer to still be current.

An empty composition is rejected before theme load or PDF compilation.

Rendering reads theme files from `artifacts/theme_snapshots/<hash>/`, not from the live `themes/` tree. The recorded `theme_files_hash` is checked against that snapshot and approval stops if they differ. `settings/jobdesk.yaml` is loaded once per approval. That same configuration supplies the ordered enabled-section list and the PDF build. Application and submission builds still load the YAML on their own path. Template paths, section titles, and the preamble path in that file are replaced from the theme. The TeX job name is fixed to `resume.tex`, so `output.filename` does not change the PDF. Comments in the YAML are not part of the publication identity. The enabled-section list is part of `compute_showcase_snapshot_hash` together with the theme id, theme-file hash, composition, and resolved JSON. That hash is separate from submission `compute_snapshot_hash`.

Each approval compiles in its own `artifacts/showcase-build/<pid>.<nanos>.<seq>/` directory. The directory is removed after a successful read of the PDF and after a failed build.

The PDF is installed with a unique temporary name. Before that rename, the parent directories that hold the new publication directory entry are synced. The file is then `sync_all`'d and renamed. After the rename, the publication directory that holds `resume.pdf` is synced. A failure before the rename says the publication was not created. A directory-sync failure after the rename says the new PDF may already be installed and does not delete it. A SQL failure removes only the new publication directory. Unreferenced artifact directories are left in place.

Each publication row stores:

- the captured draft `theme_id` and composition
- `theme_files_hash` of the snapshot that was compiled
- denormalized `resolved_json`
- `snapshot_hash` from `compute_showcase_snapshot_hash`
- `pdf_relpath` `artifacts/showcase/<id>/resume.pdf`
- `pdf_sha256` of those PDF bytes

The insert and the current-pointer update are one transaction. Approving the same snapshot as the current publication returns that row and does not insert another. Later catalog edits and later draft edits do not update an existing row.

`restore_showcase_pointer` moves the pointer to an older id after the PDF is present and its checksum matches `pdf_sha256`. It does not delete history and does not replace the draft. A missing or mismatched PDF leaves the pointer where it was.

There is no application id on these tables.

## Failure

- Empty composition: no render, no row, no artifact directory.
- Theme load, a theme-hash mismatch, resolve, or PDF compile fails: no publication row. The build directory for that attempt is removed. A content-addressed copy under `artifacts/theme_snapshots/` may remain. It is not a publication.
- The draft changes after capture: conflict, new artifact removed, previous pointer unchanged.
- The PDF file cannot be installed: no row, previous pointer unchanged.
- Directory sync fails after the PDF rename: no row, previous pointer unchanged, and the new file may already be on disk. It is not swept up later.
- The database insert fails: only the new publication directory is removed. Previously approved PDFs and unreferenced artifact directories are left in place.

## Not in this milestone

Settings UI, Showcase browser UI, `jobdesk showcase export`, and the website publisher are not implemented.
