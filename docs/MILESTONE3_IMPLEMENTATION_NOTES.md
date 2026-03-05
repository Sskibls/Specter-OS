# PhantomKernel OS — Milestone 3 Implementation Notes

## Milestone Objective

Milestone 3 hardens runtime durability, key handling, network enforcement foundations, Airlock validation, and audit integrity while preserving the existing Milestone 2 architecture.

## Implemented Areas

## 1) Persistence + Durability
- Added shared persistence crate: `core/libs/gk-persistence`.
- Persistence model uses schema-tagged envelopes with migration hooks and atomic write + recover-from-`.tmp` behavior.
- Added runtime persistence methods to:
  - `phantomkernel-policyd`
  - `phantomkernel-shardd`
  - `phantomkernel-airlockd`
- Added crash recovery tests validating state restore after simulated interrupted writes.

## 2) Crypto Hardening
- Replaced deterministic signature stub with `KeyRing` and `SignatureEnvelope` model in `gk-crypto`.
- Added key loading/saving interface via schema-tagged key material file.
- Added key rotation hooks and active key management.
- Added revocation and expiry checks in signing and verification path.
- Added tests for invalid signatures, revoked keys, expired keys, and rollover behavior.

## 3) Network Enforcement Foundation
- Added `NetworkBackend` abstraction with `NftablesRouteBackend` implementation.
- Added staged and enforcing backend modes.
- `phantomkernel-netd` now delegates profile and kill-switch operations to backend boundary.
- Backend failures force fail-closed kill-switch behavior.
- Added tests showing no route allowance when kill-switch is active.

## 4) Airlock Hardening Foundation
- Replaced mock-only sanitizer with pluggable chain model:
  - `MetadataStripAdapter`
  - `RiskScoringAdapter`
- Added strict MIME sniffing from content bytes (not extension-based decision).
- Enforced reject-by-default for unknown/executable MIME and high-risk scores.
- Added tests for unknown MIME rejection, high-risk rejection, and approved commit flow.

## 5) Audit Integrity Improvements
- Upgraded audit events to hash-linked append-only chain records.
- Added deterministic chain verification for sequence/hash integrity.
- Added tamper detection tests and valid snapshot recovery tests.

## Known TODOs (Prioritized)

1. Wire daemon boot/reload paths to persistent state locations under `/var/lib/phantomkernel/*`.
2. Implement real nftables/route command executor behind `NetworkBackend` with privileged boundary controls.
3. Extend MIME detection and sanitizer adapters with external tooling integrations.
4. Add persistent audit log storage and replay tooling using the upgraded chain format.
5. Add key rotation policy automation (scheduled rollover + deprecation windows).
