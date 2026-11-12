# Milestone L — Transport Simplification and Operational Acceptance

**Date:** 2026-11-11 (UTC)
**Prior work:** [`MILESTONE_L_CORRECTION_AND_ACCEPTANCE.md`](./MILESTONE_L_CORRECTION_AND_ACCEPTANCE.md)

---

## 1. Why `reqwest` instead of custom transport

The prior `SecurePostingHttpFetcher` implemented raw TCP, TLS (`rustls`), HTTP/1.1 framing, chunked bodies, and gzip — hundreds of lines that duplicate mature libraries and increase parsing/security risk.

**`reqwest` 0.12** (workspace, `blocking` + `rustls-tls` + `gzip`) provides:

- Standard **TLS certificate and hostname verification** (no custom trust bypass).
- **`Policy::none()`** — automatic redirects disabled; JobDesk follows `Location` manually after policy checks.
- **`no_proxy()`** — no `HTTP_PROXY` fallback.
- **`resolve(domain, SocketAddr)`** — connect to the **vetted** address from `domain::resolve_public_socket_addr`.
- **Timeouts** (`timeout`, `connect_timeout`) and **gzip** decompression with post-decompress size cap via `read_body_limited`.

**Removed from `server` crate:** direct `rustls`, `webpki-roots`, `flate2`, and all custom protocol code (~350 lines).

**Remaining custom code (justified):**

| Piece | Role |
|-------|------|
| `fetch_with_policy` | URL validation, redirect loop, content-type gate, body cap |
| `pinned_client` | Policy + `reqwest::ClientBuilder` per hop |
| `resolve_public_socket_addr` (`domain`) | JobDesk-owned SSRF resolution |
| `MockPostingHttpFetcher` | Test injection only (`#[cfg(test)]`) |

---

## 2. SSRF policy enforcement

**JobDesk owns:** `validate_public_fetch_url`, `resolve_public_socket_addr`, redirect target validation (`validate_public_fetch_url` on each hop), redirect count, content-type, body size.

**`reqwest` owns:** TLS, HTTP encoding, decompression, socket I/O — always via a client built with:

```text
redirect(none) · no_proxy() · resolve(host, vetted_addr)
```

Each redirect hop builds a **new** pinned client for the next host/port.

---

## 3. Automated test evidence

| Command | Exit |
|---------|------|
| `cargo test --workspace` | **0** |
| `cargo test -p server posting_http` | **0** (12 run, 3 `#[ignore]` live) |
| `cargo test -p server live_fetch_job_posting_json_ld -- --ignored` | **0** |

**Deterministic transport/policy tests** (`posting_http::tests`):

- `fetch_policy_rejects_file_scheme`
- `loopback_ingest_url_rejected_before_reqwest`
- `next_redirect_rejects_private_location` / `multi_hop_redirect_logic`
- `connect_timeout_to_unreachable_private`
- `reqwest_client_disallows_proxy_and_redirect` (client builds with pinned resolve)

**Ignored live checks (run explicitly):**

- `live_fetch_public_example` — `https://example.com/`
- `live_fetch_job_posting_json_ld` — Google careers job URL (HTTP 200, HTML job page)
- `live_tls_bad_certificate_rejected` — `https://self-signed.badssl.com/`

**Not automated without live loopback redirect server** (policy would block URL): full multi-hop **HTTP** redirect through `reqwest`; **logic** covered by `multi_hop_redirect_logic` on URL join + validation.

---

## 4. Operational acceptance procedure and observed results

### 4.1 Setup (documented JobDesk commands)

```bash
cd /home/voyager/Projects/resume
export OP_WS=target/milestone-l-op-accept
cargo run -p jobdesk -- --workspace "$OP_WS" import
cargo run -p jobdesk -- serve --workspace "$OP_WS" --host 127.0.0.1 --port 3847
```

Isolated DB used for evidence:
`/home/voyager/Projects/resume/target/milestone-l-op-accept/jobdesk.db`

### 4.2 Live job URL (genuine posting page)

**URL tested:**
`https://www.google.com/about/careers/applications/jobs/results/317042454704059940-software-engineer-iii-geo-maps/`

(Google Careers — public listing; not a generic `example.com` page.)

### 4.3 Step matrix

