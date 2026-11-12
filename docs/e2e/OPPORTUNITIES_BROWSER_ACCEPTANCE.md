# Opportunities — browser acceptance (Playwright)

Permanent regression gate for Milestone L prior-review UX and ingest workflow. Uses the **real** JobDesk UI and HTTP API (no mocked JSON). Posting HTML is served via **`--acceptance-fixtures`** only (production fetch policy unchanged).

## Command

**Linux / generic (Node installed):**

```bash
./scripts/run-opportunities-e2e.sh
```

**NixOS (recommended here):**

```bash
nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh
```

**Firefox only / Chromium only:**

```bash
cd e2e/opportunities
nix-shell --run 'npx playwright test --project=firefox'
```

Artifacts on failure: `target/e2e-opportunities-artifacts/` (screenshots, traces), HTML report `target/e2e-opportunities-report/`.

## What it covers

Single spec `e2e/opportunities/tests/opportunities-prior-review.spec.ts`:

1. Opportunities tab → fixture ingest → list + detail
2. Edit title → save → mark reviewed → visible “Reviewed at”
3. Reload → reopen → title + reviewed persist
4. Re-fetch → title reset to machine V2 (no copied overrides)
5. **Prior-review banner visible** with matching interpretation id + timestamp
6. List shows `prior review <timestamp>`
7. No `prior_review` banner on first ingest
8. `GET /api/applications` length 0

`beforeEach` calls `POST /api/_acceptance/reset-opportunities` (fixture serve only) to reset DB + fixture sequence.

## Environment

| Piece | Location |
|--------|-----------|
| Fixture manifest | `e2e/opportunities/fixtures/manifest.json` |
| HTML v1 / v2 | `job-v1.html`, `job-v2.html` |
| Serve wrapper | `scripts/jobdesk-e2e-server.sh` |
| Port | `19547` (`JOBDESK_E2E_PORT`) |
| Workspace DB | `target/e2e-opportunities-workspace/` |

## NixOS notes

- Plain `npm playwright install` browsers are **not** usable on NixOS (glibc / store path). Use `e2e/opportunities/shell.nix`, which sets `JOBDESK_E2E_FIREFOX` / `JOBDESK_E2E_CHROMIUM` to nixpkgs `playwright-driver.browsers` + pins `@playwright/test@1.49.1`.
- `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` in the shell.

## CI

Run when a browser+Nix or Node+Playwright image is available. Keep **live ATS** tests separate (`#[ignore]` Rust network tests).

## Initial run (pre-fix) failures

Automated tests were added **before** final fixes. Observed failures that matched owner symptoms:

| Assertion | Mechanism |
|-----------|-----------|
| `#opp-status` never showed “Saved posting” | `loadOpportunitiesList()` cleared status **after** ingest set it (race) |
| Prior banner visible on “first” ingest | Stale DB + **two Playwright projects** sharing one server; API returned real `prior_reviewed` from prior project run |
| Playwright timeout on status text | Tests waited on status line instead of network/UI field outcomes |

## Fixes applied

- Reorder ingest handler: apply detail **before** final status; status set **after** list refresh (`86-opportunities.js?v=4`).
- `applyPostingDetail` uses ingest `PostingDetail` (includes `prior_reviewed`).
- Prior-review banner styling without `color-mix` (Firefox visibility).
- Acceptance-only: `POST /api/_acceptance/reset-opportunities` + fixture sequence reset.
- Playwright: wait on ingest HTTP response + field/banner visibility, not `#opp-status` alone.

## Post-fix evidence

```text
nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh
# playwright: 2 passed (firefox + chromium)
# cargo test --workspace: ok
```

**Not committed** until owner review.
