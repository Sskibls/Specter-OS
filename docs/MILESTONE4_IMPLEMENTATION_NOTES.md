# PhantomKernel OS — Milestone 4 Implementation Notes

## Milestone Objective

Milestone 4 wires the hardened logic from prior milestones into concrete runtime execution paths for Debian and Fedora editions while preserving fail-closed security behavior.

## 1) IPC and Runtime Wiring

- Added/extended authz-enforced IPC handler boundaries for:
  - `phantomkernel-policyd`
  - `phantomkernel-shardd`
  - `phantomkernel-netd`
  - `phantomkernel-airlockd`
  - `phantomkernel-auditd`
- Each handler validates caller roles before invoking daemon logic.
- Daemon startup paths now instantiate runtime handlers against real service state.

## 2) Runtime Directories and Startup Validation

- Runtime layout utility in `gk-config` is now used by daemon startup flows.
- Active runtime paths are:
  - `/etc/phantomkernel/`
  - `/var/lib/phantomkernel/`
  - `/var/log/phantomkernel/`
- Startup now performs directory creation + write validation.
- Added `GK_RUNTIME_ROOT` override support in daemon binaries for non-root smoke testing.

## 3) Network Executor Wiring

- `phantomkernel-netd` now uses a real command execution boundary:
  - `ProcessCommandExecutor` for runtime command dispatch
  - staged/enforcing backend modes
  - nftables/route command modeling in backend layer
- Added backend sync on recovered state load.
- Backend failures force kill-switch and remain fail-closed.

## 4) Airlock Executor and Sanitizer Adapters

- Airlock sanitizer chain is now explicitly adapter-driven:
  - `MetadataStripAdapter`
  - `DocumentFlattenAdapter`
  - `RiskScoringAdapter`
- MIME decisions are content-sniffed (not extension-based).
- Unknown or high-risk artifacts are rejected by default.
- Cross-shard direct transfer remains denied unless approved via Airlock flow.

## 5) Audit Persistence and Tooling

- `gk-audit` now includes append-only on-disk chain storage (`AuditStore`).
- Added replay verification and truncated-tail recovery behavior.
- `phantomkernel-auditd` now persists and verifies chain data via `AuditStore`.
- `gkctl` now exposes `audit-verify [path]` for replay/verify CLI checks.

## 6) Restart Reliability and Cross-Edition Smoke

- Added restart durability tests across core services (policy, shard, net, airlock, audit).
- Added cross-edition startup + workflow smoke tests:
  - Debian route profile path (`Tor`)
  - Fedora route profile path (`Vpn`)
- `ci/scripts/run-vm-smoke.sh` now runs the cross-edition harness test target.

## Known TODOs

1. Integrate real privileged execution controls for production nftables/route application under system service user constraints.
2. Extend Airlock adapters with optional external tooling bridges for format-specific flattening/sanitization.
3. Add audit CLI subcommands for filtered replay/query output beyond boolean verification.
4. Expand edition smoke tests into package-installed VM boot validations (post-install service start checks).
