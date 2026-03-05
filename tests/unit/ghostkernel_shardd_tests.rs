//! Unit tests for phantomkernel-shardd
//!
//! Coverage:
//! - Shard lifecycle state machine validation
//! - Config/runtime path validation
//! - Platform boundary stub behavior
//! - Persistence and recovery

use phantomkernel_shardd::{
    LinuxNamespaceStub, NamespaceBoundary, ShardError, ShardManager, ShardRuntimeState, ShardState,
    ShardTransitionRecord,
};
use gk_audit::AuditChain;
use gk_persistence::PersistedState;
use std::cell::RefCell;
use std::path::PathBuf;

// ============================================================================
// Shard Lifecycle State Machine
// ============================================================================

#[test]
fn lifecycle_valid_transitions() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    // None -> Created
    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert_eq!(manager.state_of("work"), Some(ShardState::Created));

    // Created -> Running
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());
    assert_eq!(manager.state_of("work"), Some(ShardState::Running));

    // Running -> Stopped
    assert!(manager.stop_shard("work", 3, &mut audit).is_ok());
    assert_eq!(manager.state_of("work"), Some(ShardState::Stopped));

    // Stopped -> None (destroy)
    assert!(manager.destroy_shard("work", 4, &mut audit).is_ok());
    assert_eq!(manager.state_of("work"), None);
}

#[test]
fn lifecycle_stopped_to_running_allowed() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());
    assert!(manager.stop_shard("work", 3, &mut audit).is_ok());

    // Stopped -> Running (restart)
    assert!(manager.start_shard("work", 4, &mut audit).is_ok());
    assert_eq!(manager.state_of("work"), Some(ShardState::Running));
}

#[test]
fn lifecycle_created_to_stopped_denied() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());

    // Created -> Stopped is invalid (must go through Running)
    let result = manager.stop_shard("work", 2, &mut audit);
    assert!(matches!(
        result,
        Err(ShardError::InvalidTransition {
            from: Some(ShardState::Created),
            attempted: "stop"
        })
    ));
}

#[test]
fn lifecycle_running_to_created_denied() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());

    // Running -> Created is invalid
    let result = manager.create_shard("work", 3, &mut audit);
    assert!(matches!(result, Err(ShardError::AlreadyExists)));
}

#[test]
fn lifecycle_destroy_requires_stopped_state() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());

    // Created -> destroy is invalid
    let result = manager.destroy_shard("work", 2, &mut audit);
    assert!(matches!(
        result,
        Err(ShardError::InvalidTransition {
            from: Some(ShardState::Created),
            attempted: "destroy"
        })
    ));

    // Running -> destroy is invalid
    assert!(manager.start_shard("work", 3, &mut audit).is_ok());
    let result = manager.destroy_shard("work", 4, &mut audit);
    assert!(matches!(
        result,
        Err(ShardError::InvalidTransition {
            from: Some(ShardState::Running),
            attempted: "destroy"
        })
    ));
}

#[test]
fn lifecycle_not_found_errors() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    // Operations on non-existent shard
    assert!(matches!(
        manager.start_shard("nonexistent", 1, &mut audit),
        Err(ShardError::NotFound)
    ));
    assert!(matches!(
        manager.stop_shard("nonexistent", 2, &mut audit),
        Err(ShardError::NotFound)
    ));
    assert!(matches!(
        manager.destroy_shard("nonexistent", 3, &mut audit),
        Err(ShardError::NotFound)
    ));
}

// ============================================================================
// Config/Runtime Path Validation
// ============================================================================

