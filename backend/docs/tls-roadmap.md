# TLS Roadmap

## Status

| Protocol | Status | Notes |
|----------|--------|-------|
| TCP (TLS 1.2/1.3) | ✅ Done | `tokio-rustls` wraps `TcpStream`; enabled by default |
| WebSocket (WSS) | 🔜 TODO | See §2 below |
| HTTP/HTTPS | 🔜 TODO | See §3 below |
| UDP / DTLS | 🔜 TODO | See §4 below |
| QUIC | ✅ Done | Uses `quinn` (always encrypted) |

---

## 1. TCP TLS (current)

TCP TLS is implemented in `crates/socket-server/src/tcp.rs`.

- **Library**: `tokio-rustls`
- **Flow**: `TcpListener::accept()` → `TlsAcceptor::accept(stream)` → split into `(reader, writer)` trait objects → `connection_task`
- **Config**: `[tls] enabled = true` in `config/default.toml`
- **Dev certs**: run `backend/scripts/generate-certs.sh` (Linux/macOS) or `backend/scripts/generate-certs.ps1` (Windows)
- **Graceful fallback**: if cert files are missing at startup, the server logs a warning and continues without TLS rather than crashing

---

## 2. WebSocket (WSS) — TODO

### Approach

Switch the WebSocket server from bare `tokio-tungstenite` to **`axum-server`** with `rustls`, which handles the TLS layer before upgrading to WebSocket.

```rust
// Pseudocode
use axum_server::tls_rustls::RustlsConfig;

let tls_config = RustlsConfig::from_pem_file("certs/server.crt", "certs/server.key").await?;
axum_server::bind_rustls(addr, tls_config)
    .serve(app.into_make_service())
    .await?;
```

### Key steps

1. Replace `tokio-tungstenite` direct bind in `crates/socket-server/src/ws.rs` with an `axum` router that upgrades to WebSocket.
2. Add `axum-server` + `rustls` to `socket-server/Cargo.toml`.
3. Pass the same `TlsConfig` struct from `server-config` to derive `RustlsConfig`.
4. Reuse the existing `TlsConfig.cert_path` / `TlsConfig.key_path` fields.
5. SDK clients connect to `wss://host:9002` when TLS is enabled.

### Config impact

No new config fields needed — `[tls] enabled = true` covers WSS once implemented.

---

## 3. HTTPS — TODO

### Approach

The HTTP server in `crates/socket-server/src/http.rs` already uses `axum`. Adding HTTPS follows the same `axum-server` pattern as WSS above.

```rust
axum_server::bind_rustls(addr, tls_config)
    .serve(router.into_make_service())
    .await?;
```

### Key steps

1. Add `axum-server` dependency to `socket-server/Cargo.toml` (shared with WSS work).
2. Branch on `config.tls.enabled` in `http.rs` start — use `bind_rustls` when true, `bind` when false.
3. Admin API (port 9100) is a separate `axum` server in `crates/admin-api`; apply the same pattern there.

### Config impact

Consider adding `[tls] http_redirect = true` to auto-redirect HTTP → HTTPS (optional, out of scope for now).

---

## 4. UDP / DTLS — TODO

UDP does not have a standard async TLS library with the same ergonomics as `tokio-rustls`. Options:

### Option A: Application-layer encryption (simpler, non-standard)

Encrypt each UDP datagram with **ChaCha20-Poly1305** (via the `chacha20poly1305` crate) using a session key negotiated over the already-TLS-encrypted TCP control connection.

- Pro: no new networking library; session key is tied to JWT auth
- Con: not DTLS-compliant; custom protocol that third-party clients must implement

### Option B: DTLS 1.3 via `webrtc-dtls` or `openssl`

Use the `webrtc-dtls` crate (part of the `webrtc-rs` ecosystem) to wrap `UdpSocket` in a DTLS session.

- Pro: standards-compliant; compatible with DTLS clients (game engines, embedded devices)
- Con: `webrtc-dtls` is less mature than `rustls`; async integration requires care

### Recommendation

Implement **Option A** first (lower risk, faster), then evaluate DTLS if interoperability with third-party clients becomes a requirement.

### Sketch (Option A)

```rust
// On auth success, both sides derive session_key = HKDF(jwt_secret, session_id)
// UDP sender:
let nonce = random_nonce();
let ciphertext = chacha20poly1305::seal(&key, &nonce, &plaintext)?;
let frame = [&nonce[..], &ciphertext[..]].concat();
socket.send(&frame)?;

// UDP receiver:
let (nonce, ciphertext) = frame.split_at(NONCE_LEN);
let plaintext = chacha20poly1305::open(&key, nonce, ciphertext)?;
```

---

## 5. mTLS (mutual TLS) — TODO

The `TlsConfig` struct already has `mtls: bool` and `ca_path: Option<String>`. To enable client certificate verification:

1. In `crates/socket-server/src/tls.rs::load_tls_config()`, when `config.mtls == true`:
   - Load the CA cert from `ca_path`
   - Set `ClientAuth::RequireAny(ca_roots)` on `rustls::ServerConfig`
2. After the TLS handshake, extract the peer certificate from the TLS session and populate `ConnectionInfo.client_cert`.
3. `AuthHandler` can then skip JWT validation for mTLS connections (the cert is the credential).

---

## References

- `tokio-rustls`: https://docs.rs/tokio-rustls
- `axum-server` TLS: https://docs.rs/axum-server/latest/axum_server/tls_rustls/index.html
- `webrtc-dtls`: https://docs.rs/webrtc-dtls
- `chacha20poly1305`: https://docs.rs/chacha20poly1305
- DTLS 1.3 RFC 9147: https://www.rfc-editor.org/rfc/rfc9147
