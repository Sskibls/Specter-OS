#!/bin/bash
#
# PhantomKernel OS Installer
# Privacy-First Secure Operating System
#
# Usage: ./install-phantomkernel.sh [options]
#   --dry-run        Show what would be done without making changes
#   --skip-secure-boot  Skip Secure Boot key enrollment
#   --theme <name>   Set default theme (default|fsociety|allsafe|darkarmy)
#   --help           Show this help message
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
INSTALL_PREFIX="${INSTALL_PREFIX:-/opt/phantomkernel}"
SYSTEMD_DIR="/etc/systemd/system"
CONFIG_DIR="/etc/phantomkernel"
USER_CONFIG_DIR="$HOME/.config/phantomkernel"
BIN_DIR="$INSTALL_PREFIX/bin"
LIB_DIR="$INSTALL_PREFIX/lib"
THEME_DIR="$INSTALL_PREFIX/themes"

# Options
DRY_RUN=false
SKIP_SECURE_BOOT=false
DEFAULT_THEME="default"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${CYAN}[STEP]${NC} $1"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --skip-secure-boot)
            SKIP_SECURE_BOOT=true
            shift
            ;;
        --theme)
            DEFAULT_THEME="$2"
            shift 2
            ;;
        --help)
            head -20 "$0" | tail -15
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if running as root
check_root() {
    log_step "Checking privileges..."
    if [[ $EUID -ne 0 ]]; then
        log_warning "Not running as root. Some features may require sudo."
        SUDO_CMD="sudo"
    else
        SUDO_CMD=""
    fi
}

# System requirements check
check_requirements() {
    log_step "Checking system requirements..."
    
    # Check for required tools
    local required_tools=("systemctl" "cargo" "rustc")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_warning "$tool not found. Some features may be limited."
        fi
    done
    
    # Check disk space (need at least 2GB free)
    local available_space=$(df -P / | tail -1 | awk '{print $4}')
    if [[ $available_space -lt 2097152 ]]; then
        log_error "Insufficient disk space. Need at least 2GB free."
        exit 1
    fi
    
    log_success "System requirements check passed"
}

# Build binaries
build_binaries() {
    log_step "Building PhantomKernel OS binaries..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would run: cargo build --release"
        return 0
    fi
    
    cd "$(dirname "$0")"
    
    if ! cargo build --release 2>&1; then
        log_error "Build failed!"
        exit 1
    fi
    
    log_success "Binaries built successfully"
}

