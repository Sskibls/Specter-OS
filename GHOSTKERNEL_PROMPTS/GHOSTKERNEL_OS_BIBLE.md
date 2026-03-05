# PhantomKernel OS — OS Bible (v1.0)

## 1) Vision

**PhantomKernel OS** is a privacy-first, defensive operating system designed to reduce surveillance, deanonymization, and forensic leakage in daily computing.

It is not a pentest distro clone. It is a secure, usable platform for high-risk users (journalists, researchers, activists, security engineers, and privacy-focused individuals).

Core design mantra:

> **Own your trust root. Leak nothing by default. Prove no extra data.**

---

## 2) Product Definition

- **Name:** PhantomKernel OS
- **Type:** Secure desktop OS
- **Release approach:**
  - **Phase A:** Hardened Linux base (fastest path to working product)
  - **Phase B:** Migrate critical services toward microkernel-compatible architecture
- **User experience:** Terminal-first, fsociety-inspired optional theme layer
- **Primary objective:** Defensive privacy and compartmentalization

---

## 3) Scope and Non-Scope

### In Scope
- Identity compartmentalization (Persona Shards)
- Capability-based permission system
- Strong encryption and secure boot chain
- Network privacy routing and leak controls
- Built-in metadata sanitization
- Emergency containment workflows
- Verifiable policy/audit trail

### Out of Scope
- Features optimized for unauthorized intrusion
- “Preloaded offensive toolbox” as default workflow
- Closed-source core trust components

---

## 4) Security and Privacy Principles

1. **No implicit trust:** every sensitive action needs explicit policy.
2. **No global identity context:** user actions occur inside shards.
3. **Default deny:** camera, mic, filesystem, and network access denied unless granted.
4. **Time-bound access:** permissions expire automatically.
5. **Least metadata:** scrub and minimize by default.
6. **User-owned trust roots:** boot trust controlled by user keys where possible.
7. **Local-first intelligence:** privacy advisor runs on-device.
8. **Tamper evidence:** key policy events are signed and auditable.

---

## 5) Threat Model

### Adversaries
- Commercial trackers and ad-tech fingerprinting ecosystems
- Hostile/public network observers
- Malicious local software with user-level foothold
- Opportunistic physical seizure of device
- Supply-chain compromise attempts (updates/packages)

### Main Threats
- Cross-context deanonymization
- Metadata leakage (files, clipboard, timezone, language patterns)
- Permission creep and accidental overexposure
- Unsafe USB/removable media vectors
- Update poisoning and untrusted binaries

### Assumptions
- User may operate in high-risk or censored networks
- User may need plausible deniability workflows
- User still needs practical daily-use apps

---

## 6) Functional Pillars

## 6.1 Persona Shards

Separate identities with hard boundaries:

- `Work`
- `Anon`
- `Burner`
- `Lab` (isolated security tooling only)

Each shard has:
- Independent encrypted storage tree
- Independent network profile and routes
- Independent app sessions and browser profiles
- Independent policy history

No direct cross-shard copy is allowed.

## 6.2 Airlock Transfer

All cross-shard transfer passes through an **Airlock** service:

- Mandatory metadata/redaction preview
- Optional format conversion (PDF/image flatten)
- Content risk warnings (EXIF, macros, hidden data)
- Signed transfer record

## 6.3 Capability Permissions

Permissions are tokenized and expiring:

- “Allow mic for 15 minutes in Work shard only”
- “Allow network for this app instance only”
- “Allow file access to one folder, read-only, until app exit”

## 6.4 Network Privacy Stack

- Per-shard routes: clearnet/VPN/Tor profiles
- DNS encryption and resolver isolation per shard
- Leak protections: IPv6 policy, DNS leak checks, kill-switch
- Session rotation mode for sensitive shards

## 6.5 Metadata and Data Lifecycle

- Auto EXIF stripping for exports
- Clipboard auto-expire and secure clear
- Download quarantine and optional auto-delete TTL
- Policy-based file expiration for sensitive workflows

## 6.6 Emergency Modes

