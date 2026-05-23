#!/usr/bin/env bash
# ================================================
# Draox Server — Linux Install Script
# Usage: sudo ./install.sh [OPTIONS]
#
# Options:
#   --prefix DIR    Install prefix (default: /opt/draox-server)
#   --config DIR    Config directory (default: /etc/draox-server)
#   --data DIR      Data directory (default: /var/lib/draox-server)
#   --log DIR       Log directory (default: /var/log/draox-server)
#   --user USER     Service user (default: draox)
#   --binary PATH   Path to draox-server binary (default: auto-detect)
#   --no-service    Skip systemd service installation
#   --no-firewall   Skip firewall rules
#   --unattended    Non-interactive mode
# ================================================

set -euo pipefail

# ── Colors ──
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }

# ── Defaults ──
PREFIX="/opt/draox-server"
CONFIG_DIR="/etc/draox-server"
DATA_DIR="/var/lib/draox-server"
LOG_DIR="/var/log/draox-server"
SERVICE_USER="draox"
BINARY_PATH=""
INSTALL_SERVICE=true
INSTALL_FIREWALL=true
UNATTENDED=false

# ── Parse arguments ──
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)       PREFIX="$2"; shift 2 ;;
        --config)       CONFIG_DIR="$2"; shift 2 ;;
        --data)         DATA_DIR="$2"; shift 2 ;;
        --log)          LOG_DIR="$2"; shift 2 ;;
        --user)         SERVICE_USER="$2"; shift 2 ;;
        --binary)       BINARY_PATH="$2"; shift 2 ;;
        --no-service)   INSTALL_SERVICE=false; shift ;;
        --no-firewall)  INSTALL_FIREWALL=false; shift ;;
        --unattended)   UNATTENDED=true; shift ;;
        -h|--help)
            head -15 "$0" | tail -13
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ── Pre-checks ──
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root (use sudo)"
    exit 1
fi

