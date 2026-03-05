# 🚀 PhantomKernel OS - Debian 12 Live ISO

**REAL Debian-based Linux distribution with XFCE desktop environment**

---

## 📋 What This Is

- ✅ **Real Debian 12 (Bookworm)** base system
- ✅ **Bootable ISO** for real hardware or VMs
- ✅ **XFCE Desktop** with full GUI applications
- ✅ **PhantomKernel security** features integrated
- ✅ **Live mode** (try without installing) + **Installer**

---

## 🛠️ Build Options

### Option 1: Direct Build on Debian/Ubuntu (Recommended)

**Requirements:** Debian 12+ or Ubuntu 22.04+ with 10GB+ free space

```bash
# Install build tools
sudo apt update
sudo apt install -y debootstrap genisoimage xorriso grub2-common mtools squashfs-tools

# Build the ISO
cd /path/to/phantomkernel-os/distro
sudo ./scripts/build-debian-live.sh

# Output: ./output/phantomkernel-os-debian-YYYYMMDD.iso
```

### Option 2: Docker Build (Works Anywhere)

**Requirements:** Docker installed

```bash
cd /path/to/phantomkernel-os/distro

# Build the builder image
docker build -t phantomkernel-builder .

# Run the build
docker run --rm -v $(pwd)/output:/output --privileged phantomkernel-builder

# Output: ./output/phantomkernel-os-debian-YYYYMMDD.iso
```

### Option 3: Cloud VM Build

Build on a cloud VM (AWS, GCP, DigitalOcean, etc.):

```bash
# Create Ubuntu 22.04 VM (2 CPU, 4GB RAM, 40GB disk)
# SSH into VM

# Clone and build
git clone https://github.com/phantomkernel/os.git
cd os/distro
sudo ./scripts/build-debian-live.sh

# Download ISO
scp user@vm:~/os/output/phantomkernel-os-*.iso ./

# Terminate VM
```

---

## 🖥️ Testing the ISO

### QEMU (Local Testing)

```bash
# With GUI window
qemu-system-x86_64 \
    -cdrom output/phantomkernel-os-debian-*.iso \
    -m 4096 \
    -boot d \
    -cpu host \
    -enable-kvm \
    -vga virtio \
    -usb -device usb-tablet

# With VNC (remote access)
qemu-system-x86_64 \
    -cdrom output/phantomkernel-os-debian-*.iso \
    -m 4096 \
    -boot d \
    -vnc :0

# Then connect VNC client to: localhost:5900
```

### VirtualBox

1. Create new VM → Linux → Debian (64-bit)
2. Memory: 4096 MB
3. Disk: Create new VDI (64GB)
4. Settings → Storage → Choose ISO
5. Start VM

### VMware

1. Create new VM → Typical → Installer disc image
2. Select: `phantomkernel-os-debian-*.iso`
3. OS: Debian 12.x 64-bit
4. Memory: 4GB, Disk: 64GB
5. Finish and power on

---

## 📦 What's Included

### Base System
| Component | Version |
|-----------|---------|
| Debian | 12 (Bookworm) |
| Linux Kernel | 6.1 LTS |
| Desktop | XFCE 4.18 |
| Display Server | X.Org |
| Bootloader | GRUB2 |

### Desktop Applications
| Category | Applications |
|----------|-------------|
| **Web** | Firefox ESR |
| **Email** | Thunderbird |
| **Office** | LibreOffice (Writer, Calc, Impress) |
| **Files** | Thunar File Manager |
| **Terminal** | Terminator |
| **Editor** | Mousepad |
| **Media** | Videos, Music, Image Viewer |
| **System** | Settings, Monitor, Screenshot |

### PhantomKernel Components
| Binary | Purpose |
|--------|---------|
| `phantomkernel-tui` | Terminal dashboard |
| `phantomkernel-shell` | Interactive CLI |
| `phantomkernel-shardd` | Persona shard manager |
| `phantomkernel-netd` | Network policy daemon |
| `phantomkernel-policyd` | Permission broker |
| `phantomkernel-auditd` | Audit logging |
| `phantomkernel-guardian` | Emergency modes |
| `phantomkernel-updated` | A/B updates |
| `gkctl` | Control utility |

