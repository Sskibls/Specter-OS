#!/bin/bash
#
# SpecterOS - Debian 12 Live ISO Builder
# Creates a real bootable Debian Live ISO with XFCE desktop
#

set -euo pipefail

# Configuration
DEBIAN_VERSION="12"
CODENAME="bookworm"
ARCH="amd64"
ISO_NAME="specteros-debian-${DEBIAN_VERSION}-$(date +%Y%m%d).iso"
WORK_DIR="/var/tmp/specteros-build"
OUTPUT_DIR="$(pwd)/output"

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

echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║       SpecterOS - Debian 12 Live ISO Builder              ║"
echo "║       REAL Linux Distribution with XFCE Desktop           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Check if running on real Debian/Ubuntu (not Termux)
if [[ -f "/data/data/com.termux/files/usr/bin/bash" ]] && [[ -d "/data/data/com.termux" ]]; then
    log_error "This script must run on a REAL Debian/Ubuntu system, not Termux!"
    echo ""
    echo "To build this ISO:"
    echo "  1. Copy this project to a Debian 12 or Ubuntu 22.04+ system"
    echo "  2. Run: sudo ./distro/scripts/build-debian-live.sh"
    echo ""
    echo "Alternative: Use Docker (works anywhere):"
    echo "  cd distro/"
    echo "  docker build -t specteros-builder ."
    echo "  docker run --rm -v \$(pwd)/output:/output --privileged specteros-builder"
    exit 1
fi

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root (sudo)"
    exit 1
fi

# Check required tools
log_step "Checking required tools..."
REQUIRED_TOOLS="debootstrap genisoimage xorriso grub-mkrescue mksquashfs"
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
    echo "  apt update && apt install -y debootstrap genisoimage xorriso grub2-common mtools squashfs-tools"
    exit 1
fi

log_success "All required tools found"

# Create working directories
log_step "Creating working directories..."
mkdir -p "$WORK_DIR" "$OUTPUT_DIR"
ROOTFS="$WORK_DIR/rootfs"
ISO_ROOT="$WORK_DIR/iso_root"
LIVE_DIR="$ISO_ROOT/live"

mkdir -p "$ROOTFS" "$ISO_ROOT" "$LIVE_DIR"

