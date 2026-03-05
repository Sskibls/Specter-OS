# Milestone 4 Test Coverage Map

## Overview

This document maps test coverage for Milestone 4 runtime-executor paths across:
- `phantomkernel-policyd`
- `phantomkernel-shardd`
- `phantomkernel-netd`
- `phantomkernel-airlockd`
- `gk-audit` (audit chain persistence)

## Test Files

### Unit Tests (`tests/unit/`)

| File | Component | Test Count |
|------|-----------|------------|
| `phantomkernel_policyd_tests.rs` | policyd | 18 |
| `phantomkernel_shardd_tests.rs` | shardd | 19 |
| `phantomkernel_netd_tests.rs` | netd | 26 |
| `phantomkernel_airlockd_tests.rs` | airlockd | 35 |
| `gk_audit_tests.rs` | audit | 36 |

### Integration Tests (`tests/integration/`)

| File | Coverage | Test Count |
|------|----------|------------|
| `milestone4_runtime_tests.rs` | Cross-service integration | 15 |

### E2E Harness (`tests/harness/tests/`)

| File | Scenario | Test Count |
|------|----------|------------|
| `milestone4_e2e_scenarios.rs` | End-to-end workflows | 4 |

---

## Risk Coverage Map

### phantomkernel-policyd

| Risk | Tests | File |
|------|-------|------|
| **R1: Unauthorized access (deny-by-default bypass)** | `authz_deny_by_default_no_rules_configured` | unit/phantomkernel_policyd_tests.rs |
| **R2: Privilege escalation (partial rule match)** | `authz_partial_rule_match_still_denies` | unit/phantomkernel_policyd_tests.rs |
| **R3: Token forgery/tampering** | `tamper_detection_rejects_modified_tokens` (existing) | core/daemons/phantomkernel-policyd/src/lib.rs |
| **R4: Token expiry bypass** | `expiry_is_enforced` (existing), `authz_shard_mismatch_in_request_denied_at_validation` | core/daemons/phantomkernel-policyd/src/lib.rs, unit/phantomkernel_policyd_tests.rs |
| **R5: Signing key compromise** | `fail_closed_on_signing_key_expired`, `key_revocation_audit_event_recorded` | unit/phantomkernel_policyd_tests.rs |
| **R6: State persistence corruption** | `runtime_state_persistence_and_recovery`, `fail_closed_token_revocation_persists` | unit/phantomkernel_policyd_tests.rs |
| **R7: Crypto failure handling** | `fail_closed_on_crypto_failure` | unit/phantomkernel_policyd_tests.rs |

### phantomkernel-shardd

| Risk | Tests | File |
|------|-------|------|
| **R1: Invalid state transitions** | `lifecycle_created_to_stopped_denied`, `lifecycle_destroy_requires_stopped_state` | unit/phantomkernel_shardd_tests.rs |
| **R2: Namespace boundary failures** | `platform_boundary_create_failure_is_propagated`, `platform_boundary_start_failure_is_propagated` | unit/phantomkernel_shardd_tests.rs |
| **R3: State persistence corruption** | `runtime_state_persistence_and_recovery`, `fail_closed_crash_recovery_from_tmp` | unit/phantomkernel_shardd_tests.rs |
| **R4: Multi-shard isolation** | `multiple_shards_independent`, `transitions_recorded_for_all_shards` | unit/phantomkernel_shardd_tests.rs |
| **R5: Audit trail gaps** | `audit_events_recorded_for_transitions`, `audit_events_not_recorded_on_failure` | unit/phantomkernel_shardd_tests.rs |

### phantomkernel-netd