# Auto-detect binary
if [[ -z "$BINARY_PATH" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

    if [[ -f "$REPO_ROOT/target/release/draox-server" ]]; then
        BINARY_PATH="$REPO_ROOT/target/release/draox-server"
    elif [[ -f "$REPO_ROOT/target/debug/draox-server" ]]; then
        BINARY_PATH="$REPO_ROOT/target/debug/draox-server"
        log_warn "Using debug build — consider building with: cargo build --release"
    else
        log_error "Binary not found. Build first: cargo build --release --bin=draox-server"
        exit 1
    fi
fi

if [[ ! -f "$BINARY_PATH" ]]; then
    log_error "Binary not found: $BINARY_PATH"
    exit 1
fi

# ── Confirmation ──
echo ""
echo "========================================="
echo "  Draox Server — Linux Installer"
echo "========================================="
echo ""
echo "  Binary:     $BINARY_PATH"
echo "  Install to: $PREFIX/bin/"
echo "  Config:     $CONFIG_DIR/"
echo "  Data:       $DATA_DIR/"
echo "  Logs:       $LOG_DIR/"
echo "  User:       $SERVICE_USER"
echo "  Service:    $INSTALL_SERVICE"
echo "  Firewall:   $INSTALL_FIREWALL"
echo ""

if [[ "$UNATTENDED" != true ]]; then
    read -rp "Proceed with installation? [Y/n] " confirm
    if [[ "$confirm" =~ ^[Nn] ]]; then
        log_info "Installation cancelled."
        exit 0
    fi
fi

# ── Step 1: Create user ──
log_step "Creating service user: $SERVICE_USER"
if id "$SERVICE_USER" &>/dev/null; then
    log_info "User '$SERVICE_USER' already exists"
else
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
    log_info "Created system user: $SERVICE_USER"
fi

# ── Step 2: Create directories ──
log_step "Creating directories"
mkdir -p "$PREFIX/bin"
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$PREFIX/plugins"

# ── Step 3: Install binary ──
log_step "Installing binary"
install -m 755 "$BINARY_PATH" "$PREFIX/bin/draox-server"
log_info "Installed: $PREFIX/bin/draox-server"

# ── Step 4: Install config ──
log_step "Installing configuration"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    # Fresh install — copy default config with Linux paths
    sed \
        -e "s|\"plugins\"|\"$PREFIX/plugins\"|" \
        -e "s|sqlite://data/draox.db|sqlite://$DATA_DIR/draox.db|" \
        -e "s|\"certs/|\"$CONFIG_DIR/certs/|" \
        -e "s|\"logs/|\"$LOG_DIR/|" \
        "$REPO_ROOT/config/default.toml" > "$CONFIG_DIR/config.toml"
    log_info "Installed default config: $CONFIG_DIR/config.toml"
else
    log_warn "Config already exists, skipping (backup old config if upgrading)"
fi

# Install environment file
if [[ ! -f "$CONFIG_DIR/draox-server.env" ]]; then
    if [[ -f "$SCRIPT_DIR/draox-server.env" ]]; then
        install -m 600 "$SCRIPT_DIR/draox-server.env" "$CONFIG_DIR/draox-server.env"
    else
        cat > "$CONFIG_DIR/draox-server.env" << 'ENVEOF'
RUST_LOG=info,draox_server=info
RUST_BACKTRACE=0
DRAOX_ADMIN_JWT_SECRET=CHANGE_ME_TO_A_SECURE_RANDOM_STRING
ENVEOF
    fi
    log_info "Installed env file: $CONFIG_DIR/draox-server.env"
    log_warn "IMPORTANT: Edit $CONFIG_DIR/draox-server.env and set DRAOX_ADMIN_JWT_SECRET"
fi

# ── Step 5: Set permissions ──
log_step "Setting permissions"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$PREFIX/plugins"
chown root:"$SERVICE_USER" "$CONFIG_DIR/config.toml"
chmod 640 "$CONFIG_DIR/config.toml"
chmod 600 "$CONFIG_DIR/draox-server.env"
log_info "Permissions set"

# ── Step 6: Install systemd service ──
if [[ "$INSTALL_SERVICE" == true ]]; then
    log_step "Installing systemd service"

    # Generate service file with actual paths
    cat > /etc/systemd/system/draox-server.service << SVCEOF
[Unit]
Description=Draox Server — Plugin-powered multi-protocol socket server
Documentation=https://github.com/draox/draox-server
After=network-online.target
Wants=network-online.target
StartLimitIntervalSec=60
StartLimitBurst=5

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$PREFIX
ExecStart=$PREFIX/bin/draox-server --config $CONFIG_DIR/config.toml
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=5
TimeoutStartSec=30
TimeoutStopSec=30

EnvironmentFile=-$CONFIG_DIR/draox-server.env

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
RestrictNamespaces=yes
LockPersonality=yes
MemoryDenyWriteExecute=yes
RestrictRealtime=yes
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
SystemCallArchitectures=native

ReadWritePaths=$DATA_DIR
ReadWritePaths=$LOG_DIR

LimitNOFILE=65536
LimitNPROC=4096

StandardOutput=journal
StandardError=journal
SyslogIdentifier=draox-server

[Install]
WantedBy=multi-user.target
SVCEOF

    systemctl daemon-reload
    systemctl enable draox-server
    log_info "Systemd service installed and enabled"
fi

# ── Step 7: Firewall rules ──
if [[ "$INSTALL_FIREWALL" == true ]]; then
    log_step "Configuring firewall"

    if command -v ufw &>/dev/null; then
        ufw allow 9000/tcp comment "Draox TCP"
        ufw allow 9001/udp comment "Draox UDP"
        ufw allow 9002/tcp comment "Draox WebSocket"
        ufw allow 9003/tcp comment "Draox HTTP"
        ufw allow from 127.0.0.0/8 to any port 9100 proto tcp comment "Draox Admin (localhost only)"
        log_info "UFW rules added"
    elif command -v firewall-cmd &>/dev/null; then
        firewall-cmd --permanent --add-port=9000/tcp
        firewall-cmd --permanent --add-port=9001/udp
        firewall-cmd --permanent --add-port=9002/tcp
        firewall-cmd --permanent --add-port=9003/tcp
        firewall-cmd --permanent --add-rich-rule='rule family="ipv4" source address="127.0.0.0/8" port port="9100" protocol="tcp" accept'
        firewall-cmd --reload
        log_info "firewalld rules added"
    else
        log_warn "No supported firewall found (ufw/firewalld). Open ports manually:"
        log_warn "  TCP: 9000, 9002, 9003  |  UDP: 9001  |  Admin: 9100 (localhost)"
    fi
fi

# ── Step 8: Install logrotate ──
log_step "Installing logrotate config"
cat > /etc/logrotate.d/draox-server << LREOF
$LOG_DIR/*.log {
    daily
    missingok
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 $SERVICE_USER $SERVICE_USER
    sharedscripts
    postrotate
        systemctl reload draox-server 2>/dev/null || true
    endscript
}
LREOF
log_info "Logrotate configured: 14 days retention"

# ── Done ──
echo ""
echo "========================================="
echo "  Installation Complete!"
echo "========================================="
echo ""
echo "  Binary:   $PREFIX/bin/draox-server"
echo "  Config:   $CONFIG_DIR/config.toml"
echo "  Env:      $CONFIG_DIR/draox-server.env"
echo "  Data:     $DATA_DIR/"
echo "  Logs:     $LOG_DIR/"
echo "  Plugins:  $PREFIX/plugins/"
echo ""
echo "  IMPORTANT:"
echo "    1. Edit $CONFIG_DIR/draox-server.env"
echo "       Set DRAOX_ADMIN_JWT_SECRET to a secure value"
echo ""
echo "    2. Review config: $CONFIG_DIR/config.toml"
echo ""
echo "  Commands:"
echo "    sudo systemctl start draox-server     # Start server"
echo "    sudo systemctl status draox-server     # Check status"
echo "    sudo journalctl -u draox-server -f     # View logs"
echo "    sudo systemctl restart draox-server    # Restart"
echo "    sudo systemctl stop draox-server       # Stop"
echo ""
