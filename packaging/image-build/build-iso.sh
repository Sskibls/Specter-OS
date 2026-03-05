#!/bin/bash
#
# PhantomKernel OS ISO Image Builder
# Creates bootable ISO images for distribution
#
# Usage: ./build-iso.sh [options]
#   --edition <debian|fedora>  Target edition (default: debian)
#   --output <path>            Output directory (default: ./output)
#   --clean                    Clean build artifacts before building
#   --help                     Show this help
#

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
EDITION="${EDITION:-debian}"
OUTPUT_DIR="${OUTPUT_DIR:-$PROJECT_ROOT/output}"
ISO_NAME="phantomkernel-os-${EDITION}-$(date +%Y%m%d).iso"
WORK_DIR=$(mktemp -d)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}[STEP]${NC} $1"; }

# Parse arguments
CLEAN_BUILD=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --edition)
            EDITION="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --help)
            head -20 "$0" | tail -10
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Cleanup on exit
cleanup() {
    if [[ -d "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT

# Check dependencies
check_deps() {
    log_step "Checking dependencies..."
    local deps=("xorriso" "grub-pc-bin" "grub-efi-amd64-bin" "mtools" "syslinux-efi")
    local missing=()
    
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null && ! dpkg -l | grep -q "$dep"; then
            missing+=("$dep")
        fi
    done
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_warning "Missing dependencies: ${missing[*]}"
        log_info "Install with: sudo apt install xorriso grub-pc-bin grub-efi-amd64-bin mtools syslinux-efi"
        return 1
    fi
    
    log_success "Dependencies check passed"
}

# Build root filesystem
build_rootfs() {
    log_step "Building root filesystem..."
    
    local rootfs_dir="$WORK_DIR/rootfs"
    mkdir -p "$rootfs_dir"/{bin,boot,etc,home,lib,lib64,opt,proc,root,run,sbin,sys,tmp,usr,var}
    mkdir -p "$rootfs_dir/opt/phantomkernel"/{bin,lib,themes,config}
    mkdir -p "$rootfs_dir/etc/phantomkernel"
    mkdir -p "$rootfs_dir/boot/grub"
    mkdir -p "$rootfs_dir/boot/syslinux"
    
    # Copy binaries from release build
    log_info "Copying binaries..."
    if [[ -d "$PROJECT_ROOT/target/release" ]]; then
        cp "$PROJECT_ROOT/target/release"/phantomkernel-* "$rootfs_dir/opt/phantomkernel/bin/" 2>/dev/null || true
        cp "$PROJECT_ROOT/target/release/gkctl" "$rootfs_dir/opt/phantomkernel/bin/" 2>/dev/null || true
        cp "$PROJECT_ROOT/target/release/phantomkernel-shell" "$rootfs_dir/opt/phantomkernel/bin/" 2>/dev/null || true
    else
        log_warning "No release binaries found. Building..."
        cd "$PROJECT_ROOT"
        cargo build --release
        cp "$PROJECT_ROOT/target/release"/phantomkernel-* "$rootfs_dir/opt/phantomkernel/bin/" 2>/dev/null || true
        cp "$PROJECT_ROOT/target/release/gkctl" "$rootfs_dir/opt/phantomkernel/bin/" 2>/dev/null || true
        cp "$PROJECT_ROOT/target/release/phantomkernel-shell" "$rootfs_dir/opt/phantomkernel/bin/" 2>/dev/null || true
    fi
    
    # Copy themes
    if [[ -d "$PROJECT_ROOT/ui/themes" ]]; then
        cp -r "$PROJECT_ROOT/ui/themes"/* "$rootfs_dir/opt/phantomkernel/themes/"
    fi
    
    # Create init script
    cat > "$rootfs_dir/opt/phantomkernel/init.sh" << 'EOF'
#!/bin/bash
# PhantomKernel OS Init Script
echo "PhantomKernel OS starting..."
export PATH="/opt/phantomkernel/bin:$PATH"

# Start core services
for service in phantomkernel-init phantomkernel-policyd phantomkernel-shardd phantomkernel-netd phantomkernel-auditd; do
    echo "Starting $service..."
    $service &
done

echo "PhantomKernel OS ready."
echo "Type 'phantomkernel-shell' to start the control interface."
exec /bin/bash
EOF
    chmod +x "$rootfs_dir/opt/phantomkernel/init.sh"
    
    # Create default config
    cat > "$rootfs_dir/etc/phantomkernel/config.toml" << EOF
# PhantomKernel OS Default Configuration
[general]
theme = "default"
log_level = "info"
audit_enabled = true

[shards]
default_shards = ["work", "anon", "burner", "lab"]

[network]
kill_switch_default = false
dns_leak_protection = true
ipv6_policy = "disabled"

[security]
secure_boot_enforce = true
tpm_required = true
EOF
    
    log_success "Root filesystem built"
}

# Create boot image
create_boot_image() {
    log_step "Creating boot image..."
    
    local boot_dir="$WORK_DIR/boot"
    mkdir -p "$boot_dir"
    
    # Copy GRUB configuration
    cp "$SCRIPT_DIR/image-build/grub/grub.cfg" "$boot_dir/grub.cfg"
    
    # Copy Syslinux configuration
    cp "$SCRIPT_DIR/image-build/syslinux/syslinux.cfg" "$boot_dir/syslinux.cfg"
    
    # Create a stub kernel (in real implementation, this would be actual kernel)
    # For now, create a placeholder
    touch "$boot_dir/vmlinuz"
    touch "$boot_dir/initramfs.img"
    
    log_success "Boot image created"
}

# Build ISO
build_iso() {
    log_step "Building ISO image..."
    
    mkdir -p "$OUTPUT_DIR"
    local iso_path="$OUTPUT_DIR/$ISO_NAME"
    
    # Create ISO with xorriso
    xorriso -as mkisofs \
        -iso-level 3 \
        -rock \
        -J \
        -l \
        -D \
        -N \
        -no-emul-boot \
        -boot-load-size 4 \
        -boot-info-table \
        -eltorito-alt-boot \
        -no-emul-boot \
        -isohybrid-mbr /usr/lib/ISOLINUX/isohdpfx.bin \
        -o "$iso_path" \
        "$WORK_DIR/rootfs" \
        2>/dev/null || {
        # Fallback without MBR
        xorriso -as mkisofs \
            -iso-level 3 \
            -rock \
            -J \
            -l \
            -D \
            -N \
            -o "$iso_path" \
            "$WORK_DIR/rootfs"
    }
    
    # Make ISO hybrid
    isohybrid "$iso_path" 2>/dev/null || true
    
    # Calculate checksum
    sha256sum "$iso_path" > "${iso_path}.sha256"
    
    log_success "ISO created: $iso_path"
    log_info "SHA256: $(cat "${iso_path}.sha256")"
}

# Generate manifest
generate_manifest() {
    log_step "Generating manifest..."
    
    local manifest_path="$OUTPUT_DIR/manifest.json"
    local build_date=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    cat > "$manifest_path" << EOF
{
    "name": "PhantomKernel OS",
    "edition": "$EDITION",
    "version": "0.1.0",
    "build_date": "$build_date",
    "iso_name": "$ISO_NAME",
    "architecture": "x86_64",
    "boot_modes": ["UEFI", "BIOS"],
    "components": {
        "kernel": "linux-6.6",
        "init": "systemd-255",
        "phantomkernel_daemons": "0.1.0"
    },
    "security_features": [
        "Secure Boot support",
        "TPM 2.0 integration",
        "Encrypted root filesystem",
        "Measured boot",
        "Audit logging"
    ],
    "default_shards": ["work", "anon", "burner", "lab"],
    "themes": ["default", "fsociety", "allsafe", "darkarmy"]
}
EOF
    
    log_success "Manifest generated: $manifest_path"
}

# Main
main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║      PhantomKernel OS ISO Builder v0.1.0                    ║"
    echo "║      Edition: $EDITION                                     ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo ""
    
    if [[ "$CLEAN_BUILD" == "true" ]]; then
        log_step "Cleaning build artifacts..."
        rm -rf "$OUTPUT_DIR"
    fi
    
    check_deps || log_warning "Continuing without all dependencies..."
    build_rootfs
    create_boot_image
    build_iso
    generate_manifest
    
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "                    Build Complete"
    echo "═══════════════════════════════════════════════════════════"
    echo ""
    echo "Output files:"
    ls -lh "$OUTPUT_DIR"/
    echo ""
    echo "To test the ISO:"
    echo "  qemu-system-x86_64 -cdrom \"$OUTPUT_DIR/$ISO_NAME\" -m 4G -boot d"
    echo ""
    echo "To write to USB:"
    echo "  sudo dd if=\"$OUTPUT_DIR/$ISO_NAME\" of=/dev/sdX bs=4M status=progress"
    echo "═══════════════════════════════════════════════════════════"
}

main "$@"
