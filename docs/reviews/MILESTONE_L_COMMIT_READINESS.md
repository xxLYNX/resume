# Milestone L — pre-commit validation (final)

**Date:** 2026-11-12
**Staged:** Milestone L implementation, tests, e2e, docs (see below).
**Not staged:** Milestone M spec, root operating-model drafts, `target/`, e2e `node_modules/`.
**Commit / push:** Not performed — awaiting authorization.

## 1. Browser-test isolation

| Control | Implementation |
|---------|------------------|
| Concurrency | `workers: 1`, `fullyParallel: false` in `playwright.config.ts` |
| Project ordering | Firefox then Chromium **never parallel** (single worker) |
| DB isolation | Single e2e workspace `target/e2e-opportunities-workspace/`; `POST /api/_acceptance/reset-opportunities` + fixture sequence reset in `beforeEach` |
| Serial tests | `test.describe.configure({ mode: "serial" })` on acceptance specs |

**Determinism (5 consecutive runs):**

```bash
./scripts/run-e2e-determinism.sh 5
# All 5 runs passed. (after list-status race fix in 86-opportunities.js v5)
```

**One intermittent failure observed before fix:** Firefox timed out on `#opp-status` “Interpretation updated” because `loadOpportunitiesList()` cleared status after save (same class of bug as ingest). **Fixed:** stop clearing `#opp-status` on list refresh; tests await PATCH/POST responses instead.

**Full suite command:**

```bash
nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh
# Playwright: 4 passed (2 specs × firefox + chromium)
```

## 2. Acceptance-mode safety

| Check | Evidence |
|-------|----------|
| Reset unavailable in production | `curl -X POST http://127.0.0.1:19550/api/_acceptance/reset-opportunities` → **404** (normal `serve`, workspace `target/fmt-check-ws2`) |
| Handler when `acceptance_testing: false` | `server::acceptance_safety_tests::reset_handler_not_found_when_acceptance_disabled` |
| Reset forbidden on `workspace/` dir even if flag on | `reset_handler_forbidden_on_production_workspace_dir` → **403** |
| CLI blocks fixtures on production workspace | `jobdesk serve --acceptance-fixtures … --workspace ./workspace` → **exit 1** with explicit error |
| Workspace name guard | `acceptance_e2e_workspace()`; required `e2e-opportunities-workspace` or `e2e-opportunities-*` |
| Positive control (fixture serve) | Playwright `acceptance-api-guard.spec.ts` |

Production network fetch unchanged: `SecurePostingHttpFetcher` unless `--acceptance-fixtures` is set **and** workspace name is allowed.

## 3. Pre-commit validation

| Step | Result |
|------|--------|
| `nix shell nixpkgs#rustfmt -c cargo fmt -p domain -p server -p store -p jobdesk -- --check` | **PASS** (Milestone L crates only; full-workspace `--check` reports pre-existing drift elsewhere) |
| `cargo build --workspace` | **PASS** |
| `cargo test --workspace` | **PASS** (17 server tests incl. 3 acceptance_safety) |
| Playwright full + determinism ×5 | **PASS** |
| `git diff --cached --check` | **PASS** (trailing whitespace fixed in staged docs) |

### Staged scope (44 files, ~6388 insertions)

- Rust: `posting_fetch`, `posting_http`, `opportunities`, `postings`, migrate v16, `jobdesk` acceptance serve flag
- UI: Opportunities tab + `86-opportunities.js` v5
- Tests: domain fixtures, store/server unit + HTTP tests
- E2E: `e2e/opportunities/` (Playwright, fixtures, Nix `shell.nix`)
- Scripts: `jobdesk-e2e-server.sh`, `run-opportunities-e2e.sh`, `run-e2e-determinism.sh`
- Docs: `MILESTONE_L_*`, `docs/e2e/OPPORTUNITIES_BROWSER_ACCEPTANCE.md`, reviews under `docs/reviews/MILESTONE_L_*`

### Explicitly excluded (untracked / not staged)

- `docs/MILESTONE_M_GREENHOUSE_ADAPTER_SPEC.md`
- `docs/00_*` … `docs/09_*`, `docs/PROPOSED_project.jobdesk_*`
- `target/`, `e2e/opportunities/node_modules/`, Playwright artifacts

## Milestone L verdict

Under the **limited acceptance contract** in `docs/MILESTONE_L_CLOSURE.md`, with executable browser regression and acceptance guards, Milestone L is **ready to commit** when authorized.
