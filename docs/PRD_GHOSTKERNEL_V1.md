# PhantomKernel OS: Product Requirements & Architecture (v1.0)

**Date:** 2026-03-05
**Status:** DRAFT
**Version:** 1.0
**Context:** Privacy-First Defensive OS (Debian Lite / Fedora Full)

---

## 1. Executive Summary

PhantomKernel OS is a defensive operating system designed to protect high-risk users (journalists, activists, researchers) from surveillance, data leakage, and physical seizure.

To maximize accessibility and usability without compromising security, PhantomKernel will ship in two editions:
1.  **PhantomKernel Lite (Debian):** Optimized for older hardware, air-gapped cold storage, and maximum stability.
2.  **PhantomKernel Full (Fedora):** Optimized for modern workstations, developer workflows, and bleeding-edge hardware support.

Both editions share a unified runtime environment called **PhantomKernel Core**, which enforces the OS's core invariants: persona isolation, capability-based permissions, and lawful defensive posture.

---

## 2. Edition Strategy

We employ a "Shared Core, Divergent Base" strategy. The `PhantomKernel` middleware abstracts the underlying OS differences.

| Feature | PhantomKernel **Lite** | PhantomKernel **Full** |
| :--- | :--- | :--- |
| **Base Distro** | Debian Stable (Slim) | Fedora Workstation |
| **Kernel** | Linux LTS (Hardened) | Linux Mainline (Hardened/Zen) |
| **Target Hardware** | 4GB+ RAM, Dual Core, Legacy BIOS/UEFI | 16GB+ RAM, Quad Core, UEFI + TPM 2.0 |
| **Default UI** | XFCE (Custom Themed) | GNOME (Stripped) or Sway (Tiling) |
| **Isolation Tech** | `systemd-nspawn` / `bubblewrap` (Container) | `podman` + KVM (VM-like containers) |
| **Update Cycle** | Slow, Security-only | Rolling/Frequent |
| **Package Format** | `.deb` (apt) | `.rpm` (dnf) |
| **Primary Use Case** | Burner laptops, Cold Storage, Low-bandwidth | Daily Driver, Dev Workstation, Media Editing |

---

## 3. Shared Core Architecture: "PhantomKernel Core"

The **PhantomKernel Core** is a suite of daemons that run as root/system services. They form the "PhantomKernel" identity. They are distro-agnostic (written in Go/Rust) and interact with the host OS via an Abstraction Layer.

### 3.1 Component Diagram

```text
[ User Space (UI / CLI) ]
       ^      ^
       | gRPC |
[ PhantomKernel Middleware ] ------------------------------+
|  phantomkernel-policyd  (Brain: Perms & Auth)                |
|  phantomkernel-shardd   (Isolation Manager)                  |
|  phantomkernel-netd     (Network & Route Manager)            |
|  phantomkernel-airlockd (Sanitization Gate)                  |
+--------------------------------------------------------+
       | Calls (HAL)
[ Hardware/Distro Abstraction Layer (HAL) ]
|  /usr/libexec/ghost/adapter-{net, iso, pkg}            |
+--------------------------------------------------------+
       |
[ Host System (Debian or Fedora) ]
|  systemd | nftables | cryptsetup | podman/nspawn       |
+--------------------------------------------------------+
```

### 3.2 Core Daemons

1.  **`phantomkernel-policyd`**: The central authority.
    *   **Responsibility:** Issues capability tokens. Validates every privileged action.
    *   **Storage:** Encrypted SQLite DB (Policy History).
2.  **`phantomkernel-shardd`**: The isolation engine.
    *   **Responsibility:** Spins up "Personas" (Work, Anon, etc.).
    *   **Lite impl:** Manages `systemd-nspawn` containers.
    *   **Full impl:** Manages `podman` containers or KVM slices.
3.  **`phantomkernel-netd`**: The network controller.
    *   **Responsibility:** Enforces per-shard routing (VPN, Tor, Clearnet). Implements the "Kill Switch".
4.  **`phantomkernel-airlockd`**: The transfer agent.
    *   **Responsibility:** Moves files between shards.
    *   **Action:** Strips metadata (EXIF), converts dangerous formats, scans for malware.

---

## 4. Personas & User Journeys

The OS enforces rigid "Persona Shards". Users cannot create arbitrary global workspaces; they must work within a shard.

### 4.1 Standard Personas

*   **Admin/Host:** Minimal maintenance mode. No browsing.
*   **Work:** Trusted identity. Standard VPN. Persistent storage.
*   **Anon:** Hostile identity. Tor-enforced. Ephemeral storage (optional).
*   **Burner:** One-time use. Wipes on exit.
*   **Lab:** Malware analysis. Network disconnected by hardware/software.

### 4.2 User Journey: "The Whistleblower"
1.  **Receive:** User boots **Lite** on an old laptop. Logs into **Anon** shard.
2.  **Download:** Downloads a leaked document via Tor Browser.
3.  **Sanitize:** User attempts to move file to **Work** shard.
4.  **Airlock:** `phantomkernel-airlockd` intercepts. Strips metadata. Converts `.docx` to `.pdf`.
5.  **Store:** Clean PDF appears in **Work** shard. Original file stays in **Anon** (or is wiped).

