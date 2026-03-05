# PhantomKernel OS — API Reference (v1.0)

This document provides complete API documentation for PhantomKernel OS core daemons, including their purpose, public structures, IPC methods, and error handling.

---

## 1. phantomkernel-policyd (Policy Engine)

### Purpose
The capability broker and permission policy engine. It evaluates requests for resources and issues time-bound, cryptographically signed capability tokens.

### Public API
- `PolicyService`: Main service for managing rules and tokens.
- `CapabilityRule`: Defines a permitted action (`subject`, `shard`, `resource`, `action`).
- `CapabilityRequest`: Request for a token with a specific TTL.
- `CapabilityToken`: Signed token granting access.

### IPC Methods
- `EvaluateCapability`: Issue a token if an allow rule exists.
- `ValidateCapability`: Verify a token's signature, expiry, and shard context.
- `RevokeCapability`: Mark a token as revoked.
- `RotateSigningKey`: Update the internal signing keys.

### Error Types
- `DenyByDefault`: No matching rule found.
- `ExpiredToken`: Token TTL has passed.
- `InvalidSignature`: Token tampered or signed by untrusted key.
- `ShardMismatch`: Token used in the wrong persona shard.

---

## 2. phantomkernel-shardd (Shard Manager)

### Purpose
Handles the lifecycle of persona shards (`Work`, `Anon`, `Burner`, `Lab`) and ensures process/namespace isolation.

### Public API
- `ShardManager<P: NamespaceBoundary>`: Manages shard states and transitions.
- `ShardState`: `Created`, `Running`, `Stopped`.
- `NamespaceBoundary`: Trait for platform-specific isolation (e.g., Linux Namespaces).

### IPC Methods
- `CreateShard`: Initialize a new shard filesystem and namespace.
- `StartShard`: Launch the shard environment.
- `StopShard`: Gracefully terminate a shard's processes.
- `DestroyShard`: Remove a shard's ephemeral state.
- `GetShardState`: Query the current status of a shard.

### Error Types
- `AlreadyExists` / `NotFound`.
- `InvalidTransition`: e.g., attempting to start a running shard.
- `PlatformFailure`: Error in the underlying namespace provider.

---

## 3. phantomkernel-netd (Network Daemon)

### Purpose
Shard-aware network policy manager. Enforces per-shard routing (Tor, VPN, Direct) and implements the global emergency kill-switch.

### Public API
- `NetworkPolicyService`: Orchestrates routing and leak checks.
- `RouteProfile`: `Offline`, `Direct`, `Tor`, `Vpn`.
- `LeakCheckReport`: Results of a network isolation audit.

### IPC Methods
- `ApplyRouteProfile`: Set the routing strategy for a specific shard.
- `SetKillSwitch`: Globally enable/disable all network egress.
- `RunLeakCheck`: Perform a diagnostic scan for unauthorized traffic.
- `GetRouteState`: Query active routes and kill-switch status.

### Error Types
- `KillSwitchEnabled`: Operation blocked by active kill-switch.
- `EgressBlocked`: Backend (nftables) rejected the route.
- `BackendFailure`: Error communicating with kernel networking subsystem.

---

## 4. phantomkernel-airlockd (Airlock Service)

### Purpose
Controlled inter-shard transfer pipeline. Sanitizes files (metadata stripping, flattening) before allowing cross-shard movement.

### Public API
- `AirlockService`: Manages transfer sessions and sanitization.
- `SanitizerPipeline`: Chain of adapters (Metadata, Office, PDF) for cleaning artifacts.
- `TransferSessionState`: `Open`, `Scanned`, `Approved`, `Rejected`, `Committed`.

### IPC Methods
- `OpenTransferSession`: Initiate a transfer between two shards.
- `ScanArtifact`: Submit a file for sanitization and risk scoring.
- `ApproveTransfer`: Manual or policy-based approval of a scanned file.
- `CommitTransfer`: Finalize the transfer, allowing direct bit-copy.
- `RequestDirectTransfer`: Verify if a transfer is pre-authorized.

### Error Types
- `HighRiskRejected`: Risk score exceeded safety threshold.
- `UnknownMimeRejected`: File type is not supported or inherently unsafe.
- `DirectTransferDenied`: Attempted transfer without Airlock approval.

---

## 5. phantomkernel-auditd (Audit Service)

### Purpose
Tamper-evident, append-only security event logger. Uses a hash-chain to ensure log integrity.

### Public API
- `AuditDaemon`: High-level interface for appending and querying.
- `SignedAuditEvent`: Event record with sequence, hash, and previous-hash.
- `VerifyChainResult`: Outcome of a full-chain integrity audit.

### IPC Methods
- `AppendEvent`: Add a new security event to the log.
- `VerifyChain`: Run an integrity check on the entire audit log.
- `QueryEvents`: Retrieve historical events starting from a sequence number.

### Error Types
- `IntegrityMismatch`: Chain hash verification failed (tampering detected).
- `StoreError`: Filesystem or persistence failure.

---

## 6. phantomkernel-updated (Update Daemon)

### Purpose
Manages signed, reproducible OS updates using an A/B partition scheme with rollback support.

### Public API
- `UpdateService`: Handles checking, downloading, and applying updates.
- `UpdateSlot`: `A` or `B`.
- `UpdateManifest`: Signed JSON describing the release and components.

### IPC Methods (Planned)
- `CheckUpdates`: Query server for new versions.
- `ApplyUpdate`: Install update to the inactive slot.
- `CommitUpdate`: Mark the current slot as successful after reboot.
- `Rollback`: Force boot into the previous working slot.

---

## 7. phantomkernel-guardian (OPSEC Advisor)

### Purpose
Monitors system state and provides emergency containment workflows (Panic, Mask, Travel).

### Public API
- `GuardianService`: Orchestrates emergency state transitions.
- `Panic`: One-step lockdown (Kill Net, Lock Shards, Wipe Secrets).
- `Mask`: Switch to decoy workspace.
- `Travel`: Enforce strict ephemeral-only policies.

---

## 8. phantomkernel-init (Boot Orchestrator)

### Purpose
(Stub) Boot orchestration and immutable root integrity checks.

---

## Cross-Component Relationships

1. **Isolation Chain**: `shardd` creates namespaces -> `netd` applies routing to those namespaces -> `policyd` issues tokens for apps inside them.
2. **Transfer Chain**: Shard app -> `policyd` (request airlock access) -> `airlockd` (sanitize) -> `auditd` (log transfer).
3. **Emergency Chain**: `guardian` -> triggers `netd` (kill-switch) and `shardd` (stop shards).
