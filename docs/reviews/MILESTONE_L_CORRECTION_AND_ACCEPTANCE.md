# Milestone L — Corrective Hardening and Operational Acceptance

**Baseline Git revision:** `97fb46d2428f887327b5d2ad070f32536eddeaad` (unchanged commit; working tree includes Milestone L + corrections)
**Evidence run date:** 2026-11-06 (UTC)
**Prior audit:** [`MILESTONE_L_IMPLEMENTATION_AUDIT.md`](./MILESTONE_L_IMPLEMENTATION_AUDIT.md) → **Reject pending correction**

---

## 1. Reproducible baseline

### 1.1 Commands (pre-correction)

```bash
git rev-parse HEAD   # 97fb46d2428f887327b5d2ad070f32536eddeaad
cargo test --workspace   # exit 0, 67 tests (pre-correction count)
```

### 1.2 Findings demonstrable via failing test before fix

| Finding | Demonstration approach | Post-fix test |
|---------|-------------------------|---------------|
| Non-atomic `save_posting_fetch` | SQLite trigger aborts interpretation insert → posting row must not persist | `store::postings::tests::save_posting_fetch_rolls_back_when_interpretation_blocked` |
| FK off by default | Orphan snapshot insert succeeds without PRAGMA | `foreign_keys_reject_orphan_snapshot` (fails if FK disabled) |
| Stale `user_json` on re-fetch | Store test with edited overrides + second fetch | `refetch_starts_with_empty_user_overrides` |
| Review stale after edit | PATCH overrides without `mark_reviewed` | `editing_reviewed_without_remark_clears_review` |
| Hostname-only SSRF | **Not** a single failing test before fix (design gap); corrected via new client + resolver tests | `resolve_public_socket_addr_rejects_*`, `fetch_policy_rejects_*`, live `#[ignore]` fetch |

---

## 2. Corrections (summary table)

| ID | Original finding | Root cause | Mechanism | Key files |
|----|------------------|------------|-----------|-----------|
| **C1** | Blocker SSRF / no connect-time policy | `ureq` auto-redirect + string-only host check | Custom `SecurePostingHttpFetcher`: per-hop URL validate, `resolve_public_socket_addr`, `TcpStream::connect_timeout` to vetted address, manual redirects, TLS SNI, gzip cap | `crates/server/src/posting_http.rs`, `crates/domain/src/posting_fetch.rs` |
| **C2** | Non-transactional ingest | Sequential `execute` without `BEGIN` | `BEGIN IMMEDIATE` … `COMMIT` / `ROLLBACK` around posting + snapshot + interpretation | `crates/store/src/postings.rs` |
| **C3** | FK not enforced | Missing `PRAGMA foreign_keys` | `PRAGMA foreign_keys = ON` in `Store::open` | `crates/store/src/lib.rs` |
| **C4** | Stale overrides after re-fetch | Copied `user_json` into new interpretation | New interpretation always `{}`; history preserved in older rows | `crates/store/src/postings.rs` |
| **C5** | Review semantics | `reviewed_at` not tied to edits | `prior_reviewed` on detail; PATCH clears review when overrides change without re-mark | `postings.rs`, `86-opportunities.js` |
| **C6** | Blocking HTTP in async handler | Sync fetch in async fn | `tokio::task::spawn_blocking` | `crates/server/src/opportunities.rs` |
| **C7** | XSS surface (`innerHTML`) | List/meta HTML injection | `createElement` + `textContent` | `86-opportunities.js` |

---

## 3. Security — SSRF and transport

### 3.1 Production path (`SecurePostingHttpFetcher`)

1. `validate_public_fetch_url` on each hop URL (scheme, host literal rules).
2. `resolve_public_socket_addr(host, port)` — DNS via `ToSocketAddrs`, reject disallowed IPs (IPv4 private/loopback/link-local/multicast; IPv6 loopback/ULA/link-local/multicast).
3. **Connect to the chosen `SocketAddr`**, not unresolved hostname (mitigates connect-time re-resolution TOCTOU for typical stacks).
4. Manual redirect loop (max `MAX_POSTING_REDIRECTS`); full chain stored in `PostingHttpResult.redirect_chain`.
5. Body read cap; optional **gzip** decode with same byte cap.
6. **No** `HTTP_PROXY` / alternative client paths in this fetcher (explicit omission).

### 3.2 Test transport (does not weaken production)

- `MockPostingHttpFetcher` (`#[cfg(test)]` in `posting_http.rs`) keyed by URL string.
- Ingest integration tests inject mock via `AppState::with_posting_fetcher`.

### 3.3 Security disposition

| Finding (audit) | Disposition | Notes |
|-----------------|-------------|-------|
| D-SSRF-1 Blocker | **Mitigated** (residual: exotic DNS race / first-public-IP choice) | Live HTTPS to `https://example.com/` exercised (`live_fetch_public_example`, ignored by default) |
| D-TXN-1 High | **Fixed** | Rollback test |
| D-FK-1 Medium | **Fixed** | Orphan snapshot test |
| D-XSS-1 Medium | **Reduced** | DOM text-only for list/meta |
| gzip bomb | **Bounded** | `decode_gzip_limited` + `MAX_POSTING_BODY_BYTES` |

---

## 4. Persistence and interpretation lifecycle

### 4.1 Review semantics (documented in `docs/MILESTONE_L_OPPORTUNITY_POSTINGS.md`)

