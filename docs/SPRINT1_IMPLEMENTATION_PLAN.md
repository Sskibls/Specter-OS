# PhantomKernel OS Sprint 1 Implementation Plan

This sprint scaffolds the PhantomKernel repository with compile-pass stubs, interface placeholders, edition layers, systemd units, and CI gates.

## Ticket Backlog (Day 1–14)

### GK-001 — Workspace Bootstrap (Day 1)
- Scope: Initialize Rust workspace, root manifests, and crate topology.
- Done criteria: `cargo check --workspace` succeeds with placeholder crates.
- Dependencies: None.

### GK-002 — Interface Contracts v1 (Day 2)
- Scope: Add `interfaces/proto/phantomkernel/v1` service definitions.
- Done criteria: Proto files for policy/shard/network/airlock/audit/update/common committed.
- Dependencies: GK-001.

### GK-003 — Layered Config Engine (Day 3)
- Scope: Add `gk-config` crate with layered loader and schema export stub.
- Done criteria: Loader merges layered TOML values and unit tests pass.
- Dependencies: GK-001.

### GK-004 — IPC/Auth Base (Day 4)
- Scope: Add `gk-ipc` abstractions and request/response stubs.
- Done criteria: IPC crate compiles, includes minimal contract test.
- Dependencies: GK-002, GK-003.

### GK-005 — phantomkernel-auditd Skeleton (Day 5)
- Scope: Add audit types and append/verify chain placeholder flow.
- Done criteria: Audit crate test validates append path and daemon stub builds.
- Dependencies: GK-004.

### GK-006 — phantomkernel-policyd v0 (Day 6)
- Scope: Add deny-by-default policy evaluator stub.
- Done criteria: Policy tests verify default deny and explicit allow behavior.
- Dependencies: GK-002, GK-003, GK-005.

### GK-007 — phantomkernel-shardd v0 (Day 7)
- Scope: Add shard lifecycle daemon scaffold and integration touchpoints.
- Done criteria: Daemon crate builds and lifecycle placeholders return success.
- Dependencies: GK-006.

### GK-008 — phantomkernel-netd v0 (Day 8)
- Scope: Add network daemon scaffold with route profile placeholders.
- Done criteria: Daemon builds and leak-check stub test exists.
- Dependencies: GK-006, GK-007.

### GK-009 — phantomkernel-airlockd v0 (Day 9)
- Scope: Add transfer session daemon scaffold.
- Done criteria: Daemon builds and direct-transfer denial scenario documented as pending.
- Dependencies: GK-006, GK-007.

### GK-010 — Service Lifecycle Units (Day 10)
- Scope: Add `phantomkernel.target` and daemon unit placeholders.
- Done criteria: Unit ordering reflects lifecycle dependency chain.
- Dependencies: GK-005 through GK-009.

### GK-011 — Debian Edition Layer (Day 11)
- Scope: Add Debian defaults, packaging placeholders, and backend crate.
- Done criteria: Debian backend crate compiles and edition files are present.
- Dependencies: GK-010.

### GK-012 — Fedora Edition Layer (Day 12)
- Scope: Add Fedora defaults, packaging placeholders, and backend crate.
- Done criteria: Fedora backend crate compiles and edition files are present.
- Dependencies: GK-010.

### GK-013 — CI Gate Pipeline (Day 13)
- Scope: Add CI workflow skeleton with formatting, lint, tests, interfaces, config, security, package, and VM gates.
- Done criteria: Workflow file captures all mandatory gates and scripts are scaffolded.
- Dependencies: GK-011, GK-012.

### GK-014 — Harness + Sprint Exit (Day 14)
- Scope: Add test harness crate and baseline smoke scenario.
- Done criteria: Harness tests pass and sprint exit checklist is documented.
- Dependencies: GK-013.

## Sprint Exit Checklist
- Workspace and all placeholder crates compile.
- Proto contracts exist under versioned path.
- Layered config loader and schema placeholder are available.
- Shared and edition service/packaging scaffolds are committed.
- CI gates are represented in a single workflow skeleton.
- Minimal tests execute successfully.

