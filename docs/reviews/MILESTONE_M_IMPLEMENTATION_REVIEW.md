# Milestone M — implementation review bundle (pre-commit)

**Date:** 2026-11-12
**Status:** Review materials prepared — **not committed**.

## 1. Interpretation selection and edit behavior

| Surface | Behavior |
|---------|----------|
| **API `GET /api/opportunities/{id}`** | `current` = latest interpretation; `last_usable` = latest `recognized`/`partial`; `latest_fetch_attempt` = newest attempt row; `prior_reviewed` unchanged (L). |
| **UI display** | `interpretationForDisplay()` uses `current` when usable, else `last_usable` (form + list titles). |
| **UI PATCH** | `PATCH …/interpretation` → store `interpretation_id_for_user_edit()`: patches **last usable** when latest is failed (fixed in this review). |
| **Test** | `store::postings::tests::patch_targets_last_usable_after_greenhouse_mismatch_refetch` — review → mismatch refetch → edit applies to first usable row, failed row `user_json` stays `{}`. |
| **E2E** | `greenhouse-ingest.spec.ts` — mismatch refetch shows “last usable” copy + prior fields. |

**Defect fixed this review:** PATCH previously always updated `ORDER BY created_at DESC, id DESC` (latest = failed). Now aligns with display contract.

## 2. Ordering and atomicity

| Area | Implementation |
|------|----------------|
| **Chronology** | `ORDER BY created_at/attempted_at DESC, rowid DESC` (SQLite insertion order tie-break; **not** UUID lexical order). |
| **Transactions** | Greenhouse ingest: `BEGIN IMMEDIATE` → attempt row (+ optional snapshot + interpretation) → `COMMIT`. Failed transport: attempt only. HTML path: unchanged single transaction. |
| **History** | No deletes on failed refresh; prior snapshots/interpretations retained. |

## 3. Source and security contracts

| Check | Result |
|-------|--------|
| **SHA-256** | `body_sha256` over stored `raw_body_html` bytes (`domain::sha256_hex` on ingest). |
| **HTML regression** | L JSON-LD fixture path unchanged; `save_posting_fetch_extended` defaults `http_html`. |
| **Body limits** | `read_body_limited` / `MAX_POSTING_BODY_BYTES` on document fetch; attempt `response_body` omitted when snapshot stores full JSON (no duplicate). |
| **SSRF** | Greenhouse API URL via `validate_public_fetch_url` + pinned client (L policy). |
| **Acceptance reset** | `POST /api/_acceptance/reset-opportunities` → **404** on normal serve (spot-checked). |

## 4. Real UI observation

| Method | What was verified |
|--------|-------------------|
| **Live API/curl** | Flexport ingest HTTP 201, `recognized`, structured fields — [`MILESTONE_M_LIVE_ACCEPTANCE.md`](MILESTONE_M_LIVE_ACCEPTANCE.md). |
| **Playwright** | Full JobDesk UI (Firefox + Chromium): Greenhouse fixture ingest, fields, JSON source preview, mismatch + last usable. |
| **Manual headed browser** | **Not** observed by implementer in this session; owner headed pass recommended before commit authorization. |

## 5. Documentation / fmt cleanup

- Spec persistence summary updated: `current` / `last_usable` (not `current_interpretation`).
- Unrelated `rustfmt` drift **reverted** on non-M domain/store/server files.

## 6. Validation commands (actual results)

```bash
cargo test --workspace -q          # PASS (post rowid + PATCH fix)
nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh  # see below
git diff --cached --check          # run after stage
```

**Playwright:** run after staging fixes — expect L + greenhouse specs (8 tests × 2 browsers).

## 7. Staged bundle

- Patch export: `/tmp/milestone-m-review.patch` (`git diff --cached --binary`).
- Inventory: see commit-readiness section in owner handoff.

## Unresolved / low risk

- `content_meaningful()` heuristics may need owner QA on edge fixtures.
- HTML ingest still does not write `posting_fetch_attempts` (deferred per spec).
- Headed manual UI sign-off pending owner.
