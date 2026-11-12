# Milestone M — Independent Patch Review Corrections

Bounded correction pass after architecture acceptance. **No commit** in this pass.

## Before → after (defect → reproducing tests)

| # | Defect | Before | After | Tests |
|---|--------|--------|-------|-------|
| 1 | HTTP failures swallowed / 429–5xx misclassified | `read_body_limited` used `unwrap_or_default()`; empty-body heuristic mapped rate limits to `source_unavailable` | Propagate `BodyTooLarge` / transport / read errors; `greenhouse_http_error_outcome` for 404 vs 429/5xx; 404 persists diagnostics | `server::posting_http::read_body_limited_*`, `greenhouse_ingest::http_*`, `opportunities::ingest_tests::greenhouse_empty_body_503_records_extraction_failed` |
| 2 | PATCH target ≠ visible interpretation | Server picked latest interpretation; stale tabs could write wrong row | `interpretation_id` on PATCH; `stale_interpretation` → **409**; UI sends `interpretationForDisplay().id` | `store::patch_stale_interpretation_id_rejected`, `patch_targets_last_usable_after_greenhouse_mismatch_refetch`, `opportunities::patch_stale_interpretation_returns_409`, Playwright stale-tab (`greenhouse-ingest.spec.ts`) |
| 3 | Failed latest fetch hidden when `last_usable` shown | Fetch line only when current outcome failed | `latestFetchFailureMessage(latest_fetch_attempt)` independent of display extraction | Playwright `review then transport-failed re-fetch…`, mismatch line still in `re-fetch mismatch` |
| 4 | Lossy UTF-8 storage / hash drift | Lossy string storage | `snapshot_body_for_storage` + raw-byte SHA-256 verify; view via `snapshot_body_for_view` | `domain::snapshot_storage_tests`, `store::invalid_utf8_snapshot_stores_base64_and_matching_hash`, `store::snapshot_sha256_matches_raw_bytes` |
| 5 | Description order / omission / duplication | Two-pass CSS `select()` reordered blocks; missed container text; weak `<br>` | Single source-order DOM walk (`ego-tree`); skip `script`/`style`; golden full-text fixtures | `greenhouse_content::golden_*` (order, mixed container, nested list, line breaks, Flexport sections, boilerplate bullets + `content_meaningful`) |
| 6 | Acceptance gaps | Partial GH E2E | App count guard, source preview, transport sequence in manifest | Playwright `greenhouse-ingest.spec.ts` (6 scenarios) + full workspace suites below |

## Validation (post-correction)

| Suite | Result |
|-------|--------|
| `cargo test --workspace` | **Pass** (121 tests + 3 ignored live-network) |
| `nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh` | **14/14 Playwright** (Firefox + Chromium) |

Post-correction live Greenhouse smoke: see [`MILESTONE_M_LIVE_ACCEPTANCE.md`](MILESTONE_M_LIVE_ACCEPTANCE.md) (Flexport 8009198, 201, `recognized`, SHA-256 match).

## Source snapshot storage contract (`raw_body_html`)

| Topic | Contract |
|-------|-----------|
| **Wire bytes** | HTTP response body as returned by `read_body_limited` (no lossy UTF-8 conversion before hash or store). |
| **`body_sha256`** | `sha256_hex(raw_wire_bytes)` — always the digest of the **original fetched bytes**, not the SQLite string encoding. |
| **UTF-8 JSON/HTML** | Stored as-is in `posting_snapshots.raw_body_html`; stored UTF-8 bytes must equal wire bytes. |
| **Non–UTF-8 bodies** | Stored as `base64:` + standard base64 of wire bytes (`SNAPSHOT_BODY_PREFIX_BASE64` in `domain::posting_fetch`). Diagnostic `snapshot_body:base64` on interpretation. Hash still computed on **wire bytes** before encoding. |
| **Ingest verify** | `ingest_greenhouse_fetch` rejects insert when `sha256_hex(snapshot_raw_bytes) != body_sha256`. |
| **UI / GET source** | `snapshot_body_for_view` decodes `base64:` for display (pretty JSON when valid); **does not** rewrite stored form. |
| **Exact byte recovery** | **UTF-8 snapshots:** read `raw_body_html` and encode as UTF-8 → original bytes. **Base64 snapshots:** strip `base64:` prefix, decode → original bytes (see `store::invalid_utf8_snapshot_stores_base64_and_matching_hash`). |

## Interpretation ID on Save and Mark Reviewed

| Layer | Behavior |
|-------|----------|
| **UI** | `86-opportunities.js` sends `interpretation_id: editingInterpretationId` (from `interpretationForDisplay`) on **both** Save edits and Mark reviewed PATCH. |
| **Store** | `update_posting_interpretation`: resolve server target via `interpretation_id_for_user_edit`; if client sends `interpretation_id` ≠ target → `stale_interpretation` error; `assert_interpretation_belongs_to_posting` enforces posting ownership. |
| **HTTP** | `patch_posting` maps `stale_interpretation` → **409 Conflict**. |
| **Tests** | `patch_stale_interpretation_id_rejected`, `mark_reviewed_stale_interpretation_id_rejected`, `opportunities::patch_stale_interpretation_returns_409`, Playwright two-tab stale save (409). |

## Staged patch export

```bash
git diff --cached --binary > milestone-m-review-final.patch
git diff --cached --check
```