## Milestone 2 Progress Notes

### Core Flow Implementation Status
- `phantomkernel-policyd`: Capability issue/validate/revoke flow implemented with deterministic signature verification, deny-by-default policy, shard binding, expiry checks, and tamper detection tests.
- `phantomkernel-shardd`: Lifecycle state machine (`Create -> Start -> Stop -> Destroy`) implemented with namespace boundary trait, transition persistence, audit event emission, and invalid-transition tests.
- `phantomkernel-netd`: Per-shard route profiles (`Offline`, `Direct`, `Tor`, `Vpn`) implemented with kill-switch precedence, fail-closed routing logic, deterministic leak-check adapter, and policy tests.
- `phantomkernel-airlockd`: Transfer session lifecycle (`Open -> Scan -> Approve/Reject -> Commit`) implemented with sanitizer pipeline trait, high-risk rejection path, and direct-copy bypass denial unless approved.

### Integration Validation Added
- End-to-end deny-by-default rejection.
- Token expiry enforcement.
- Cross-shard direct-copy denial unless committed via Airlock.
- Kill-switch operation blocking in network policy layer.
- Shared audit chain assertions for policy/shard/network/airlock events.

## Milestone 3 Progress Notes

### Completed
- Persistence foundation implemented via `gk-persistence` with schema-aware save/load envelopes, migration hook support, and crash `.tmp` recovery logic.
- `phantomkernel-policyd`, `phantomkernel-shardd`, and `phantomkernel-airlockd` runtime states now support durable save/recover flows.
- Crypto path upgraded to `KeyRing` + `SignatureEnvelope` model with file loading, active key selection, key revocation, and rotation hooks.
- `phantomkernel-policyd` now uses key-ring signing and verification path instead of deterministic hash stubs.
- `phantomkernel-netd` now includes backend abstraction for nftables/routes with staged and enforcing modes.
- Kill-switch semantics hardened to fail-closed on backend failures.
- Airlock sanitizer upgraded to pluggable sanitizer chain with MIME sniffing and metadata stripping adapter.
- Airlock now rejects unknown MIME and high-risk artifacts by default.
- Audit chain upgraded to hash-linked append-only events with tamper detection and verified recovery.

### Remaining
- Wire persistent state backends to daemon runtime boot hooks and service unit paths.
- Integrate real nftables/route operations beyond the staged logic boundary.
- Replace synthetic metadata strip implementation with external tooling adapter (e.g., exiftool-backed execution boundary).
- Add long-run soak tests for crash recovery across repeated save/restart cycles.

## Milestone 4 Progress Notes

### Completed
- IPC authz boundaries are now wired for `phantomkernel-policyd`, `phantomkernel-shardd`, `phantomkernel-netd`, `phantomkernel-airlockd`, and `phantomkernel-auditd`.
- Runtime startup wiring now validates `/etc/phantomkernel`, `/var/lib/phantomkernel`, and `/var/log/phantomkernel` paths with safe creation checks.
- Daemons now support `GK_RUNTIME_ROOT` for deterministic startup testing without privileged host writes.
- `phantomkernel-netd` now persists/restores runtime state and syncs recovered state to backend abstractions.
- Net backend execution boundary now supports staged/enforcing command execution with deterministic fail-closed fallback.
- Airlock sanitizer chain now includes metadata stripping + document flattening adapters and retains strict content-based MIME handling.
- `gk-audit` now persists append-only hash-linked records to disk with replay verification + truncated-tail recovery.
- `gkctl` now supports `audit-verify` to replay/verify persisted audit chains.
- Cross-edition smoke tests now validate Debian/Fedora startup and core workflows in harness and VM smoke script.

### Remaining
- Introduce privileged execution policy for production nftables/route application (capability/system service hardening).
- Extend Airlock adapter coverage for more artifact classes and external sanitizer tool integrations.
- Add richer audit replay querying in CLI (filters, ranges, service grouping).
