#!/usr/bin/env bash
# ================================================
# Draox Server — Linux Uninstall Script
# Usage: sudo ./uninstall.sh [OPTIONS]
#
# Options:
#   --prefix DIR    Install prefix (default: /opt/draox-server)
#   --config DIR    Config directory (default: /etc/draox-server)
#   --data DIR      Data directory (default: /var/lib/draox-server)
#   --log DIR       Log directory (default: /var/log/draox-server)
#   --user USER     Service user (default: draox)
#   --keep-data     Keep data directory (database, plugins)
#   --keep-config   Keep config directory
#   --keep-logs     Keep log directory
#   --purge         Remove everything including data, config, logs, user
#   --unattended    Non-interactive mode
# ================================================

set -euo pipefail

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
KEEP_DATA=true
KEEP_CONFIG=true
KEEP_LOGS=false
PURGE=false
UNATTENDED=false

# ── Parse arguments ──
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)       PREFIX="$2"; shift 2 ;;
        --config)       CONFIG_DIR="$2"; shift 2 ;;
        --data)         DATA_DIR="$2"; shift 2 ;;
        --log)          LOG_DIR="$2"; shift 2 ;;
        --user)         SERVICE_USER="$2"; shift 2 ;;
        --keep-data)    KEEP_DATA=true; shift ;;
        --keep-config)  KEEP_CONFIG=true; shift ;;
        --keep-logs)    KEEP_LOGS=true; shift ;;
        --purge)        PURGE=true; KEEP_DATA=false; KEEP_CONFIG=false; KEEP_LOGS=false; shift ;;
        --unattended)   UNATTENDED=true; shift ;;
        -h|--help)
            head -17 "$0" | tail -15
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root (use sudo)"
    exit 1
fi

echo ""
echo "========================================="
echo "  Draox Server — Uninstaller"
echo "========================================="
echo ""
echo "  Prefix:       $PREFIX"
echo "  Config:       $CONFIG_DIR ($([ "$KEEP_CONFIG" = true ] && echo 'KEEP' || echo 'REMOVE'))"
echo "  Data:         $DATA_DIR ($([ "$KEEP_DATA" = true ] && echo 'KEEP' || echo 'REMOVE'))"
echo "  Logs:         $LOG_DIR ($([ "$KEEP_LOGS" = true ] && echo 'KEEP' || echo 'REMOVE'))"
echo "  Remove user:  $([ "$PURGE" = true ] && echo 'YES' || echo 'NO')"
echo ""

if [[ "$UNATTENDED" != true ]]; then
    read -rp "Proceed with uninstallation? [y/N] " confirm
    if [[ ! "$confirm" =~ ^[Yy] ]]; then
        log_info "Uninstallation cancelled."
        exit 0
    fi
fi

# ── Step 1: Stop and disable service ──
log_step "Stopping service"
if systemctl is-active --quiet draox-server 2>/dev/null; then
    systemctl stop draox-server
    log_info "Service stopped"
fi

if systemctl is-enabled --quiet draox-server 2>/dev/null; then
    systemctl disable draox-server
    log_info "Service disabled"
fi

# ── Step 2: Remove systemd unit ──
log_step "Removing systemd service"
rm -f /etc/systemd/system/draox-server.service
systemctl daemon-reload
log_info "Service file removed"

# ── Step 3: Remove firewall rules ──
log_step "Removing firewall rules"
if command -v ufw &>/dev/null; then
    ufw delete allow 9000/tcp 2>/dev/null || true
    ufw delete allow 9001/udp 2>/dev/null || true
    ufw delete allow 9002/tcp 2>/dev/null || true
    ufw delete allow 9003/tcp 2>/dev/null || true
    log_info "UFW rules removed"
elif command -v firewall-cmd &>/dev/null; then
    firewall-cmd --permanent --remove-port=9000/tcp 2>/dev/null || true
    firewall-cmd --permanent --remove-port=9001/udp 2>/dev/null || true
    firewall-cmd --permanent --remove-port=9002/tcp 2>/dev/null || true
    firewall-cmd --permanent --remove-port=9003/tcp 2>/dev/null || true
    firewall-cmd --permanent --remove-rich-rule='rule family="ipv4" source address="127.0.0.0/8" port port="9100" protocol="tcp" accept' 2>/dev/null || true
    firewall-cmd --reload 2>/dev/null || true
    log_info "firewalld rules removed"
fi

# ── Step 4: Remove logrotate ──
rm -f /etc/logrotate.d/draox-server
log_info "Logrotate config removed"

# ── Step 5: Remove binary and prefix ──
log_step "Removing binary"
rm -f "$PREFIX/bin/draox-server"
rmdir "$PREFIX/bin" 2>/dev/null || true

if [[ "$KEEP_DATA" != true ]]; then
    rm -rf "$PREFIX/plugins"
fi
rmdir "$PREFIX" 2>/dev/null || true
log_info "Binary removed"

# ── Step 6: Remove directories ──
if [[ "$KEEP_CONFIG" != true ]]; then
    log_step "Removing config: $CONFIG_DIR"
    rm -rf "$CONFIG_DIR"
    log_info "Config removed"
else
    log_warn "Keeping config: $CONFIG_DIR"
fi

if [[ "$KEEP_DATA" != true ]]; then
    log_step "Removing data: $DATA_DIR"
    rm -rf "$DATA_DIR"
    log_info "Data removed"
else
    log_warn "Keeping data: $DATA_DIR"
fi

if [[ "$KEEP_LOGS" != true ]]; then
    log_step "Removing logs: $LOG_DIR"
    rm -rf "$LOG_DIR"
    log_info "Logs removed"
else
    log_warn "Keeping logs: $LOG_DIR"
fi

# ── Step 7: Remove user ──
if [[ "$PURGE" == true ]]; then
    log_step "Removing system user: $SERVICE_USER"
    if id "$SERVICE_USER" &>/dev/null; then
        userdel "$SERVICE_USER" 2>/dev/null || true
        log_info "User '$SERVICE_USER' removed"
    fi
fi

echo ""
echo "========================================="
echo "  Uninstallation Complete!"
echo "========================================="
echo ""
if [[ "$KEEP_CONFIG" == true ]]; then
    echo "  Config preserved at: $CONFIG_DIR"
fi
if [[ "$KEEP_DATA" == true ]]; then
    echo "  Data preserved at:   $DATA_DIR"
fi
echo ""
