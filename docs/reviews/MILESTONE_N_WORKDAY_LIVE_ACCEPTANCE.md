# Milestone N — Workday CXS acceptance evidence

**Date:** 2026-11-12
**Status:** **Accepted** (final corrections + validation gate).
**Review patch:** [`milestone-n-workday-review.patch`](../../milestone-n-workday-review.patch)

---

## 1. Verification commands and exit codes

Captured in [`target/milestone-n-verification.log`](../../target/milestone-n-verification.log), Playwright log [`target/milestone-n-playwright.log`](../../target/milestone-n-playwright.log), and live smoke [`target/milestone-n-live-smoke.log`](../../target/milestone-n-live-smoke.log).

| Command | Exit code | Result |
|---------|-----------|--------|
| `cargo test --workspace` | **0** | All workspace tests passed (includes `workday_http_404_no_snapshot_preserves_reviewed`) |
| `nix-shell e2e/opportunities/shell.nix --run './scripts/run-opportunities-e2e.sh'` | **0** | **24 passed**, 0 failed (Firefox + Chromium) |
| `nix-shell -p rustfmt --run 'cargo fmt --all -- --check'` | **1** | **Fails on pre-existing repo-wide formatting drift** (unrelated to Milestone N). Touched Rust paths verified with `rustfmt --check`. |
| `git diff --check` | **0** | No conflict markers / trailing whitespace in Milestone N diff |
| `scripts/workday-live-smoke.sh` | **0** | Six live ingests into `target/milestone-n-workday-live-ws/jobdesk.db`; owner `workspace/jobdesk.db` posting count **unchanged** (7 → 7) |

**Browser environment:** Node/npm via `nix-shell e2e/opportunities/shell.nix` (required on this host).

---

## 2. Final correction evidence

### Correction 1 — HTTP error snapshot eligibility

- **Fix:** `try_ingest_workday` passes `snapshot: None` on non-2xx; error JSON stored on `posting_fetch_attempts` only (`crates/server/src/workday_ingest.rs`).
- **Regression:** `workday_http_404_no_snapshot_preserves_reviewed` (`crates/server/src/opportunities.rs`) — sequential mock on `fetch_posting_document`; asserts one snapshot, HTTP 404 attempt body, `source_unavailable`, reviewed interpretation remains `current` / `last_usable`.

### Correction 2 — Live smoke isolation

- **Fix:** `jobdesk serve` uses `--root "$ROOT" --workspace "$WS"` with `WS=target/milestone-n-workday-live-ws`; schema-only init via `application list` (no catalog import); owner DB count check before/after live ingests.

---

## 3. Playwright — Opportunities suite

**Command:**

```bash
nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh
```

**Workday file:** `e2e/opportunities/tests/workday-ingest.spec.ts`

**Final run:** 24 tests passed (~29s). Report: `target/e2e-opportunities-report/`.

---

## 4. Rust tests — persistence / adapter

| Area | Evidence |
|------|----------|
| Greenhouse unchanged | Existing GH unit + e2e tests |
| Workday domain | `crates/domain/src/workday.rs` tests + fixtures |
| Shared `ingest_provider_fetch` | `ingest_provider_fetch_sets_workday_retrieval_kind`, `provider_fetch_404_no_snapshot_attempt_has_body` |
| HTTP 404 through handler | `workday_http_404_no_snapshot_preserves_reviewed` |
| Schema v17 | `migrate_tests::fresh_database_schema_version_is_17` |

---

## 5. Live CrowdStrike ingest (isolated smoke ws)

Six requisitions ingested with outcome **recognized**, `retrieval_kind` **workday_cxs_job_detail**, **0** applications. Log: `target/milestone-n-live-smoke.log`.

Title fidelity notes (R29756, R29568) unchanged — CXS canonical titles vs URL slugs; see prior investigation in git history of this doc.

---

## 6. Review artifact

```bash
git add -N <milestone-n paths>
git diff --binary > milestone-n-workday-review.patch
```

Regenerated after final corrections (do not commit the patch file).

---

## 7. Unresolved limitations

- `cargo fmt --all -- --check` fails repository-wide on legacy files; Milestone N Rust sources pass per-file `rustfmt --check`.
- CXS contract verified on **CrowdStrike wd5** only.
- `PostingSnapshotSummary` API still omits `retrieval_kind` (use source endpoint / DB helpers).
- Owner `workspace/jobdesk.db` may still contain postings from an earlier non-isolated smoke run; smoke gate now prevents further owner mutation.
- Playwright `npm ci` reports **2 high** severity audit findings — recorded for later; `npm audit fix --force` not run.

---

## 8. Acceptance decision

| Criterion | Met? |
|-----------|------|
| HTTP-error path (no snapshot on 404) | **Yes** |
| Isolated live smoke | **Yes** |
| Workspace tests | **Yes** |
| Playwright 24/24 | **Yes** |
| Review patch regenerated | **Yes** |

**Milestone N:** committed on `master` (no push).