- **Panic Mode:** lock all shards, kill network, wipe volatile secrets
- **Mask Mode:** instant decoy desktop/workspace
- **Travel Mode:** reduced local footprint, strict ephemeral policy

---

## 7) System Architecture

Core daemons (Linux-first implementation, microkernel-ready boundaries):

- `phantomkernel-init` — boot orchestration and immutable root checks
- `phantomkernel-policyd` — capability broker and permission policy engine
- `phantomkernel-shardd` — shard lifecycle and context isolation
- `phantomkernel-netd` — shard-aware network policy and route manager
- `phantomkernel-airlockd` — controlled inter-shard transfer pipeline
- `phantomkernel-guardian` — local OPSEC advisor and risk alerts
- `phantomkernel-updated` — signed/reproducible updates + rollback
- `phantomkernel-auditd` — tamper-evident audit log service
- `phantomkernel-shell` — terminal-first control interface

Communication model:

- Local IPC with authenticated service identities
- Policy checks before privileged actions
- Minimal shared state, strict service boundaries

---

## 8) Boot, Trust, and Cryptography

### Boot Chain
- Secure boot with user-controlled enrollment where hardware permits
- Measured boot with integrity report available to user
- Immutable or mostly immutable base partition (`A/B` update strategy)

### Crypto Choices
- KDF: `Argon2id`
- Symmetric AEAD: `XChaCha20-Poly1305`
- Signing: `Ed25519`
- Hashing: `BLAKE3` (where appropriate) / `SHA-256` for compatibility

### Key Strategy
- Device root key + per-shard data keys
- Optional hardware token for admin and shard unlock
- Memory zeroization policy for short-lived secrets

---

## 9) Application Model

- Applications run sandboxed by default.
- No app receives global home directory access.
- Network/camera/mic/filesystem permissions are separate capabilities.
- High-risk apps can be configured as disposable ephemeral sessions.
- Legacy app compatibility runs behind additional containment.

---

## 10) User Experience and FSociety Layer

Thematic layer is optional and never weakens security policy.

### Themes
- `Allsafe` (clean professional)
- `Fsociety` (terminal-centric hacker aesthetic)
- `DarkArmy` (high-contrast strict mode)

### UX Features
- Live privacy score with plain-language reasons
- “Mission” shortcuts: `ghost`, `airgap`, `drill`, `mask`, `airlock`
- Clear human-readable warnings before risky actions

---

## 11) Default App Set (MVP)

- Hardened browser (per-shard isolated profiles)
- Secure notes/vault
- File manager with sanitizer integration
- Update manager with rollback UI
- Backup tool (encrypted, split-secret recovery option)
- Network monitor (leak checks, route state, kill-switch visibility)

---

## 12) Update and Supply Chain Security

- Signed update metadata and packages
- Reproducible builds target for core components
- Multi-signer release policy for critical channels
- Rollback checkpoints before each major update
- Offline update bundle path for restricted environments

---

## 13) Hardware Support Strategy

Initial support should target a small certified matrix:

- 2–3 laptop models with reliable Linux compatibility
- Preference for physical camera/mic kill switches
- TPM/secure element integration where useful

Avoid broad hardware support early; prioritize trust and quality.

---

## 14) Legal and Ethical Guardrails

- Product direction is defensive and privacy-preserving.
- Lab tooling remains isolated and clearly user-gated.
- No default workflows encouraging unauthorized access.
- In-product messaging emphasizes lawful and ethical operation.

---

## 15) MVP Roadmap (180 Days)

### Days 1–30
- Installer, encrypted setup, secure boot integration
- Immutable base image and recovery mode

### Days 31–60
- `phantomkernel-shardd` base implementation
- Per-shard storage keys and basic policy broker

### Days 61–90
- `phantomkernel-netd` per-shard routes
- Kill-switch and leak-check baseline

### Days 91–120
- `phantomkernel-airlockd` transfer workflow
- Metadata scrubber integration
- Panic mode and mask mode first release

### Days 121–150
- `phantomkernel-guardian` v1 (rule + heuristic engine)
- Leak simulator v1 and dashboard score