### 4.3 User Journey: "The Panic"
1.  **Trigger:** User presses global hotkey (e.g., `Meta+Shift+Esc`).
2.  **Action:** `phantomkernel-policyd` revokes all active tokens.
3.  **Network:** `phantomkernel-netd` drops all interfaces.
4.  **Storage:** `phantomkernel-shardd` unmounts and LUKS-suspends all encrypted volumes.
5.  **UI:** Screen switches to a harmless "decoy" desktop (Mask Mode).

---

## 5. API & IPC Contracts

Communication uses **gRPC** over UNIX Domain Sockets. Authenticated via peer credentials (`SO_PEERCRED`).

### 5.1 Policyd Contract
```protobuf
service PolicyService {
  // Request a temporary capability (e.g., "Access Mic", "Open Network")
  rpc RequestCapability(CapRequest) returns (CapToken);
  
  // Validate a token provided by another service
  rpc ValidateToken(TokenCheck) returns (ValidationStatus);
  
  // Emergency Lockdown
  rpc TriggerPanic(PanicContext) returns (PanicAck);
}
```

### 5.2 Shardd Contract
```protobuf
service ShardService {
  // Start a persona environment
  rpc StartShard(ShardID, CapToken) returns (SessionHandle);
  
  // Stop and wipe (if burner)
  rpc StopShard(SessionHandle) returns (Status);
}
```

### 5.3 Airlockd Contract
```protobuf
service AirlockService {
  // Request file move
  rpc TransferFile(TransferRequest) returns (TransferJobId);
  
  // Get status of sanitization
  rpc GetJobStatus(TransferJobId) returns (JobResult);
}
```

---

## 6. Development Roadmap (180 Days)

### Phase 1: Foundation (Days 0-60)
*   **Goal:** Bootable ISOs for both Lite and Full with CLI-only PhantomKernel Core.
*   **Milestones:**
    *   Dev: `phantomkernel-policyd` and `phantomkernel-shardd` MVP.
    *   Lite: Debian minimal build script with `systemd-nspawn` integration.
    *   Full: Fedora Kickstart file with `podman` integration.
    *   Shared: Define HAL interface scripts (`/usr/libexec/ghost/*`).

### Phase 2: The Airlock & Network (Days 61-120)
*   **Goal:** Functional isolation and secure data transfer.
*   **Milestones:**
    *   Dev: `phantomkernel-netd` with nftables/iptables abstraction.
    *   Dev: `phantomkernel-airlockd` with basic exiftool/pdf-convert integration.
    *   UI: Basic GTK/Qt tray indicator for active shard.

### Phase 3: UI & Polish (Days 121-180)
*   **Goal:** Beta Release Candidate.
*   **Milestones:**
    *   UI: "Ghost Shell" (Custom launcher/dashboard).
    *   Sec: Audit of "Panic Mode" reliability.
    *   Distro: Installer (Calamares) customization for both editions.
    *   Docs: User Handbook.

---

## 7. Module Map & Responsibilities

| Module | Language | Maintainer Role | Criticality |
| :--- | :--- | :--- | :--- |
| **PhantomKernel Core** | Rust/Go | Core Systems Engineer | High |
| **Host Abstraction (HAL)** | Bash/Python | Distro Maintainer | High |
| **Ghost Shell (UI)** | GTK/Rust or Qt | UI Engineer | Medium |
| **Installer** | Calamares/Python | Distro Maintainer | Medium |
| **Kernel Hardening** | C/Kconfig | Security Researcher | Critical |

---

## 8. Staffing Plan

To execute this 6-month plan, the following virtual team structure is proposed:

1.  **Lead Architect (1):** Oversees API contracts and security invariants.
2.  **Core Systems Developers (2):** One for `policyd/shardd`, one for `netd/airlockd`. (Rust/Go expertise).
3.  **Distro Maintainers (2):**
    *   *Debian Specialist:* Handles Lite packaging, `apt` wrapping, legacy hardware testing.
    *   *Fedora Specialist:* Handles Full packaging, `selinux` policies, modern hardware integration.
4.  **UI/UX Designer (1):** Defines the "fsociety" theme and "Panic" workflows.
5.  **QA/Security Researcher (1):** Constant adversarial testing (breaking out of shards, bypassing airlock).

---

## 9. Risks & Mitigations

| Risk | Mitigation |
| :--- | :--- |
| **Fragmentation:** Lite and Full diverge too much. | Strict adherence to `PhantomKernel` API. HAL scripts handle *all* differences. |
| **Leakage:** Shards leak data via side-channels. | Use `bubblewrap` (Lite) and `KVM` (Full) defaults. Audit audio/video subsystems (PipeWire isolation). |
| **Complexity:** Users bypass security for convenience. | "Privacy Defaults" – make the secure way the *easy* way (e.g., one-click Airlock). |
| **Hardware:** Drivers missing on Lite. | Explicit "Certified Hardware" list. Fallback to generic VESA/wifi drivers where possible. |

---

## 10. Acceptance Criteria (v1.0)

1.  **Universal Boot:** Installer works on Reference Laptop A (Old ThinkPad) and B (New Framework).
2.  **Shard Seal:** A process in "Anon" cannot ping "Work" network or read "Work" files.
3.  **Airlock Integrity:** A `.jpg` with GPS EXIF data transferred to "Work" arrives with EXIF stripped.
4.  **Panic Reliability:** Panic key sequence instantly kills network and unmounts crypto-volumes on both editions.
