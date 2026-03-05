# PhantomKernel OS — Milestone 2 Implementation Notes

## Scope Implemented

Milestone 2 implements functional logic layers for:
- `phantomkernel-policyd`
- `phantomkernel-shardd`
- `phantomkernel-netd`
- `phantomkernel-airlockd`

These updates were completed in-place on top of the Milestone 1 scaffold with no architectural redesign.

## Architecture Decisions

### 1) Deterministic token signature path
- Added deterministic signing/verification helpers in `gk-crypto`.
- `phantomkernel-policyd` signs capability payloads using a deterministic keyed hash path.
- Validation rejects unknown, expired, revoked, shard-mismatched, or tampered tokens.

### 2) Explicit state machines and error types
- `phantomkernel-shardd` lifecycle is strict: `Create -> Start -> Stop -> Destroy`.
- `phantomkernel-airlockd` lifecycle is strict: `Open -> Scan -> Approve/Reject -> Commit`.
- Invalid transitions fail with typed errors.

### 3) Platform-boundary traits for maintainability
- `phantomkernel-shardd` introduces `NamespaceBoundary` to isolate Linux namespace operations behind a trait.
- `phantomkernel-netd` introduces `LeakChecker` for deterministic and swappable leak-check implementations.
- `phantomkernel-airlockd` introduces `SanitizerPipeline` for sanitizer/risk pipeline substitution.

### 4) Fail-closed defensive defaults
- Policy engine denies unless a matching allow-rule exists.
- Network engine denies when kill-switch is enabled or no profile exists.
- Airlock denies direct cross-shard transfer unless a session is committed.

### 5) Shared audit event propagation
- All four services emit audit records into a shared `AuditChain`.
- Integration tests assert event ingestion from every service.

## Known TODOs

1. Replace deterministic signature stub with production cryptographic key handling and rotation.
2. Add persistent storage adapters for shard state and policy token indexes.
3. Extend network logic from policy-layer model to real route/nftables integration.
4. Integrate content scanners beyond metadata-strip mock for Airlock risk analysis.
5. Wire daemon logic to IPC/proto handlers for runtime service endpoints.
6. Expand integration coverage to include update daemon and full IPC boundaries.
