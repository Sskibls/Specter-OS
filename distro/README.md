# PhantomKernel OS - Fedora-based Linux Distribution

A real, bootable Linux distribution based on Fedora with GNOME desktop and privacy features.

## 📦 What You Get

- **Real Fedora 39 base** with GNOME desktop
- **PhantomKernel security daemons** pre-installed
- **Persona Shards** for identity isolation
- **Network kill switch** and privacy controls
- **Audit logging** and tamper detection
- **SELinux** mandatory access control

## 🛠️ Build Requirements

On a Fedora 39+ system:

```bash
sudo dnf install -y \
    lorax \
    composer-cli \
    pykickstart \
    anaconda-dracut \
    createrepo_c
```

## 🚀 Quick Build

### Method 1: Direct Build (Recommended)

```bash
cd /path/to/phantomkernel-os
sudo ./distro/scripts/build-fedora.sh
```

This creates:
- `output/phantomkernel-os.iso` - Bootable ISO
- `output/install-phantomkernel.sh` - Installation script

### Method 2: Docker Build (Reproducible)

```bash
cd distro/
docker build -t phantomkernel-builder .
docker run --rm -it \
    -v /dev:/dev \
    -v $(pwd)/output:/output \
    phantomkernel-builder
```

### Method 3: Manual Lorax Build

```bash
sudo lorax \
    -p PhantomKernel \
    -v 0.1.0 \
    -r 1 \
    --releasever=39 \
    --source=distro/phantomkernel.ks \
    --output=output \
    /path/to/phantomkernel-os.iso
```

## 💻 Test with QEMU

```bash
qemu-system-x86_64 \
    -cdrom output/phantomkernel-os.iso \
    -m 4096 \
    -boot d \
    -cpu host \
    -enable-kvm \
    -vga virtio \
    -display gtk \
    -usb \
    -device usb-tablet
```

### With VNC (headless)

```bash
qemu-system-x86_64 \
    -cdrom output/phantomkernel-os.iso \
    -m 4096 \
    -boot d \
    -vnc :0
```

Then connect with VNC client to `localhost:5900`

## 📀 Install to Disk

### Graphical Installer

1. Boot from ISO
2. Select "Install PhantomKernel OS"
3. Follow Anaconda installer
4. Set up user account
5. Reboot

### Automated Install

```bash
sudo ./output/install-phantomkernel.sh
```

## 🖥️ Desktop Environment

PhantomKernel OS uses **GNOME 45** with privacy extensions:

### Pre-installed Applications

| Category | Applications |
|----------|-------------|
| **Internet** | Firefox (hardened), Thunderbird |
| **Office** | LibreOffice, Evince PDF |
| **Utilities** | Files, Text Editor, Terminal |
| **Security** | PhantomKernel TUI, Settings, Network Monitor |
| **Multimedia** | Videos, Music, Image Viewer |

### PhantomKernel Apps

- **phantomkernel-tui** - Terminal dashboard (auto-starts in terminal)
- **phantomkernel-shell** - Interactive CLI
- **Shard Manager** - GNOME app for shard control
- **Network Monitor** - System tray network status
- **Privacy Settings** - GNOME settings extension

## 🔐 Default Security Settings

| Feature | Status |
|---------|--------|
| SELinux | Enforcing |
| Firewall | Enabled (strict) |
| Full Disk Encryption | Enabled (LUKS2) |
| Secure Boot | Required |
| TPM Integration | Enabled |
| Audit Logging | Active |

## 🎯 Persona Shards

Four isolated environments created by default:

| Shard | Purpose | Network |
|-------|---------|---------|
| **Work** | Professional identity | Direct |
| **Anon** | Anonymous browsing | Tor |
| **Burner** | Temporary sessions | Tor + Rotation |
| **Lab** | Security testing | Isolated VLAN |

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Super + P` | Panic mode |
| `Super + M` | Mask mode |
| `Super + T` | Travel mode |
| `Super + K` | Kill switch |
| `Super + L` | Lock screen |
| `Alt + F4` | Close window |
| `Super` | Activities overview |

## 📁 File Structure

```
/
├── boot/          # Bootloader and kernels
├── etc/
│   └── phantomkernel/   # Configuration
├── home/          # User directories (per-shard)
├── opt/
│   └── phantomkernel/   # PhantomKernel binaries
├── usr/
│   ├── bin/       # System binaries
│   └── share/     # Data files
└── var/           # Variable data
```

## 🔧 Configuration

Main config: `/etc/phantomkernel/config.toml`

```toml
[general]
theme = "fsociety"
log_level = "info"

[shards]
default_shards = ["work", "anon", "burner", "lab"]

[network]
kill_switch_default = false
dns_over_https = true
```

## 🐛 Troubleshooting

### Boot Issues

Try nomodeset:
```
phantomkernel-os (debug) -> add nomodeset to kernel params
```

### No Network

```bash
sudo nmcli networking on
sudo systemctl restart NetworkManager
```

### PhantomKernel Services

```bash
systemctl status phantomkernel-*
journalctl -u phantomkernel-shardd -f
```

## 📝 Version Info

- **Base:** Fedora 39
- **Desktop:** GNOME 45
- **Kernel:** 6.5+
- **PhantomKernel:** 0.1.0

## 📄 License

Apache 2.0 - PhantomKernel OS Project

## 🤝 Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## 🔗 Links

- Website: https://phantomkernel.org
- Docs: https://phantomkernel.org/docs
- Issues: https://github.com/phantomkernel/os/issues
