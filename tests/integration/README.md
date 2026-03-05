# Integration Tests

Reserved for cross-daemon and interface integration scenarios.

## Milestone 4 Test Files

| File | Coverage |
|------|----------|
| `milestone4_runtime_tests.rs` | Restart recovery, cross-shard denial, kill-switch, high-risk rejection, audit tamper detection |

## Test Scenarios

### Restart Recovery
- `restart_recovery_policyd_state_preserved` - Policy tokens/rules persist across restart
- `restart_recovery_shardd_state_preserved` - Shard states persist across restart
- `restart_recovery_airlockd_state_preserved` - Approved transfers persist across restart
- `restart_recovery_audit_chain_preserved` - Audit chain integrity preserved

### Cross-Shard Security
- `cross_shard_copy_denied_without_airlock_approval` - Direct transfer blocked
- `cross_shard_copy_allowed_only_after_airlock_session` - Airlock flow required

### Kill-Switch Enforcement
- `kill_switch_active_blocks_all_network_operations` - All routing blocked
- `kill_switch_persists_across_operations` - Kill-switch state maintained
- `kill_switch_deactivation_restores_operations` - Recovery workflow

### High-Risk Rejection
- `high_risk_file_rejected_and_audit_event_created` - Executable rejection
- `high_risk_score_rejected_and_audit_event_created` - Risk score threshold

### Audit Chain Integrity
- `audit_chain_tamper_payload_modification_detected`
- `audit_chain_tamper_event_insertion_detected`
- `audit_chain_tamper_event_deletion_detected`
- `audit_chain_recovery_from_valid_snapshot`

## Running Integration Tests

```bash
# All integration tests
cargo test --test milestone4_runtime_tests --package phantomkernel-test-harness

# Specific test
cargo test --test milestone4_runtime_tests --package phantomkernel-test-harness -- kill_switch
```

