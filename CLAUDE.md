# Draox Server

## Project Overview
A Rust-based plugin-powered multi-protocol socket server that manages client connections (TCP, UDP, WebSocket, HTTP/HTTPS) with a VS Code-inspired plugin architecture. Supports server-authoritative multi-connections, WASM-sandboxed third-party plugins, and a marketplace for plugin distribution. Built as a modular Cargo workspace with 14 specialized crates.

## Architecture
- **Workspace**: Cargo workspace with crates under `crates/`
- **Runtime**: Tokio async (multi-threaded)
- **Plugin Model**: Hybrid — Built-in (Rust crates) + External (WASM via wasmtime)
- **Networking**: TCP, UDP, WebSocket, HTTP/HTTPS (via `socket-server` crate)
- **Storage**: PostgreSQL, MySQL, SQLite, MongoDB (via `data-store` crate)
- **Cache**: Redis + in-memory (via `cache-layer` crate)
- **Admin API**: REST API + WebSocket for dashboard + plugin management (via `admin-api` crate)
- **Marketplace**: Plugin registry, download, install, update (via `plugin-host` crate)

## Layer Model
```
Layer 6: Application       main.rs (server binary)
Layer 5: API               admin-api (REST + WS + plugin management + marketplace)
Layer 4: Plugins           plugin-clans, plugin-messaging, [external WASM plugins...]
Layer 3: Plugin Runtime    plugin-host (lifecycle, WASM sandbox, marketplace client)
Layer 2: Services          connection-manager, data-store, cache-layer, activity-log, billing
Layer 1: Networking        socket-server (TCP, UDP, WS, HTTP/HTTPS), traffic-guard
Layer 0: Foundation        server-core, server-config, plugin-sdk
```

## Crates
| Crate | Path | Purpose |
|-------|------|---------|
| `server-core` | `crates/server-core/` | Core types, traits, errors |
| `server-config` | `crates/server-config/` | Config loading, hot-reload, TOML |
| `plugin-sdk` | `crates/plugin-sdk/` | Plugin developer API, types, macros |
| `socket-server` | `crates/socket-server/` | Raw networking: TCP, UDP, WS, HTTP/HTTPS |
| `traffic-guard` | `crates/traffic-guard/` | Anti-spam, DDoS protection, rate limiting, IP reputation |
| `connection-manager` | `crates/connection-manager/` | Multi-connection pool, server-authoritative sessions |
| `data-store` | `crates/data-store/` | SQL + NoSQL database storage |
| `cache-layer` | `crates/cache-layer/` | Redis + in-memory caching |
| `activity-log` | `crates/activity-log/` | Connection logging, data metrics |
| `billing` | `crates/billing/` | Usage billing + subscription plans |
| `plugin-host` | `crates/plugin-host/` | Plugin lifecycle, WASM sandbox, marketplace client |
| `admin-api` | `crates/admin-api/` | REST API (~72 endpoints) + WS (5 streams) + Swagger UI |
| `plugin-clans` | `crates/plugin-clans/` | Built-in plugin: Clans/Groups management |
| `plugin-messaging` | `crates/plugin-messaging/` | Built-in plugin: Instant messaging |

## Key Commands
```bash
cargo build                    # Build all crates
cargo test                     # Run all tests
cargo run                      # Start the server
cargo run -- --config config/default.toml  # Start with specific config
```

## Deployment
- **Linux (systemd)**: `deploy/linux/install.sh` — automated installer with systemd, firewall, logrotate
- **Linux (uninstall)**: `deploy/linux/uninstall.sh` — clean removal with `--purge` option
- **Docker**: `docker compose up -d` — multi-stage build, health check, resource limits
- **Debian/Ubuntu**: `cargo deb -p draox-server` — produces `.deb` package with systemd integration
- **Windows (MSI)**: `cargo wix` — produces `.msi` installer with WiX Toolset (features: Core, Windows Service, Firewall Rules)
- **Windows (manual)**: `deploy/windows/scripts/install-service.ps1` — register Windows Service + firewall rules
- **Windows (uninstall)**: `deploy/windows/scripts/uninstall-service.ps1` — remove service + optional data purge
- **Ports**: TCP=9000, UDP=9001, WS=9002, HTTP=9003, Metrics=9090, Admin=9100

## Configuration
- Default config: `config/default.toml`
- Env var prefix: `DRAOX_`
- Config hot-reload supported via file watcher
- Plugin configs: `[plugins.clans]`, `[plugins.messaging]`
- Marketplace config: `[marketplace]`
- Linux env file: `/etc/draox-server/draox-server.env`
- Windows config: `C:\ProgramData\DraoxServer\config\default.toml`

## Design Documents
- English: `docs/design_en.html`
- Vietnamese: `docs/design_vi.html`

## Conventions
- Error handling: `thiserror` for library errors, `anyhow` for application errors
- Async: All I/O operations must be async (tokio)
- Logging: Use `tracing` crate (not `log` or `println!`)
- Serialization: `serde` + `serde_json` for all protocol messages
- Config: `serde` + `toml` for configuration
- Naming: snake_case for files/modules, CamelCase for types
- Commit messages: `[type] title` format (feat, fix, docs, refactor, test, chore)

## Important Notes
- Plugin manifest: `plugin.toml` (TOML format, reverse-domain ID)
- Plugin package: `.dxp` (Draox Plugin) — zip archive with manifest + WASM + assets
- Plugin lifecycle: activate → enable ↔ disable → deactivate (VS Code-inspired)
- Plugin loading: Built-in (Rust crate, compile-time) + WASM (wasmtime, runtime)
- Plugin signing: Ed25519 signature verification for marketplace plugins
- `traffic-guard` sits between socket-server and connection-manager in the pipeline
- `socket-server` has zero dependencies on plugin crates (protocol-agnostic)
- `plugin-sdk` provides PluginContext with handles to all server services
- Connection sessions: server-authoritative, multi-connection per client
- Connection roles: primary, notification, control, streaming
- Admin API runs on a separate port (default 9100) with JWT/API key auth
- Admin API provides OpenAPI/Swagger UI at `/swagger-ui`
- Database: sqlx for SQL, mongodb for NoSQL
- Cache: fred for Redis, moka for in-memory
- WASM plugins: sandboxed via wasmtime with memory/CPU limits
- Marketplace: plugin registry at marketplace.draox-server.io