- **Current** = `ORDER BY created_at DESC, id DESC`.
- Re-fetch → new snapshot + interpretation with **empty** `user_json`; prior rows unchanged.
- **`prior_reviewed`** when current is unreviewed but an older row has `reviewed_at`.
- PATCH with `user_overrides` clears existing `reviewed_at` unless `mark_reviewed: true`.

### 4.2 Verification tests

| Scenario | Test | Result |
|----------|------|--------|
| Transaction rollback | `save_posting_fetch_rolls_back_when_interpretation_blocked` | **Verified** |
| FK enforcement | `foreign_keys_reject_orphan_snapshot` | **Verified** |
| Second snapshot kept | `second_fetch_appends_snapshot_keeps_history` | **Verified** |
| Empty overrides on refetch | `refetch_starts_with_empty_user_overrides` | **Verified** |
| Review cleared on edit | `editing_reviewed_without_remark_clears_review` | **Verified** |
| E2E ingest + refetch + prior review | `opportunities::ingest_tests::refetch_does_not_copy_user_overrides` | **Verified** |

---

## 5. HTTP / API integration

| Command | Exit | Notes |
|---------|------|-------|
| `cargo test --workspace` | **0** | **81** unit/integration tests passed (post-correction) |
| `cargo build --workspace` | **0** | |
| `cargo test -p server live_fetch_public_example -- --ignored` | **0** | Public HTTPS fetch |

**New / notable tests (server):**

- `posting_http::tests::gzip_decompress_respects_limit`
- `posting_http::tests::fetch_policy_rejects_file_scheme`
- `posting_http::tests::fetch_policy_rejects_redirect_to_loopback_url`
- `opportunities::ingest_tests::ingest_via_mock_fetcher_persists_posting`
- `opportunities::ingest_tests::refetch_does_not_copy_user_overrides`

**Domain:**

- `posting_fetch::tests::resolve_public_socket_addr_rejects_loopback_literal`
- `posting_fetch::tests::resolve_public_socket_addr_rejects_private_literal`

---

## 6. Browser / UI

| Item | Status |
|------|--------|
| Opportunities tab manual walkthrough | **Unverified** (no browser automation in CI) |
| Source preview via `textContent` on `<pre>` | **Inspected** (unchanged safe pattern) |
| List/detail meta DOM | **Inspected** — switched to `textContent` / `createElement` |

**Manual procedure (owner):** start `jobdesk serve`, Opportunities tab → paste URL → Fetch & save → edit → Mark reviewed → reload tab → re-fetch; confirm prior review line and two snapshots.

---

## 7. Requirement-to-evidence matrix (acceptance packet)

| Requirement | Evidence | Result |
|-------------|----------|--------|
| Connection-level SSRF policy | `posting_http.rs` + domain `resolve_public_socket_addr` + tests + live example.com | **Verified** (with residual DNS notes) |
| Redirect hop validation | `fetch_with_policy` loop | **Inspected** + redirect URL tests |
| Atomic ingest | Transaction + rollback test | **Verified** |
| `foreign_keys=ON` | `Store::open` + orphan test | **Verified** |
| No app rows on ingest | `posting_ingest_does_not_create_application` + handler guard | **Verified** |
| Re-fetch history | `second_fetch_*` + ingest refetch test | **Verified** |
| No stale overrides on re-fetch | `refetch_starts_with_empty_user_overrides` | **Verified** |
| Review visibility | `prior_reviewed` + UI line | **Verified** (API/store); **Unverified** (browser) |
| Mock E2E ingest | `ingest_via_mock_fetcher_persists_posting` | **Verified** |
| Public site fetch | `live_fetch_public_example` | **Verified** (example.com only) |

---

## 8. Change inventory (correction increment)

**New:** `crates/server/src/posting_http.rs`, `docs/reviews/MILESTONE_L_CORRECTION_AND_ACCEPTANCE.md`
**Major edits:** `posting_fetch.rs`, `postings.rs`, `opportunities.rs`, `store/lib.rs`, `server/lib.rs`, `86-opportunities.js`, `MILESTONE_L_OPPORTUNITY_POSTINGS.md`
**Deps:** removed workspace `ureq`; server adds `rustls`, `webpki-roots`, `flate2`, `url`

---

## 9. Outstanding issues

| Issue | Severity | Impact |
|-------|----------|--------|
| First resolved public IP only (multi-A record flip) | Low | Rare CDN mis-route; could pin all resolved addrs in future |
| No automated multi-hop redirect integration test | Low | Policy unit tests cover blocked targets |
| Visible-text fallback boilerplate | Low | Quality of `description_plain` |
| Concurrent duplicate ingest | Low | SQLite UNIQUE + transactions; possible user-visible race |
| Real job board URLs (LinkedIn, etc.) | N/A | Often blocked / login-walled — not supported claims |
| Browser E2E | Medium | Owner manual check still needed |

---

## 10. Reproducible commands

```bash
# Full verification
cargo test --workspace
cargo build --workspace

# Live network smoke (optional)
cargo test -p server live_fetch_public_example -- --ignored --nocapture

# Focused Milestone L
cargo test -p domain posting_fetch::
cargo test -p store postings::
cargo test -p server opportunities:: posting_http::
```

---

## 11. Acceptance recommendation

**Conditional acceptance:** Blockers from the implementation audit are addressed with automated evidence for persistence, lifecycle, mock ingest E2E, and connection-pinned HTTPS to a public host. **Browser demonstration remains unverified** in this environment; **live job-board URLs** were not claimed.

**Do not commit** until the owner reviews this evidence (per assignment).