### Days 151–180
- Update signing/rollback hardening
- Hardware validation on target matrix
- External security review prep

---

## 16) Module Contracts (v1)

### `phantomkernel-policyd`
- Input: permission requests (app, shard, resource, action, duration)
- Output: signed capability token or denial reason
- Guarantees: deterministic policy evaluation, token expiry enforcement

### `phantomkernel-shardd`
- Input: shard lifecycle commands (create/start/stop/suspend)
- Output: isolated execution context handles
- Guarantees: no cross-shard mount/session bleed

### `phantomkernel-netd`
- Input: shard network intents (Tor/VPN/Direct/Offline)
- Output: route application status and leak-check result
- Guarantees: enforceable shard route isolation

### `phantomkernel-airlockd`
- Input: export/import requests between shards
- Output: sanitized artifact package + transfer log record
- Guarantees: inspection gate before release

### `phantomkernel-auditd`
- Input: signed event reports from core daemons
- Output: append-only tamper-evident audit chain
- Guarantees: event integrity and verification tools

---

## 17) Repository Blueprint

```text
phantomkernel-fs/
  docs/
    OS_BIBLE.md
    PRD.md
    THREAT_MODEL.md
    ARCHITECTURE.md
    API_CONTRACTS.md
    HARDWARE_MATRIX.md
  core/
    phantomkernel-init/
    phantomkernel-policyd/
    phantomkernel-shardd/
    phantomkernel-netd/
    phantomkernel-airlockd/
    phantomkernel-auditd/
    phantomkernel-updated/
    phantomkernel-guardian/
    phantomkernel-shell/
  ui/
    themes/
      allsafe/
      fsociety/
      darkarmy/
  packaging/
    installer/
    image-build/
    update-channel/
  tests/
    integration/
    policy/
    network-leaks/
    shard-isolation/
```

---

## 18) Validation and Definition of Done

Release candidate is accepted when:

1. Clean install works on certified hardware.
2. At least three shards are fully isolated in storage, process, and network context.
3. Cross-shard transfer only succeeds through Airlock.
4. Panic mode reliably locks and wipes volatile sensitive state.
5. Leak simulator provides measurable before/after reduction.
6. Signed updates and rollback pass end-to-end tests.
7. External review reports no critical default misconfiguration.

---

## 19) Multi-Model Collaboration Protocol

Use models as complementary roles:

- **Codex-class model**: implementation, scaffolding, tests, refactors
- **Kimi-class model**: long-context consistency, dependency tracing, architecture coherence
- **Opus-class model**: adversarial review, threat thinking, policy clarity, UX language quality

### Workflow
1. Draft architecture/spec in one source of truth.
2. Generate module tickets with acceptance criteria.
3. Implement in short sprints with test-first for isolation boundaries.
4. Run adversarial and leak-surface reviews each sprint.
5. Update the source-of-truth docs before next sprint.

---

## 20) Master Prompt (Paste-Ready)

```text
You are contributing to PhantomKernel OS, a privacy-first defensive OS.

Objectives:
- Enforce persona-based isolation (Work, Anon, Burner, Lab)
- Implement capability-based, expiring permissions
- Preserve strict shard boundaries in storage/network/process contexts
- Support Airlock-only cross-shard transfer with mandatory metadata sanitization
- Maintain secure boot/update trust and rollback safety
- Provide local OPSEC guidance and leak-surface reporting

Constraints:
- Defensive and lawful design only
- No features that optimize unauthorized access
- Keep defaults secure and understandable
- Produce auditable, testable modules

Deliverables:
- Architecture updates
- API contracts
- Implementation plan
- Tests and validation criteria
```

---

## 21) Stretch Goals (Post-MVP)

- Microkernel-native service migration for core policy path
- Stronger deniable workspace capabilities
- Remote attestation for enterprise-managed high-risk devices
- Formal verification of selected policy-critical components
- Federated privacy analytics that never export raw user telemetry

---

## 22) Final Note

PhantomKernel OS should be judged by one standard:

> **Can normal users stay private under pressure without becoming security experts?**

If the answer is yes, the OS succeeds.