| Risk | Tests | File |
|------|-------|------|
| **R1: Kill-switch bypass** | `kill_switch_blocks_all_routing`, `kill_switch_active_blocks_all_network_operations` | unit/phantomkernel_netd_tests.rs, integration/milestone4_runtime_tests.rs |
| **R2: Network profile misconfiguration** | `profile_apply_persisted_and_retrievable`, `profile_multiple_shards_independent` | unit/phantomkernel_netd_tests.rs |
| **R3: Backend failure handling** | `fail_closed_backend_failure_triggers_kill_switch`, `executor_backend_failure_mode_propagated` | unit/phantomkernel_netd_tests.rs |
| **R4: Leak detection failures** | `leak_checker_deterministic_multiple_calls`, `fail_closed_no_profile_fail_closed_leak_report` | unit/phantomkernel_netd_tests.rs |
| **R5: Offline profile enforcement** | `fail_closed_offline_profile_blocked` | unit/phantomkernel_netd_tests.rs |

### phantomkernel-airlockd

| Risk | Tests | File |
|------|-------|------|
| **R1: MIME type confusion attacks** | `mime_sniff_*_detected`, `mime_mismatch_declared_pdf_actual_text_increases_risk` | unit/phantomkernel_airlockd_tests.rs |
| **R2: Unknown MIME bypass** | `unknown_mime_executable_rejected`, `unknown_mime_truly_unknown_rejected` | unit/phantomkernel_airlockd_tests.rs |
| **R3: High-risk artifact bypass** | `high_risk_threshold_is_70`, `high_risk_artifacts_are_rejected` | unit/phantomkernel_airlockd_tests.rs |
| **R4: Sanitizer adapter failure** | `sanitizer_metadata_strip_removes_exif`, `sanitizer_risk_scoring_*` | unit/phantomkernel_airlockd_tests.rs |
| **R5: State machine violations** | `session_state_machine_*` (6 tests) | unit/phantomkernel_airlockd_tests.rs |
| **R6: Direct transfer bypass** | `direct_transfer_denied_without_approval`, `cross_shard_copy_denied_without_airlock_approval` | unit/phantomkernel_airlockd_tests.rs, integration/milestone4_runtime_tests.rs |
| **R7: Persistence corruption** | `runtime_state_persistence_and_recovery` | unit/phantomkernel_airlockd_tests.rs |

### gk-audit

| Risk | Tests | File |
|------|-------|------|
| **R1: Chain tampering (payload)** | `tamper_payload_modification_detected` | unit/gk_audit_tests.rs |
| **R2: Chain tampering (sequence)** | `tamper_sequence_modification_detected`, `verify_sequence_mismatch_detected` | unit/gk_audit_tests.rs |
| **R3: Chain tampering (hash)** | `tamper_event_insertion_detected`, `verify_previous_hash_mismatch_detected` | unit/gk_audit_tests.rs |
| **R4: Event deletion** | `tamper_delete_event_detected` | unit/gk_audit_tests.rs |
| **R5: Event reordering** | `tamper_reorder_events_detected` | unit/gk_audit_tests.rs |
| **R6: Recovery from valid snapshot** | `recovery_succeeds_for_untampered_snapshot`, `audit_chain_recovery_from_valid_snapshot` | unit/gk_audit_tests.rs, integration/milestone4_runtime_tests.rs |
| **R7: Hash chain integrity** | `hash_chain_*` (6 tests) | unit/gk_audit_tests.rs |

---

## Integration Test Coverage

| Scenario | Tests | File |
|----------|-------|------|
| **Restart recovery (policyd)** | `restart_recovery_policyd_state_preserved` | integration/milestone4_runtime_tests.rs |
| **Restart recovery (shardd)** | `restart_recovery_shardd_state_preserved` | integration/milestone4_runtime_tests.rs |
| **Restart recovery (airlockd)** | `restart_recovery_airlockd_state_preserved` | integration/milestone4_runtime_tests.rs |
| **Restart recovery (audit)** | `restart_recovery_audit_chain_preserved` | integration/milestone4_runtime_tests.rs |
| **Cross-shard denial** | `cross_shard_copy_denied_without_airlock_approval`, `cross_shard_copy_allowed_only_after_airlock_session` | integration/milestone4_runtime_tests.rs |
| **Kill-switch enforcement** | `kill_switch_active_blocks_all_network_operations`, `kill_switch_persists_across_operations` | integration/milestone4_runtime_tests.rs |
| **High-risk rejection** | `high_risk_file_rejected_and_audit_event_created`, `high_risk_score_rejected_and_audit_event_created` | integration/milestone4_runtime_tests.rs |
| **Audit tamper detection** | `audit_chain_tamper_*` (4 tests) | integration/milestone4_runtime_tests.rs |
| **Cross-service flow** | `full_request_flow_policy_shard_net_airlock` | integration/milestone4_runtime_tests.rs |

