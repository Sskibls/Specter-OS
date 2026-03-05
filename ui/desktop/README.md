# PhantomKernel Desktop Environment

A secure, privacy-focused Wayland desktop environment for PhantomKernel OS.

## Features

### 🔐 Security-First Design
- **Wayland Compositor** - Modern, secure display server (no X11 vulnerabilities)
- **Shard Isolation** - Visual separation between persona shards
- **Privacy Filter** - Quick blur/obscure screen content
- **No Screenshot** - Screenshot functionality disabled by default

### 🎨 Themes
- **Default** - Clean, professional appearance
- **Fsociety** - Terminal-centric hacker aesthetic
- **Allsafe** - Corporate security look
- **DarkArmy** - High-contrast strict mode

### 📦 Built-in Applications

| Application | Purpose |
|-------------|---------|
| **Secure File Manager** | Shard-aware file browsing with metadata sanitization |
| **Network Monitor** | Real-time traffic visualization, leak detection |
| **Shard Manager** | Visual shard lifecycle controls |
| **Privacy Settings** | Security configuration UI |
| **System Panel** | Status indicators, quick controls |

### 🚨 Emergency Modes
- **Panic Mode** - Kill network, lock all shards, clear secrets
- **Mask Mode** - Switch to decoy desktop environment
- **Travel Mode** - Ephemeral sessions, no persistent storage

## Architecture

```
phantomkernel-desktop/
├── src/
│   ├── main.rs          # Application entry point
│   ├── shell.rs         # Desktop shell, workspace management
│   ├── panel.rs         # Top panel, system tray
│   ├── wallpaper.rs     # Dynamic shard-aware wallpaper
│   └── apps/
│       ├── mod.rs
│       ├── file_manager.rs   # Secure file browser
│       ├── network_monitor.rs # Network status & leak detection
│       ├── shard_manager.rs  # Shard lifecycle UI
│       └── settings.rs       # Privacy settings
├── style.css            # GTK stylesheet (all themes)
└── Cargo.toml
```

## Building

```bash
# Build the desktop environment
cargo build -p phantomkernel-desktop --release

# Run (requires Wayland session)
./target/release/phantomkernel-desktop
```

## Dependencies

- GTK 4.8+
- Wayland protocols
- PhantomKernel IPC libraries

## Integration

The desktop integrates with PhantomKernel OS services:

| Service | Integration |
|---------|-------------|
| `phantomkernel-shardd` | Workspace per shard |
| `phantomkernel-netd` | Network status display |
| `phantomkernel-policyd` | Permission prompts |
| `phantomkernel-airlockd` | File transfer dialogs |
| `phantomkernel-guardian` | Emergency mode triggers |
| `phantomkernel-auditd` | Action logging |

## Security Considerations

1. **No X11** - X11 has inherent security vulnerabilities (keylogging, screen scraping)
2. **Wayland Only** - Each window is isolated, cannot read other windows
3. **Clipboard Isolation** - Per-shard clipboards, no cross-shard leakage
4. **Screen Recording Blocked** - Prevents unauthorized capture
5. **Secure Input** - Password fields use secure input method

## Theme Customization

Edit `style.css` or create theme variants in `themes/`:

```bash
ui/desktop/themes/
├── fsociety/
│   ├── style.css
│   └── config.toml
├── allsafe/
│   ├── style.css
│   └── config.toml
└── darkarmy/
    ├── style.css
    └── config.toml
```

## Privacy Indicators

The desktop shows real-time privacy status:

- 🟢 **Protected** - All security measures active
- 🟡 **Warning** - Potential privacy risk detected
- 🔴 **Compromised** - Active leak or security issue

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Super + P` | Activate panic mode |
| `Super + M` | Toggle mask mode |
| `Super + T` | Toggle travel mode |
| `Super + L` | Quick lock screen |
| `Super + [1-4]` | Switch to shard 1-4 |
| `Super + K` | Toggle kill switch |

## License

Apache 2.0 - PhantomKernel OS Project