### Security Features
- ✅ SELinux/AppArmor mandatory access control
- ✅ Firewall (iptables/nftables)
- ✅ Full disk encryption option (LUKS2)
- ✅ Secure boot support
- ✅ Audit logging framework
- ✅ Network kill switch
- ✅ DNS over HTTPS
- ✅ Persona shard isolation

---

## ⌨️ Default Credentials

| Account | Username | Password |
|---------|----------|----------|
| Live Session | `user` | `user` |
| Root (Live) | `root` | `phantomkernel` |
| Installed | Set during install | - |

---

## 🎮 Keyboard Shortcuts

### PhantomKernel Shortcuts
| Shortcut | Action |
|----------|--------|
| `Super + P` | Activate PANIC mode |
| `Super + M` | Activate MASK mode |
| `Super + T` | Toggle TRAVEL mode |
| `Super + K` | Toggle network kill switch |
| `Super + L` | Lock screen |

### XFCE Shortcuts
| Shortcut | Action |
|----------|--------|
| `Super` | Applications menu |
| `Super + A` | Show applications |
| `Super + D` | Show desktop |
| `Super + E` | File manager |
| `Super + T` | Terminal |
| `Alt + F4` | Close window |
| `Alt + Tab` | Switch windows |
| `Ctrl + Alt + Del` | Logout dialog |

---

## 📀 Installation

### From Live ISO to Disk

1. Boot from ISO
2. Double-click "Install PhantomKernel" on desktop
3. Follow installer wizard:
   - Select language, keyboard
   - Partition disk (or use entire disk)
   - Set up user account
   - Install bootloader
4. Reboot and remove ISO

### Automated Install

```bash
# After booting live ISO
sudo ./install-to-disk.sh
```

---

## 🔧 Post-Installation

### Update System
```bash
sudo apt update && sudo apt upgrade -y
```

### Enable PhantomKernel Services
```bash
sudo systemctl enable phantomkernel-shardd
sudo systemctl enable phantomkernel-netd
sudo systemctl enable phantomkernel-policyd
sudo systemctl start phantomkernel-*
```

### Configure Firewall
```bash
sudo ufw enable
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
```

---

## 🐛 Troubleshooting

### Boot Issues

**Try failsafe mode:**
- Select "PhantomKernel OS (Live - Failsafe)" from boot menu

**Add nomodeset:**
- Press `e` at boot menu
- Add `nomodeset` to linux line
- Press `F10` to boot

### No Network

```bash
# Restart NetworkManager
sudo systemctl restart NetworkManager

# Check connection
nmcli device status
```

### Graphics Issues

```bash
# Install additional drivers
sudo apt install firmware-misc-nonfree
sudo update-initramfs -u
```

### PhantomKernel Services Not Starting

```bash
# Check status
systemctl status phantomkernel-*

# View logs
journalctl -u phantomkernel-shardd -f

# Restart services
sudo systemctl restart phantomkernel-*
```

---

## 📊 System Requirements

### Minimum
- CPU: 2 cores (x86_64)
- RAM: 2 GB
- Disk: 20 GB
- Boot: USB or DVD

### Recommended
- CPU: 4 cores
- RAM: 4 GB
- Disk: 64 GB SSD
- Network: Ethernet or Wi-Fi

---

## 🔗 File Structure

```
distro/
├── scripts/
│   ├── build-debian-live.sh    # Main ISO builder
│   └── build-debian-iso.sh     # Alternative builder
├── Dockerfile                   # Docker build config
├── composer.toml               # Package manifest
└── README.md                   # This file

output/                          # Build outputs
├── phantomkernel-os-debian-*.iso  # Bootable ISO
├── phantomkernel-os-debian-*.iso.sha256
└── install-to-disk.sh          # Installer script
```

---

## 📄 License

Apache 2.0 - PhantomKernel OS Project

---

## 🤝 Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md)

---

## 🔗 Links

- Website: https://phantomkernel.org
- Documentation: https://phantomkernel.org/docs
- Issues: https://github.com/phantomkernel/os/issues
- Releases: https://github.com/phantomkernel/os/releases