---

## E2E Harness Coverage

| Scenario | Tests | File |
|----------|-------|------|
| **Boot-restart continuity** | `e2e_boot_restart_continuity` | harness/tests/milestone4_e2e_scenarios.rs |
| **Kill-switch fail-closed** | `e2e_kill_switch_fail_closed_workflow` | harness/tests/milestone4_e2e_scenarios.rs |
| **Audit replay verification** | `e2e_audit_replay_verification` | harness/tests/milestone4_e2e_scenarios.rs |
| **Distribution smoke test** | `e2e_distribution_agnostic_smoke_test` | harness/tests/milestone4_e2e_scenarios.rs |

---

## Test Commands by Tier

### Unit Tests
```bash
# All unit tests
cargo test -p phantomkernel-policyd --lib
cargo test -p phantomkernel-shardd --lib
cargo test -p phantomkernel-netd --lib
cargo test -p phantomkernel-airlockd --lib
cargo test -p gk-audit --lib

# Or run all workspace unit tests
cargo test --lib
```

### Integration Tests
```bash
# Integration tests
cargo test --test milestone4_runtime_tests --package phantomkernel-test-harness
```

### E2E Harness Tests
```bash
# E2E scenarios
cargo test --test milestone4_e2e_scenarios --package phantomkernel-test-harness
cargo test --test milestone2_flows --package phantomkernel-test-harness
```

### Full Test Suite
```bash
# All tests
cargo test --workspace

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace --out Html

# CI-friendly (no network, deterministic)
cargo test --workspace --locked --frozen
```

---

## Known Gaps (TODO)

### High Priority

- [ ] **TODO: Network backend integration tests** - Current tests use `NftablesRouteBackend` stub. Real nftables integration tests require elevated privileges and should be run in CI containers.
  - Location: `tests/integration/` or `tests/vm/`
  - Blocked by: CI container nftables setup

- [ ] **TODO: Pluggable sanitizer adapter tests** - Custom sanitizer adapters beyond the default chain are not tested.
  - Location: `tests/unit/phantomkernel_airlockd_tests.rs`
  - Blocked by: Need example custom adapter implementations

- [ ] **TODO: Migration path tests** - Schema version migration tests for state persistence when schema versions increment.
  - Location: `tests/unit/` for each daemon
  - Blocked by: Need schema version increments to test migration

### Medium Priority

- [ ] **TODO: Concurrent access tests** - No tests for concurrent access to shared state (policy rules, shard states, etc.)
  - Location: `tests/integration/`
  - Note: Current design is single-threaded; concurrent access would require mutex/lock testing

- [ ] **TODO: Large state persistence tests** - No tests for persistence with large numbers of tokens/sessions
  - Location: `tests/integration/`
  - Note: Should test with 1000+ tokens/sessions to verify performance

- [ ] **TODO: Audit chain size limits** - No tests for audit chain behavior at scale
  - Location: `tests/unit/gk_audit_tests.rs`
  - Note: Should verify memory usage and verification time for large chains

### Low Priority

- [ ] **TODO: Debian/Fedora backend-specific tests** - Edition-specific backend tests
  - Location: `editions/debian/` and `editions/fedora/` test directories
  - Blocked by: Backend implementation details

- [ ] **TODO: CLI integration tests** - Tests for `gkctl` CLI integration with daemons
  - Location: `core/cli/gkctl/` tests
  - Blocked by: CLI implementation progress

---

## Quality Constraints Checklist

- [x] Deterministic tests only (no flaky timing assumptions)
- [x] CI-friendly and non-interactive
- [x] No architecture redesign
- [x] Keep phantomkernel-* naming only
- [x] All tests pass with `cargo test --workspace`
