# Draox Server — Windows Deployment

## Prerequisites

1. **Rust toolchain** — [rustup.rs](https://rustup.rs/)
2. **WiX Toolset v3.14+** — [wixtoolset.org](https://wixtoolset.org/docs/wix3/)
3. **cargo-wix** — `cargo install cargo-wix`
4. **WiX extensions** (for Firewall/Util features):
   - `WixFirewallExtension.dll`
   - `WixUtilExtension.dll`
   (Included with WiX Toolset installation)

## Build MSI Installer

```powershell
# 1. Build release binary
cargo build --release --bin=draox-server

# 2. Build MSI
cargo wix --no-build --nocapture `
    -o target/wix/draox-server-0.1.0-x86_64.msi

# Or build + package in one step
cargo wix
```

Output: `target/wix/draox-server-{version}-x86_64.msi`

## Install (MSI)

Double-click the `.msi` file or run:

```powershell
msiexec /i draox-server-0.1.0-x86_64.msi
```

The installer provides three optional features:
- **Draox Server Core** (required) — binary, config, scripts
- **Windows Service** — register as auto-start service with failure recovery
- **Firewall Rules** — open ports 9000-9003, 9090, 9100

### Silent Install

```powershell
msiexec /i draox-server-0.1.0-x86_64.msi /quiet /norestart
```

### Install Core Only (no service, no firewall)

```powershell
msiexec /i draox-server-0.1.0-x86_64.msi ADDLOCAL=FeatureCore /quiet
```

## Install (Manual — Without MSI)

If you prefer not to use the MSI installer:

```powershell
# 1. Build
cargo build --release --bin=draox-server

# 2. Copy binary
mkdir "C:\Program Files\DraoxServer\bin"
copy target\release\draox-server.exe "C:\Program Files\DraoxServer\bin\"

# 3. Copy config
mkdir "C:\ProgramData\DraoxServer\config"
copy deploy\windows\config\default.toml "C:\ProgramData\DraoxServer\config\"

# 4. Create data directories
mkdir "C:\ProgramData\DraoxServer\data"
mkdir "C:\ProgramData\DraoxServer\logs"
mkdir "C:\ProgramData\DraoxServer\plugins"

# 5. Register as Windows Service (optional, requires admin)
.\deploy\windows\scripts\install-service.ps1

# 6. Open firewall ports (optional, requires admin)
.\deploy\windows\scripts\manage-firewall.ps1 -Action Add
```

## Uninstall

### Via MSI

```powershell
# GUI
msiexec /x draox-server-0.1.0-x86_64.msi

# Silent
msiexec /x draox-server-0.1.0-x86_64.msi /quiet
```

### Via Add/Remove Programs

Settings > Apps > Draox Server > Uninstall

### Manual Uninstall

```powershell
.\deploy\windows\scripts\uninstall-service.ps1        # Remove service
.\deploy\windows\scripts\manage-firewall.ps1 -Action Remove  # Remove firewall rules
.\deploy\windows\scripts\uninstall-service.ps1 -Purge # Remove all data
```

## Directory Structure

```
C:\Program Files\DraoxServer\
├── bin\
│   └── draox-server.exe        # Server binary
├── config\
│   └── default.toml            # Reference config (read-only)
└── scripts\
    ├── install-service.ps1     # Service installer
    ├── uninstall-service.ps1   # Service uninstaller
    └── manage-firewall.ps1     # Firewall manager

C:\ProgramData\DraoxServer\
├── config\
│   └── default.toml            # Working config (editable)
├── data\
│   └── draox.db                # SQLite database
├── logs\
│   └── draox.log               # Log files
├── plugins\                    # Plugin directory
└── certs\                      # TLS certificates
```

## Service Management

```powershell
Start-Service DraoxServer               # Start
Stop-Service DraoxServer                # Stop
Restart-Service DraoxServer             # Restart
Get-Service DraoxServer                 # Status

# View logs
Get-EventLog -LogName Application -Source DraoxServer -Newest 50
```

## Ports

| Port | Protocol | Service              |
|------|----------|----------------------|
| 9000 | TCP      | Socket protocol      |
| 9001 | UDP      | Datagram protocol    |
| 9002 | TCP      | WebSocket            |
| 9003 | TCP      | HTTP/HTTPS           |
| 9090 | TCP      | Prometheus metrics   |
| 9100 | TCP      | Admin API (localhost)|

## Configuration

Edit the working config:
```
C:\ProgramData\DraoxServer\config\default.toml
```

**Important**: Set `DRAOX_ADMIN_JWT_SECRET` environment variable before starting:
```powershell
# System-wide
[Environment]::SetEnvironmentVariable("DRAOX_ADMIN_JWT_SECRET", "your-secret-here", "Machine")

# Or via service registry (set during install-service.ps1)
```
