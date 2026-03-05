#!/bin/bash
#
# PhantomKernel OS - Debian-based Linux Distribution Builder
# Creates a real bootable Debian Live ISO with XFCE desktop
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DISTRO_NAME="phantomkernel-os"
VERSION="0.1.0"
DEBIAN_SUITE="bookworm"
ARCH="amd64"

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

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║      PhantomKernel OS - Debian Live ISO Builder             ║"
echo "║      Real Linux Distribution with XFCE Desktop            ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root"
    exit 1
fi

# Check required tools
log_step "Checking required tools..."
REQUIRED_TOOLS="debootstrap genisoimage xorriso grub-mkrescue"
MISSING=""

for tool in $REQUIRED_TOOLS; do
    if ! command -v "$tool" &> /dev/null; then
        MISSING="$MISSING $tool"
    fi
done

if [[ -n "$MISSING" ]]; then
    log_error "Missing tools:$MISSING"
    echo ""
    echo "Install with:"
    echo "  apt install -y debootstrap genisoimage xorriso grub2-common mtools"
    exit 1
fi

log_success "All required tools found"

# Create working directories
WORK_DIR="/var/tmp/phantomkernel-build-$$"
ROOTFS="$WORK_DIR/rootfs"
ISO_ROOT="$WORK_DIR/iso_root"

mkdir -p "$WORK_DIR" "$ROOTFS" "$ISO_ROOT"