| Step | Expected | Actual (evidence type) |
|------|----------|-------------------------|
| 1 Start JobDesk | UI on `http://127.0.0.1:<port>` | **Verified (API)** — `cargo run … serve --port 3853`; log `jobdesk UI: http://127.0.0.1:3853` |
| 2 Open Opportunities | Tab visible | **Unverified (human UI)** — no browser automation |
| 3 Paste URL | — | **Unverified (UI)** — URL used in API below |
| 4 Fetch & save | HTTP 201, posting row | **Verified (API)** — `POST /api/opportunities/ingest` → **HTTP:201**; posting id `ceba09ac-4a25-4550-9edc-3f6983a712f2` |
| 5 Observe extraction | Title/employer if JSON-LD | **Partial** — page returned **200** HTML; `json_extract` title/employer **empty** (minimal JSON-LD on this page; description/fallback may still populate in full JSON response) |
| 6 Original source | Snapshot HTML retrievable | **Verified (API)** — `GET /api/opportunities/snapshots/{id}/source` returns `text/plain` body (truncated in console) |
| 7 Edit field | PATCH merges overrides | **Verified (API)** — `PATCH …/interpretation` with `user_overrides.title` → **HTTP 200** |
| 8 Mark reviewed | `reviewed_at` set | **Verified (API)** — `PATCH` `mark_reviewed:true` → **HTTP 200** |
| 9 Restart JobDesk | Process stop/start | **Verified (API)** — server killed and restarted on port 3854 |
| 10 Persistence after restart | Edits still present | **Verified (API)** — `GET /api/opportunities/{id}` returns merged title containing saved override (grep on response) |
| 11 Re-fetch same URL | Second snapshot | **Verified (API)** — second `POST ingest` → **HTTP 201** |
| 12 Two snapshots | COUNT = 2 | **Verified (SQL)** — `SELECT COUNT(*) … posting_snapshots` → **2** after second ingest |
| 13 New interpretation unreviewed | `reviewed_at` NULL on latest | **Verified (SQL)** — latest interp `reviewed_at IS NULL` = 1; response includes `prior_reviewed` |
| 14 No application change | `applications` count unchanged | **Verified (SQL)** — count **0** before/after |

### 4.4 Database queries (reproduce)

```bash
DB=/home/voyager/Projects/resume/target/milestone-l-op-accept/jobdesk.db
PID=ceba09ac-4a25-4550-9edc-3f6983a712f2   # from ingest response

sqlite3 "$DB" "SELECT COUNT(*) FROM applications;"
sqlite3 "$DB" "SELECT COUNT(*) FROM posting_snapshots WHERE posting_id='$PID';"
sqlite3 "$DB" "SELECT id, reviewed_at FROM posting_interpretations WHERE posting_id='$PID' ORDER BY created_at DESC, id DESC;"
```

**Inaccessible posting (graceful failure):**

```bash
curl -X POST http://127.0.0.1:3854/api/opportunities/ingest \
  -H 'Content-Type: application/json' -d '{"url":"http://127.0.0.1/"}'
# Expected: HTTP 400, SSRF / disallowed host
```

---

## 5. Acceptance class summary

| Class | Status |
|-------|--------|
| Automated unit/integration tests | **Verified** |
| Programmatic API operational demo | **Verified** (steps 4, 6–14) |
| Browser automation | **Not available** |
| Human-operated UI (steps 2–3, Opportunities tab click-through) | **Unverified** |

**Owner UI checklist (same as API, in browser):**

1. `cargo run -p jobdesk -- --workspace target/milestone-l-op-accept serve --open`
2. **Opportunities** tab → paste Google careers URL above → **Fetch & save**
3. Confirm list entry, detail fields, source preview (`<pre>` text), save edit, **Mark reviewed**
4. Restart serve, reopen posting, re-fetch URL, confirm **prior review** line and **two** snapshots in API detail (or future UI surfacing both snapshots).

---

## 6. Remaining limitations

| Issue | Severity | Notes |
|-------|----------|-------|
| Sparse JSON-LD on some boards (incl. tested Google URL) | Low | Extraction may rely on visible-text fallback; owner may see empty title until edit |
| UI acceptance not machine-verified | Medium | API path fully exercised |
| Multi-hop redirect not tested end-to-end over TCP | Low | Logic + loopback rejection tested |
| LinkedIn / login-walled ATS URLs | Expected unsupported | Use public career site URLs |

---

## 7. Reproducible commands

```bash
cargo test --workspace
cargo test -p server posting_http
cargo test -p server live_fetch_job_posting_json_ld -- --ignored --nocapture
cargo run -p jobdesk -- --workspace target/milestone-l-op-accept import
cargo run -p jobdesk -- --workspace target/milestone-l-op-accept serve --port 3847
```

**No commit or push** per assignment.