# Install binaries
install_binaries() {
    log_step "Installing binaries to $INSTALL_PREFIX..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would create directories: $BIN_DIR, $LIB_DIR"
        log_info "[DRY-RUN] Would copy binaries to $BIN_DIR"
        return 0
    fi
    
    $SUDO_CMD mkdir -p "$BIN_DIR" "$LIB_DIR" "$THEME_DIR"
    
    # Copy binaries
    local binaries=(
        "phantomkernel-init"
        "phantomkernel-policyd"
        "phantomkernel-shardd"
        "phantomkernel-netd"
        "phantomkernel-airlockd"
        "phantomkernel-auditd"
        "phantomkernel-guardian"
        "phantomkernel-updated"
        "gkctl"
        "phantomkernel-shell"
    )
    
    for binary in "${binaries[@]}"; do
        local src="target/release/$binary"
        if [[ -f "$src" ]]; then
            $SUDO_CMD cp "$src" "$BIN_DIR/$binary"
            $SUDO_CMD chmod 755 "$BIN_DIR/$binary"
            log_info "  Installed: $binary"
        else
            log_warning "Binary not found: $binary"
        fi
    done
    
    # Copy themes
    if [[ -d "ui/themes" ]]; then
        $SUDO_CMD cp -r ui/themes/* "$THEME_DIR/"
        log_success "Themes installed"
    fi
    
    log_success "Binaries installed"
}

# Install systemd services
install_systemd_services() {
    log_step "Installing systemd services..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would install systemd service units"
        return 0
    fi
    
    # Create service files
    local services=(
        "phantomkernel-init"
        "phantomkernel-policyd"
        "phantomkernel-shardd"
        "phantomkernel-netd"
        "phantomkernel-airlockd"
        "phantomkernel-auditd"
        "phantomkernel-guardian"
        "phantomkernel-updated"
    )
    
    for service in "${services[@]}"; do
        cat > "/tmp/${service}.service" << EOF
[Unit]
Description=PhantomKernel OS ${service}
Documentation=https://phantomkernel.org/docs
After=network.target
Wants=network.target

[Service]
Type=simple
ExecStart=$BIN_DIR/$service
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF
        
        $SUDO_CMD cp "/tmp/${service}.service" "$SYSTEMD_DIR/"
        rm -f "/tmp/${service}.service"
        log_info "  Installed service: $service"
    done
    
    # Reload systemd
    $SUDO_CMD systemctl daemon-reload
    
    log_success "Systemd services installed"
}

# Enable services
enable_services() {
    log_step "Enabling PhantomKernel services..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would enable and start services"
        return 0
    fi
    
    local services=(
        "phantomkernel-init"
        "phantomkernel-policyd"
        "phantomkernel-shardd"
        "phantomkernel-netd"
        "phantomkernel-airlockd"
        "phantomkernel-auditd"
        "phantomkernel-guardian"
        "phantomkernel-updated"
    )
    
    for service in "${services[@]}"; do
        if $SUDO_CMD systemctl enable "$service" 2>/dev/null; then
            log_info "  Enabled: $service"
        fi
    done
    
    log_success "Services enabled"
}

# Create configuration
create_config() {
    log_step "Creating configuration files..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would create config in $CONFIG_DIR"
        return 0
    fi
    
    $SUDO_CMD mkdir -p "$CONFIG_DIR"
    
    # Main config
    cat > "/tmp/phantomkernel.conf" << EOF
# PhantomKernel OS Configuration
# Generated by installer

[general]
theme = $DEFAULT_THEME
log_level = info
audit_enabled = true

[shards]
default_shards = work,anon,burner,lab

[network]
kill_switch_default = false
dns_leak_protection = true
ipv6_policy = disabled

[security]
secure_boot_enforce = true
tpm_required = true
EOF
    
    $SUDO_CMD cp "/tmp/phantomkernel.conf" "$CONFIG_DIR/"
    rm -f "/tmp/phantomkernel.conf"
    
    # User config
    mkdir -p "$USER_CONFIG_DIR"
    cat > "$USER_CONFIG_DIR/config.toml" << EOF
# User-specific PhantomKernel configuration

[preferences]
default_shard = work
auto_lock_timeout = 300  # seconds
panic_button_enabled = true

[theme]
name = $DEFAULT_THEME
EOF
    
    log_success "Configuration created"
}

# Setup Secure Boot keys (optional)
setup_secure_boot() {
    if [[ "$SKIP_SECURE_BOOT" == "true" ]]; then
        log_info "Skipping Secure Boot setup (user requested)"
        return 0
    fi
    
    log_step "Setting up Secure Boot keys..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would generate Secure Boot keys"
        log_info "[DRY-RUN] Would enroll keys in firmware"
        return 0
    fi
    
    # Check if Secure Boot is available
    if ! command -v mokutil &> /dev/null; then
        log_warning "mokutil not found. Secure Boot setup skipped."
        log_info "Install mokutil to enable Secure Boot key enrollment"
        return 0
    fi
    
    # Generate keys
    $SUDO_CMD mkdir -p "$CONFIG_DIR/secure-boot"
    
    # Generate Platform Key (PK)
    if ! openssl genrsa -out "$CONFIG_DIR/secure-boot/PK.key" 4096 2>/dev/null; then
        log_warning "Failed to generate PK key"
        return 0
    fi
    
    openssl req -new -x509 -key "$CONFIG_DIR/secure-boot/PK.key" \
        -out "$CONFIG_DIR/secure-boot/PK.crt" \
        -days 3650 -subj "/CN=PhantomKernel OS Platform Key" 2>/dev/null
    
    # Generate Key Exchange Key (KEK)
    openssl genrsa -out "$CONFIG_DIR/secure-boot/KEK.key" 4096 2>/dev/null
    openssl req -new -x509 -key "$CONFIG_DIR/secure-boot/KEK.key" \
        -out "$CONFIG_DIR/secure-boot/KEK.crt" \
        -days 3650 -subj "/CN=PhantomKernel OS Key Exchange Key" 2>/dev/null
    
    # Generate Signature Database Key (db)
    openssl genrsa -out "$CONFIG_DIR/secure-boot/db.key" 4096 2>/dev/null
    openssl req -new -x509 -key "$CONFIG_DIR/secure-boot/db.key" \
        -out "$CONFIG_DIR/secure-boot/db.crt" \
        -days 3650 -subj "/CN=PhantomKernel OS Signature Database Key" 2>/dev/null
    
    log_success "Secure Boot keys generated in $CONFIG_DIR/secure-boot/"
    log_warning "To enroll keys, run: sudo mokutil --import PK.crt"
}

# Create user groups
setup_groups() {
    log_step "Setting up user groups..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY-RUN] Would create phantomkernel group"
        return 0
    fi
    
    if ! getent group phantomkernel > /dev/null 2>&1; then
        $SUDO_CMD groupadd phantomkernel
        log_info "Created group: phantomkernel"
    fi
    
    # Add current user to group
    if ! groups "$USER" | grep -q phantomkernel; then
        $SUDO_CMD usermod -aG phantomkernel "$USER"
        log_info "Added $USER to phantomkernel group"
    fi
    
    log_success "User groups configured"
}

# Print summary
print_summary() {
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "           PhantomKernel OS Installation Complete"
    echo "═══════════════════════════════════════════════════════════"
    echo ""
    echo "Installation prefix: $INSTALL_PREFIX"
    echo "Configuration dir:   $CONFIG_DIR"
    echo "User config dir:     $USER_CONFIG_DIR"
    echo "Default theme:       $DEFAULT_THEME"
    echo ""
    echo "Next steps:"
    echo "  1. Add to PATH: export PATH=\"$BIN_DIR:\$PATH\""
    echo "  2. Start shell: phantomkernel-shell"
    echo "  3. Configure shards: gkctl shard ls"
    echo ""
    if [[ "$SKIP_SECURE_BOOT" != "true" ]]; then
        echo "  4. Enroll Secure Boot keys (recommended):"
        echo "     sudo mokutil --import $CONFIG_DIR/secure-boot/PK.crt"
    fi
    echo ""
    echo "Documentation: https://phantomkernel.org/docs"
    echo "═══════════════════════════════════════════════════════════"
}

# Main
main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║         PhantomKernel OS Installer v0.1.0                   ║"
    echo "║     Privacy-First Secure Operating System                 ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo ""
    
    check_root
    check_requirements
    build_binaries
    install_binaries
    install_systemd_services
    enable_services
    create_config
    setup_secure_boot
    setup_groups
    print_summary
}

main "$@"