#[test]
fn runtime_state_schema_version_is_correct() {
    assert_eq!(ShardRuntimeState::CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(ShardRuntimeState::STATE_KIND, "phantomkernel-shardd-runtime");
}

#[test]
fn runtime_state_serialization_round_trip() {
    let mut state = ShardRuntimeState {
        shard_states: std::collections::HashMap::new(),
        transitions: Vec::new(),
    };

    state.shard_states.insert("work".to_string(), ShardState::Running);
    state.shard_states.insert("anon".to_string(), ShardState::Stopped);
    state.transitions.push(ShardTransitionRecord {
        shard_name: "work".to_string(),
        from: None,
        to: Some(ShardState::Created),
        at_epoch_s: 100,
    });

    let json = serde_json::to_string(&state).expect("should serialize");
    let deserialized: ShardRuntimeState = serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(state.shard_states, deserialized.shard_states);
    assert_eq!(state.transitions.len(), deserialized.transitions.len());
}

#[test]
fn runtime_state_persistence_and_recovery() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("shardd-state.json");

    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());

    manager
        .save_runtime_state(&path)
        .expect("state should save");

    let recovered_platform = LinuxNamespaceStub;
    let mut recovered = ShardManager::new(recovered_platform);
    recovered
        .load_runtime_state(&path)
        .expect("state should load");

    assert_eq!(recovered.state_of("work"), Some(ShardState::Running));
    assert_eq!(recovered.transitions().len(), 2);
}

// ============================================================================
// Platform Boundary Behavior
// ============================================================================

#[derive(Default)]
struct RecordingPlatform {
    calls: RefCell<Vec<String>>,
    fail_on: RefCell<Option<String>>,
}

impl RecordingPlatform {
    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    fn set_fail_on(&self, operation: &str) {
        *self.fail_on.borrow_mut() = Some(operation.to_string());
    }
}

impl NamespaceBoundary for RecordingPlatform {
    fn create_namespace(&self, shard_name: &str) -> Result<(), ShardError> {
        if self
            .fail_on
            .borrow()
            .as_ref()
            .map_or(false, |op| op == "create")
        {
            return Err(ShardError::PlatformFailure("create failed".to_string()));
        }
        self.calls.borrow_mut().push(format!("create:{shard_name}"));
        Ok(())
    }

    fn start_namespace(&self, shard_name: &str) -> Result<(), ShardError> {
        if self
            .fail_on
            .borrow()
            .as_ref()
            .map_or(false, |op| op == "start")
        {
            return Err(ShardError::PlatformFailure("start failed".to_string()));
        }
        self.calls.borrow_mut().push(format!("start:{shard_name}"));
        Ok(())
    }

    fn stop_namespace(&self, shard_name: &str) -> Result<(), ShardError> {
        if self
            .fail_on
            .borrow()
            .as_ref()
            .map_or(false, |op| op == "stop")
        {
            return Err(ShardError::PlatformFailure("stop failed".to_string()));
        }
        self.calls.borrow_mut().push(format!("stop:{shard_name}"));
        Ok(())
    }

    fn destroy_namespace(&self, shard_name: &str) -> Result<(), ShardError> {
        if self
            .fail_on
            .borrow()
            .as_ref()
            .map_or(false, |op| op == "destroy")
        {
            return Err(ShardError::PlatformFailure("destroy failed".to_string()));
        }
        self.calls
            .borrow_mut()
            .push(format!("destroy:{shard_name}"));
        Ok(())
    }
}

#[test]
fn platform_boundary_calls_recorded_in_order() {
    let platform = RecordingPlatform::default();
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());
    assert!(manager.stop_shard("work", 3, &mut audit).is_ok());
    assert!(manager.destroy_shard("work", 4, &mut audit).is_ok());

    let calls = manager.platform.calls();
    assert_eq!(
        calls,
        vec!["create:work", "start:work", "stop:work", "destroy:work"]
    );
}

#[test]
fn platform_boundary_create_failure_is_propagated() {
    let platform = RecordingPlatform::default();
    platform.set_fail_on("create");
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    let result = manager.create_shard("work", 1, &mut audit);
    assert!(matches!(result, Err(ShardError::PlatformFailure(_))));
}