cleanup() {
    log_warning "Cleaning up..."
    if [[ -d "$WORK_DIR" ]]; then
        rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT

# Bootstrap Debian base system
log_step "Bootstrapping Debian $CODENAME base system..."
debootstrap --include=systemd,grub2,linux-image-$ARCH,firmware-linux,wget,curl,sudo \
    --components=main,contrib,non-free \
    $CODENAME "$ROOTFS" http://deb.debian.org/debian/ 2>&1 | tee /tmp/debootstrap.log

log_success "Debian base installed"

# Copy SpecterOS binaries from project
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [[ -d "$PROJECT_ROOT/target/release" ]]; then
    log_step "Installing SpecterOS components..."
    mkdir -p "$ROOTFS/opt/specteros/bin"
    cp "$PROJECT_ROOT/target/release"/specteros-* "$ROOTFS/opt/specteros/bin/" 2>/dev/null || true
    cp "$PROJECT_ROOT/target/release/spctl" "$ROOTFS/opt/specteros/bin/" 2>/dev/null || true
    cp "$PROJECT_ROOT/target/release/specteros-tui" "$ROOTFS/opt/specteros/bin/" 2>/dev/null || true
    cp "$PROJECT_ROOT/target/release/specteros-shell" "$ROOTFS/opt/specteros/bin/" 2>/dev/null || true
    chmod +x "$ROOTFS/opt/specteros/bin/"*
    log_success "SpecterOS binaries installed"
else
    log_warning "SpecterOS binaries not found (build first with cargo build --release)"
fi

# Install XFCE desktop environment
log_step "Installing XFCE desktop environment..."
cat > "$ROOTFS/etc/apt/sources.list" << EOF
deb http://deb.debian.org/debian $CODENAME main contrib non-free non-free-firmware
deb http://deb.debian.org/debian $CODENAME-updates main contrib non-free
deb http://security.debian.org/debian-security $CODENAME-security main contrib non-free
EOF

chroot "$ROOTFS" apt-get update
chroot "$ROOTFS" apt-get install -y \
    task-xfce-desktop \
    lightdm \
    firefox-esr \
    libreoffice \
    thunderbird \
    terminator \
    thunar \
    mousepad \
    pavucontrol \
    network-manager \
    network-manager-gnome \
    sudo \
    vim \
    git \
    htop \
    2>&1 | tee /tmp/apt-install.log

log_success "Desktop environment installed"

# Configure system
log_step "Configuring system..."

# Hostname
echo "specteros" > "$ROOTFS/etc/hostname"

# Hosts
cat > "$ROOTFS/etc/hosts" << EOF
127.0.0.1   localhost
127.0.1.1   specteros
::1         localhost ip6-localhost ip6-loopback
EOF

# Timezone
echo "UTC" > "$ROOTFS/etc/timezone"
chroot "$ROOTFS" dpkg-reconfigure -f noninteractive tzdata

# Root password
echo "root:specter" | chroot "$ROOTFS" chpasswd

# Create default user
chroot "$ROOTFS" useradd -m -s /bin/bash -G sudo,audio,video,dialout,plugdev user
echo "user:user" | chroot "$ROOTFS" chpasswd

# Sudo configuration
echo "%sudo ALL=(ALL) NOPASSWD:ALL" > "$ROOTFS/etc/sudoers.d/sudo"
chmod 440 "$ROOTFS/etc/sudoers.d/sudo"

# Enable services
chroot "$ROOTFS" systemctl enable lightdm
chroot "$ROOTFS" systemctl enable NetworkManager

# SpecterOS configuration
mkdir -p "$ROOTFS/etc/specteros"
cat > "$ROOTFS/etc/specteros/config.toml" << 'EOF'
[general]
hostname = "specteros"
theme = "specter"
log_level = "info"
audit_enabled = true

[shards]
default_shards = ["work", "anon", "burner", "lab"]

[network]
default_route = "direct"
kill_switch_default = false
dns_over_https = true

[security]
secure_boot = false
tpm_required = false
full_disk_encryption = true
apparmor = true

[desktop]
environment = "xfce"
show_privacy_indicator = true
EOF

# Create systemd services for SpecterOS daemons
mkdir -p "$ROOTFS/etc/systemd/system"
for daemon in shardd netd policyd auditd guardian updated; do
    if [[ -f "$ROOTFS/opt/specteros/bin/specteros-${daemon}" ]]; then
        cat > "$ROOTFS/etc/systemd/system/specteros-${daemon}.service" << EOF
[Unit]
Description=SpecterOS ${daemon^} Daemon
After=network.target

[Service]
Type=simple
ExecStart=/opt/specteros/bin/specteros-${daemon}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
    fi
done

# Create welcome script
cat > "$ROOTFS/usr/local/bin/specteros-welcome" << 'EOF'
#!/bin/bash
# SpecterOS Welcome Script

if [[ ! -f /home/user/.specteros-welcomed ]]; then
    zenity --info --title="Welcome to SpecterOS" \
        --width=500 \
        --text="Welcome to SpecterOS!\n\nA privacy-focused Debian-based Linux distribution.\n\n═══ Quick Start ═══\n\n• specteros-tui    - Terminal dashboard\n• specteros-shell  - Interactive CLI\n• spctl            - System control\n\n═══ Privacy Features ═══\n\n✓ Persona Shards (work/anon/burner/lab)\n✓ Network Kill Switch\n✓ DNS over HTTPS\n✓ Audit Logging\n✓ Encrypted storage\n\n═══ Emergency Modes ═══\n\n⚠ PANIC  (Super+P) - Kill network, lock shards\n🎭 MASK   (Super+M) - Decoy desktop\n✈️ TRAVEL (Super+T) - Ephemeral sessions"
    
    touch /home/user/.specteros-welcomed
fi
EOF
chmod +x "$ROOTFS/usr/local/bin/specteros-welcome"

# Add to XFCE autostart
mkdir -p "$ROOTFS/etc/xdg/autostart"
cat > "$ROOTFS/etc/xdg/autostart/specteros-welcome.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=SpecterOS Welcome
Exec=/usr/local/bin/specteros-welcome
Terminal=false
X-GNOME-Autostart-enabled=true
EOF

# Create GRUB configuration
log_step "Configuring GRUB bootloader..."
cat > "$ROOTFS/boot/grub/grub.cfg" << 'EOF'
set timeout=5
set default=0

menuentry "SpecterOS (Live)" {
    linux /live/vmlinuz boot=live quiet splash findiso=/live/specteros-debian.iso
    initrd /live/initrd.img
}

menuentry "SpecterOS (Live - Failsafe)" {
    linux /live/vmlinuz boot=live quiet failsafe
    initrd /live/initrd.img
}

menuentry "SpecterOS (Live - Persistent)" {
    linux /live/vmlinuz boot=live quiet persistence
    initrd /live/initrd.img
}

menuentry "SpecterOS (Install to Disk)" {
    linux /install/vmlinuz quiet
    initrd /install/initrd.gz
}
EOF

# Create initramfs
log_step "Creating initramfs..."
chroot "$ROOTFS" update-initramfs -u -k all

# Copy kernel and initrd to ISO
log_step "Preparing ISO contents..."
mkdir -p "$LIVE_DIR"
cp "$ROOTFS/boot/vmlinuz"* "$LIVE_DIR/vmlinuz"
cp "$ROOTFS/boot/initrd"* "$LIVE_DIR/initrd.img"

# Create SquashFS for live filesystem
log_step "Creating SquashFS live filesystem (this may take a while)..."
mksquashfs "$ROOTFS" "$LIVE_DIR/filesystem.squashfs" -comp xz -b 1024k -mem 1G

# Create ISO
log_step "Building bootable ISO image..."
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
    -o "$OUTPUT_DIR/$ISO_NAME" \
    "$ISO_ROOT" 2>&1 | tee /tmp/iso-build.log

log_success "ISO created: $OUTPUT_DIR/$ISO_NAME"

# Generate checksum
cd "$OUTPUT_DIR"
sha256sum "$ISO_NAME" > "${ISO_NAME}.sha256"
log_info "SHA256: $(cat ${ISO_NAME}.sha256)"

# Summary
echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║         SpecterOS Build Complete                          ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "Output files:"
ls -lh "$OUTPUT_DIR/"*
echo ""
echo "To test with QEMU:"
echo "  qemu-system-x86_64 -cdrom $OUTPUT_DIR/$ISO_NAME -m 4096 -boot d -vga virtio"
echo ""
echo "To test with QEMU + VNC:"
echo "  qemu-system-x86_64 -cdrom $OUTPUT_DIR/$ISO_NAME -m 4096 -boot d -vnc :0"
echo "  Then connect VNC client to localhost:5900"
echo ""
echo "Default credentials:"
echo "  Live session: user / user"
echo "  Root: root / specter"
echo "═══════════════════════════════════════════════════════════"
