# 👻 Contributing to SpecterOS

Thank you for your interest in contributing to SpecterOS! We welcome contributions from the community to help build a more privacy-focused future.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Building the ISO](#building-the-iso)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Communication](#communication)

---

## 🤝 Code of Conduct

- Be respectful and inclusive
- Privacy is a fundamental right - keep it at the core of all decisions
- Welcome newcomers and help them learn
- No harassment or discrimination of any kind

---

## 🚀 Getting Started

### 1. Fork the Repository

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/Specter-OS.git
cd Specter-OS

# Add upstream remote
git remote add upstream https://github.com/Sskibls/Specter-OS.git
```

### 2. Find Issues

Check our [Issues page](https://github.com/Sskibls/Specter-OS/issues) for:
- 🐛 Bugs to fix
- ✨ Features to implement
- 📝 Documentation improvements
- 🧪 Testing help needed

---

## 💡 How to Contribute

### Code Contributions

- **Daemons**: Security services (policyd, shardd, netd, airlockd, etc.)
- **UI**: Desktop environment, TUI, or web interface
- **Build System**: ISO building, packaging scripts
- **Testing**: Unit tests, integration tests, E2E tests

### Non-Code Contributions

- **Documentation**: Improve docs, tutorials, guides
- **Translation**: Localize SpecterOS to your language
- **Design**: UI/UX improvements, logos, icons
- **Testing**: Report bugs, test releases, verify hardware compatibility
- **Community**: Help others, moderate discussions, spread the word

---

## 🛠️ Development Setup

### Prerequisites

```bash
# Debian/Ubuntu
sudo apt update
sudo apt install -y build-essential curl wget git pkg-config

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# For GUI development
sudo apt install -y libgtk-4-dev libgraphene-1.0-dev libpango1.0-dev libcairo2-dev
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/Sskibls/Specter-OS.git
cd Specter-OS

# Build all components
cargo build --release

# Run tests
cargo test

# Run TUI (for testing)
./target/release/specteros-tui
```

---

## 📀 Building the ISO

### Requirements

- Debian 13 (Trixie) or Ubuntu 22.04+
- 10GB+ free disk space
- Root access
- Required tools: `debootstrap`, `genisoimage`, `xorriso`, `grub-mkrescue`

### Build Steps

```bash
cd distro/scripts
sudo bash build-debian-iso.sh
```

Output: `distro/specteros-os-0.2.0.iso`

### Test in VM

```bash
# QEMU/KVM
qemu-system-x86_64 -cdrom specteros-os-0.2.0.iso -m 4096 -boot d

# VirtualBox
VBoxManage createvm --name "SpecterOS" --register
VBoxManage storagectl "SpecterOS" --name "IDE Controller" --add ide
VBoxManage storageattach "SpecterOS" --storagectl "IDE Controller" --port 0 --device 0 --type dvddrive --medium specteros-os-0.2.0.iso
```

---

## 📤 Submitting Changes

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-123
```

### 2. Make Changes

- Follow existing code style
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

### 3. Commit Messages

```
feat: add network kill switch shortcut

- Add Ctrl+Alt+K global shortcut
- Trigger guardian panic mode on activation
- Add visual indicator when kill switch is active

Closes #45
```

**Types:**
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `style:` Code style changes (formatting, etc.)
- `refactor:` Code refactoring
- `test:` Adding tests
- `chore:` Build/config changes

### 4. Create Pull Request

```bash
# Push your branch
git push origin feature/your-feature-name

# Then create PR on GitHub
# https://github.com/Sskibls/Specter-OS/pulls
```

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Tests pass locally
- [ ] Tested in VM
- [ ] ISO builds successfully

## Checklist
- [ ] Code follows project guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
```

---

## 📐 Coding Standards

### Rust Code

```rust
// Use descriptive variable names
let network_policy_service = NetworkPolicyService::new();

// Handle errors properly
match result {
    Ok(value) => value,
    Err(e) => return Err(GuardianError::NetworkOperationFailed(e.to_string())),
}

// Add doc comments for public APIs
/// Panic Mode: Emergency containment
/// - Kills all network interfaces
/// - Locks all persona shards
pub fn panic(&mut self) -> Result<(), GuardianError> {
    // Implementation
}
```

### Shell Scripts

```bash
#!/bin/bash
# Use descriptive variable names
DEBIAN_SUITE="trixie"
ARCH="amd64"

# Check for errors
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root"
    exit 1
fi
```

---

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suite

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test milestone4_integration_tests

# Specific test
cargo test test_panic_mode
```

### Manual Testing Checklist

- [ ] ISO boots in VM
- [ ] All daemons start correctly
- [ ] Desktop environment loads
- [ ] Network kill switch works
- [ ] Shard isolation functions
- [ ] Audit logging captures events
- [ ] No console errors on boot

---

## 💬 Communication

### Channels

- **GitHub Issues**: Bug reports, feature requests
- **GitHub Discussions**: General questions, ideas
- **Telegram**: Community chat (link in README)

### Getting Help

- Check existing issues before creating new ones
- Search documentation first: https://specter-os.web.app/docs
- Be patient - maintainers are volunteers
- Provide detailed information in bug reports

---

## 🏆 Recognition

Contributors will be:
- Listed in the README
- Mentioned in release notes
- Added to the contributors page on the website

---

## 📜 License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License (same as SpecterOS).

---

## 🙏 Thank You!

Every contribution, no matter how small, helps make SpecterOS better. Together, we're building a more privacy-respecting future.

**Your Privacy. Our Mission.** 👻