#[test]
fn platform_boundary_start_failure_is_propagated() {
    let platform = RecordingPlatform::default();
    platform.set_fail_on("start");
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());

    let result = manager.start_shard("work", 2, &mut audit);
    assert!(matches!(result, Err(ShardError::PlatformFailure(_))));

    // State should remain Created after failure
    assert_eq!(manager.state_of("work"), Some(ShardState::Created));
}

// ============================================================================
// Audit Chain Integration
// ============================================================================

#[test]
fn audit_events_recorded_for_transitions() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());
    assert!(manager.stop_shard("work", 3, &mut audit).is_ok());
    assert!(manager.destroy_shard("work", 4, &mut audit).is_ok());

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "shardd.transition")
        .collect();

    assert_eq!(events.len(), 4);
    assert!(events[0].payload.contains("None->Created"));
    assert!(events[1].payload.contains("Created->Running"));
    assert!(events[2].payload.contains("Running->Stopped"));
    assert!(events[3].payload.contains("Stopped->None"));
}

#[test]
fn audit_events_not_recorded_on_failure() {
    let platform = RecordingPlatform::default();
    platform.set_fail_on("start");
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    let _ = manager.start_shard("work", 2, &mut audit);

    // Only create should have been recorded
    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "shardd.transition")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("None->Created"));
}

// ============================================================================
// Multiple Shards
// ============================================================================

#[test]
fn multiple_shards_independent() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    // Create and start work shard
    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());

    // Create and start anon shard
    assert!(manager.create_shard("anon", 3, &mut audit).is_ok());
    assert!(manager.start_shard("anon", 4, &mut audit).is_ok());

    // Stop only work shard
    assert!(manager.stop_shard("work", 5, &mut audit).is_ok());

    // Verify states are independent
    assert_eq!(manager.state_of("work"), Some(ShardState::Stopped));
    assert_eq!(manager.state_of("anon"), Some(ShardState::Running));
}

#[test]
fn transitions_recorded_for_all_shards() {
    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.create_shard("anon", 2, &mut audit).is_ok());
    assert!(manager.start_shard("work", 3, &mut audit).is_ok());
    assert!(manager.start_shard("anon", 4, &mut audit).is_ok());

    let transitions = manager.transitions();
    assert_eq!(transitions.len(), 4);

    let work_transitions: Vec<_> = transitions
        .iter()
        .filter(|t| t.shard_name == "work")
        .collect();
    let anon_transitions: Vec<_> = transitions
        .iter()
        .filter(|t| t.shard_name == "anon")
        .collect();

    assert_eq!(work_transitions.len(), 2);
    assert_eq!(anon_transitions.len(), 2);
}

// ============================================================================
// Fail-Closed Behavior
// ============================================================================

#[test]
fn fail_closed_persistence_failure_graceful() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("shardd-state.json");

    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());

    // Save state
    manager
        .save_runtime_state(&path)
        .expect("state should save");

    // Corrupt the file
    std::fs::write(&path, "not valid json").expect("corrupt file");

    // Load should fail gracefully
    let mut recovered = ShardManager::new(platform);
    let result = recovered.load_runtime_state(&path);
    assert!(matches!(result, Err(ShardError::PersistenceFailure(_))));
}

#[test]
fn fail_closed_crash_recovery_from_tmp() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("shardd-state.json");
    let tmp_path = temp.path().join("shardd-state.json.tmp");

    let platform = LinuxNamespaceStub;
    let mut manager = ShardManager::new(platform);
    let mut audit = AuditChain::default();

    assert!(manager.create_shard("work", 1, &mut audit).is_ok());
    assert!(manager.start_shard("work", 2, &mut audit).is_ok());

    // Simulate crash during save (state in tmp)
    manager
        .save_runtime_state(&tmp_path)
        .expect("state should save to tmp");

    // Recovery should detect tmp and use it
    let mut recovered = ShardManager::new(platform);
    recovered
        .load_runtime_state(&path)
        .expect("state should recover from tmp");

    assert_eq!(recovered.state_of("work"), Some(ShardState::Running));
}