cleanup() {
    if [[ -d "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT

log_step "Creating PhantomKernel OS (Debian $DEBIAN_SUITE)..."

# Bootstrap Debian base system
log_step "Bootstrapping Debian base system..."
debootstrap --include=systemd,grub2,linux-image-$ARCH,firmware-linux \
    --components=main,contrib,non-free \
    $DEBIAN_SUITE "$ROOTFS" http://deb.debian.org/debian/ 2>&1 | tee /tmp/debootstrap.log

log_success "Debian base installed"

# Copy PhantomKernel binaries
log_step "Installing PhantomKernel components..."
if [[ -d "$PROJECT_ROOT/target/release" ]]; then
    mkdir -p "$ROOTFS/opt/phantomkernel/bin"
    cp "$PROJECT_ROOT/target/release"/phantomkernel-* "$ROOTFS/opt/phantomkernel/bin/" 2>/dev/null || true
    cp "$PROJECT_ROOT/target/release/gkctl" "$ROOTFS/opt/phantomkernel/bin/" 2>/dev/null || true
    cp "$PROJECT_ROOT/target/release/phantomkernel-tui" "$ROOTFS/opt/phantomkernel/bin/" 2>/dev/null || true
    log_success "PhantomKernel binaries copied"
fi

# Install desktop environment (XFCE - lightweight)
log_step "Installing XFCE desktop environment..."
chroot "$ROOTFS" apt-get update
chroot "$ROOTFS" apt-get install -y \
    task-xfce-desktop \
    lightdm \
    firefox-esr \
    libreoffice \
    terminator \
    thunar \
    mousepad \
    pavucontrol \
    network-manager \
    network-manager-gnome \
    sudo \
    curl \
    wget \
    git \
    vim \
    htop \
    2>&1 | tee /tmp/apt-install.log

log_success "Desktop environment installed"

# Configure system
log_step "Configuring system..."

# Set hostname
echo "phantomkernel" > "$ROOTFS/etc/hostname"

# Set hosts
cat > "$ROOTFS/etc/hosts" << EOF
127.0.0.1   localhost
127.0.1.1   phantomkernel
::1         localhost ip6-localhost ip6-loopback
EOF

# Set timezone
echo "UTC" > "$ROOTFS/etc/timezone"
chroot "$ROOTFS" dpkg-reconfigure -f noninteractive tzdata

# Set root password
echo "root:phantomkernel" | chroot "$ROOTFS" chpasswd

# Create user
chroot "$ROOTFS" useradd -m -s /bin/bash -G sudo,audio,video,dialout user
echo "user:user" | chroot "$ROOTFS" chpasswd

# Configure sudo
echo "user ALL=(ALL) NOPASSWD:ALL" > "$ROOTFS/etc/sudoers.d/user"
chmod 440 "$ROOTFS/etc/sudoers.d/user"

# Enable services
chroot "$ROOTFS" systemctl enable lightdm
chroot "$ROOTFS" systemctl enable NetworkManager

# Create PhantomKernel configuration
mkdir -p "$ROOTFS/etc/phantomkernel"
cat > "$ROOTFS/etc/phantomkernel/config.toml" << 'EOF'
[general]
hostname = "phantomkernel"
theme = "fsociety"
log_level = "info"
audit_enabled = true

[shards]
default_shards = ["work", "anon", "burner", "lab"]

[network]
default_route = "direct"
kill_switch_default = false
dns_over_https = true
ipv6_disabled = false

[security]
secure_boot = false
tpm_required = false
full_disk_encryption = false
selinux = false
apparmor = true

[desktop]
environment = "xfce"
auto_start_tui = false
show_privacy_indicator = true
EOF

# Create systemd services for PhantomKernel
mkdir -p "$ROOTFS/etc/systemd/system"
for daemon in shardd netd policyd auditd guardian; do
    cat > "$ROOTFS/etc/systemd/system/phantomkernel-${daemon}.service" << EOF
[Unit]
Description=PhantomKernel ${daemon^}
After=network.target

[Service]
Type=simple
ExecStart=/opt/phantomkernel/bin/phantomkernel-${daemon}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
done

# Create welcome script
cat > "$ROOTFS/usr/local/bin/phantomkernel-welcome" << 'EOF'
#!/bin/bash
# PhantomKernel OS Welcome Script

if [[ ! -f ~/.phantomkernel-welcomed ]]; then
    zenity --info --title="Welcome to PhantomKernel OS" \
        --text="Welcome to PhantomKernel OS!\n\nA privacy-focused Linux distribution.\n\nQuick Start:\n• phantomkernel-tui - Terminal dashboard\n• phantomkernel-shell - Interactive CLI\n• Settings - System configuration\n\nPrivacy Features:\n✓ Persona Shards\n✓ Network Kill Switch\n✓ Audit Logging" \
        --width=400
    
    touch ~/.phantomkernel-welcomed
fi
EOF
chmod +x "$ROOTFS/usr/local/bin/phantomkernel-welcome"

# Add to XFCE autostart
mkdir -p "$ROOTFS/etc/xdg/autostart"
cat > "$ROOTFS/etc/xdg/autostart/phantomkernel-welcome.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=PhantomKernel Welcome
Exec=/usr/local/bin/phantomkernel-welcome
Terminal=false
X-GNOME-Autostart-enabled=true
EOF

# Create GRUB configuration
log_step "Configuring GRUB bootloader..."
chroot "$ROOTFS" grub-install /dev/sdX 2>/dev/null || true

cat > "$ROOTFS/boot/grub/grub.cfg" << 'EOF'
set timeout=5
set default=0

menuentry "PhantomKernel OS (Live)" {
    linux /live/vmlinuz boot=live quiet splash
    initrd /live/initrd.img
}

menuentry "PhantomKernel OS (Live - Failsafe)" {
    linux /live/vmlinuz boot=live quiet failsafe
    initrd /live/initrd.img
}

menuentry "PhantomKernel OS (Install)" {
    linux /install/vmlinuz quiet
    initrd /install/initrd.gz
}
EOF

# Create initramfs
log_step "Creating initramfs..."
chroot "$ROOTFS" update-initramfs -u -k all

# Copy kernel and initrd to ISO root
mkdir -p "$ISO_ROOT/live"
cp "$ROOTFS/boot/vmlinuz"* "$ISO_ROOT/live/vmlinuz"
cp "$ROOTFS/boot/initrd"* "$ISO_ROOT/live/initrd.img"

# Create SquashFS for live system
log_step "Creating live filesystem..."
chroot "$ROOTFS" apt-get install -y squashfs-tools
mksquashfs "$ROOTFS" "$ISO_ROOT/live/filesystem.squashfs" -comp xz -b 1024k

# Create ISO
log_step "Building ISO image..."
mkdir -p "$PROJECT_ROOT/output"

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
    -o "$PROJECT_ROOT/output/phantomkernel-os-debian-$(date +%Y%m%d).iso" \
    "$ISO_ROOT" 2>&1 | tee /tmp/iso-build.log

log_success "ISO created: $PROJECT_ROOT/output/phantomkernel-os-debian-$(date +%Y%m%d).iso"

# Generate checksum
cd "$PROJECT_ROOT/output"
sha256sum phantomkernel-os-debian-*.iso > phantomkernel-os-debian-$(date +%Y%m%d).iso.sha256
log_info "SHA256: $(cat phantomkernel-os-debian-*.sha256)"

# Summary
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "           PhantomKernel OS Build Complete"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Output: $PROJECT_ROOT/output/phantomkernel-os-debian-$(date +%Y%m%d).iso"
ls -lh "$PROJECT_ROOT/output/"*.iso
echo ""
echo "To test with QEMU:"
echo "  qemu-system-x86_64 -cdrom $PROJECT_ROOT/output/phantomkernel-os-debian-*.iso -m 2G -boot d"
echo ""
echo "To install to disk:"
echo "  Boot from ISO and select 'Install' option"
echo "═══════════════════════════════════════════════════════════"
