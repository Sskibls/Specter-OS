# Unit Tests

Reserved for focused crate-level tests.

## Milestone 4 Test Files

| File | Component | Coverage |
|------|-----------|----------|
| `phantomkernel_policyd_tests.rs` | policyd | Authz checks, config validation, fail-closed behavior |
| `phantomkernel_shardd_tests.rs` | shardd | Lifecycle state machine, platform boundary, persistence |
| `phantomkernel_netd_tests.rs` | netd | Executor boundary, kill-switch, leak checker |
| `phantomkernel_airlockd_tests.rs` | airlockd | MIME sniff, sanitizer adapters, session state machine |
| `gk_audit_tests.rs` | gk-audit | Chain append/verify invariants, tamper detection |

## Running Unit Tests

```bash
# All unit tests
cargo test --lib

# Specific component
cargo test -p phantomkernel-policyd --lib
cargo test -p phantomkernel-shardd --lib
cargo test -p phantomkernel-netd --lib
cargo test -p phantomkernel-airlockd --lib
cargo test -p gk-audit --lib
```

