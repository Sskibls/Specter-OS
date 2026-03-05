#!/bin/bash
#
# PhantomKernel OS - Simple ISO Builder (TUI only, no GUI dependencies)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${OUTPUT_DIR:-$PROJECT_ROOT/output}"
ISO_NAME="phantomkernel-os-tui-$(date +%Y%m%d).iso"
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

cleanup() {
    if [[ -d "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT

log_step "Building PhantomKernel OS TUI ISO..."

# Create root filesystem structure
log_step "Creating root filesystem..."
mkdir -p "$WORK_DIR/rootfs"/{bin,boot,etc,home,lib,lib64,opt,proc,root,run,sbin,sys,tmp,usr,var}
mkdir -p "$WORK_DIR/rootfs/opt/phantomkernel"/{bin,lib,config}
mkdir -p "$WORK_DIR/rootfs/etc/phantomkernel"
mkdir -p "$WORK_DIR/rootfs/boot/grub"

# Build TUI binary
log_step "Building TUI binary..."
cd "$PROJECT_ROOT"
cargo build --release -p phantomkernel-tui --target x86_64-unknown-linux-gnu 2>/dev/null || {
    log_warning "Cross-compile not available, building for host..."
    cargo build --release -p phantomkernel-tui
}

# Copy binaries
log_info "Copying binaries..."
if [[ -f "target/release/phantomkernel-tui" ]]; then
    cp target/release/phantomkernel-tui "$WORK_DIR/rootfs/opt/phantomkernel/bin/"
    cp target/release/phantomkernel-shell "$WORK_DIR/rootfs/opt/phantomkernel/bin/" 2>/dev/null || true
    cp target/release/gkctl "$WORK_DIR/rootfs/opt/phantomkernel/bin/" 2>/dev/null || true
fi

# Copy all daemon binaries
for daemon in phantomkernel-init phantomkernel-policyd phantomkernel-shardd phantomkernel-netd phantomkernel-airlockd phantomkernel-auditd phantomkernel-guardian phantomkernel-updated; do
    if [[ -f "target/release/$daemon" ]]; then
        cp "target/release/$daemon" "$WORK_DIR/rootfs/opt/phantomkernel/bin/"
    fi
done

# Create init script
cat > "$WORK_DIR/rootfs/opt/phantomkernel/init.sh" << 'INITSCRIPT'
#!/bin/bash
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║         PhantomKernel OS v0.1.0 - TUI Edition               ║"
echo "║     Privacy-First Secure Operating System                 ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
export PATH="/opt/phantomkernel/bin:$PATH"

# Start core daemons in background
echo "Starting PhantomKernel services..."
for service in phantomkernel-init phantomkernel-policyd phantomkernel-shardd phantomkernel-netd phantomkernel-auditd; do
    if command -v $service &> /dev/null; then
        $service &
        echo "  ✓ Started $service"
    fi
done

echo ""
echo "Services started. Launching terminal interface..."
echo ""
echo "To start the TUI dashboard, run: phantomkernel-tui"
echo "To start the interactive shell, run: phantomkernel-shell"
echo ""

# Auto-start TUI
if command -v phantomkernel-tui &> /dev/null; then
    phantomkernel-tui
else
    /bin/bash
fi
INITSCRIPT
chmod +x "$WORK_DIR/rootfs/opt/phantomkernel/init.sh"

# Create default config
cat > "$WORK_DIR/rootfs/etc/phantomkernel/config.toml" << 'CONFIG'
[general]
theme = "default"
log_level = "info"
audit_enabled = true
tui_mode = true

[shards]
default_shards = ["work", "anon", "burner", "lab"]

[network]
kill_switch_default = false
dns_leak_protection = true
ipv6_policy = "disabled"

[security]
secure_boot_enforce = true
tpm_required = true
CONFIG

# Create GRUB config
cat > "$WORK_DIR/rootfs/boot/grub/grub.cfg" << 'GRUBCFG'
set timeout=5
set default=0

menuentry "PhantomKernel OS (TUI)" {
    linux /boot/bzImage root=/dev/sda1 ro quiet
    initrd /boot/initrd.img
}

menuentry "PhantomKernel OS (Debug)" {
    linux /boot/bzImage root=/dev/sda1 ro debug loglevel=7
    initrd /boot/initrd.img
}

menuentry "PhantomKernel OS (Rescue)" {
    linux /boot/bzImage root=/dev/sda1 ro single
    initrd /boot/initrd.img
}
GRUBCFG

# Create a minimal initrd placeholder
log_info "Creating initrd placeholder..."
cd "$WORK_DIR/rootfs"
find . | cpio -o -H newc 2>/dev/null | gzip > "$WORK_DIR/rootfs/boot/initrd.img" || {
    # Fallback if cpio not available
    dd if=/dev/zero of="$WORK_DIR/rootfs/boot/initrd.img" bs=1M count=16 2>/dev/null
}

# Create stub kernel (in production, this would be actual kernel)
log_info "Creating kernel placeholder..."
dd if=/dev/zero of="$WORK_DIR/rootfs/boot/bzImage" bs=1M count=32 2>/dev/null

# Build ISO
log_step "Building ISO image..."
mkdir -p "$OUTPUT_DIR"
cd "$WORK_DIR"

# Check for xorriso
if command -v xorriso &> /dev/null; then
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
        -o "$OUTPUT_DIR/$ISO_NAME" \
        rootfs/ 2>/dev/null || {
        # Fallback
        xorriso -as mkisofs -o "$OUTPUT_DIR/$ISO_NAME" rootfs/
    }
    log_success "ISO created with xorriso"
else
    log_warning "xorriso not found, using genisoimage or mkisofs..."
    if command -v genisoimage &> /dev/null; then
        genisoimage -o "$OUTPUT_DIR/$ISO_NAME" -R -J rootfs/
    elif command -v mkisofs &> /dev/null; then
        mkisofs -o "$OUTPUT_DIR/$ISO_NAME" -R -J rootfs/
    else
        log_error "No ISO creation tool found. Installing xorriso recommended."
        # Create a tarball instead
        tar -czf "$OUTPUT_DIR/phantomkernel-os-tui-$(date +%Y%m%d).tar.gz" -C "$WORK_DIR" rootfs
        log_info "Created tarball instead: phantomkernel-os-tui-$(date +%Y%m%d).tar.gz"
    fi
fi

# Generate checksum
if [[ -f "$OUTPUT_DIR/$ISO_NAME" ]]; then
    sha256sum "$OUTPUT_DIR/$ISO_NAME" > "$OUTPUT_DIR/$ISO_NAME.sha256"
    log_success "ISO created: $OUTPUT_DIR/$ISO_NAME"
    log_info "SHA256: $(cat "$OUTPUT_DIR/$ISO_NAME.sha256")"
fi

# Summary
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "                    Build Complete"
echo "═══════════════════════════════════════════════════════════"
echo ""
if [[ -f "$OUTPUT_DIR/$ISO_NAME" ]]; then
    echo "Output: $OUTPUT_DIR/$ISO_NAME"
    ls -lh "$OUTPUT_DIR/$ISO_NAME"
    echo ""
    echo "To test with QEMU + VNC:"
    echo "  qemu-system-x86_64 -cdrom \"$OUTPUT_DIR/$ISO_NAME\" -m 2G -boot d -vnc :0"
    echo ""
    echo "Then connect with VNC client to: localhost:5900"
else
    echo "Note: ISO tools not available. Check $WORK_DIR for rootfs."
fi
echo "═══════════════════════════════════════════════════════════"
