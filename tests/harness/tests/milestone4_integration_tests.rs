//! Integration tests for Milestone 4 runtime-executor paths
//!
//! Coverage:
//! - Restart recovery with persisted state (policy/shard/airlock/audit)
//! - Direct cross-shard copy denial unless airlock approved
//! - Kill-switch active => no network operations allowed
//! - High-risk file => reject and audit event created
//! - Audit chain tamper detection + recovery flow

use phantomkernel_airlockd::{
    AirlockService, ArtifactDescriptor, PluggableSanitizerChain, TransferSessionState,
};
use phantomkernel_netd::{
    CommandExecutor, DeterministicLeakChecker, NetworkBackendError, NetworkBackendMode,
    NetworkPolicyError, NetworkPolicyService, NftablesRouteBackend, RouteProfile,
};
use std::cell::RefCell;

// Mock executor for integration tests
struct MockExecutor {
    calls: RefCell<Vec<String>>,
}

impl MockExecutor {
    fn successful() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl CommandExecutor for MockExecutor {
    fn run(&self, program: &str, args: &[String]) -> Result<String, NetworkBackendError> {
        self.calls
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));
        Ok("ok".to_string())
    }
}
use phantomkernel_policyd::{CapabilityRequest, CapabilityRule, PolicyService};
use phantomkernel_shardd::{LinuxNamespaceStub, ShardManager, ShardState};
use gk_audit::{AuditChain, AuditIntegrityError};
use std::collections::HashSet;
use tempfile::TempDir;

// ============================================================================
// Restart Recovery with Persisted State
// ============================================================================

#[test]
fn restart_recovery_policyd_state_preserved() {
    let temp = TempDir::new().expect("tempdir should create");
    let state_path = temp.path().join("policyd-state.json");

    // Initial session
    let mut policy_service = PolicyService::new("recovery-test-key");
    let mut audit = AuditChain::default();

    policy_service.allow_rule(CapabilityRule::new(
        "app://mail",
        "work",
        "network",
        "connect",
    ));

    let request = CapabilityRequest {
        subject: "app://mail".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 300,
    };
    let token = policy_service
        .issue_token(&request, 1000, &mut audit)
        .expect("token should issue");

    policy_service
        .save_runtime_state(&state_path)
        .expect("state should save");

    // Simulate restart
    let mut recovered_service = PolicyService::new("recovery-test-key");
    recovered_service
        .load_runtime_state(&state_path)
        .expect("state should load");

    // Token should still be valid
    let validation = recovered_service.validate_token(&token, "work", 1100, &mut audit);
    assert!(validation.is_ok());

    // Allow rules should be preserved
    let state = recovered_service.runtime_state();
    assert_eq!(state.allow_rules.len(), 1);
}

#[test]
fn restart_recovery_shardd_state_preserved() {
    let temp = TempDir::new().expect("tempdir should create");
    let state_path = temp.path().join("shardd-state.json");

    // Initial session
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    let mut audit = AuditChain::default();

    assert!(shard_manager.create_shard("work", 100, &mut audit).is_ok());
    assert!(shard_manager.start_shard("work", 101, &mut audit).is_ok());
    assert!(shard_manager.create_shard("anon", 102, &mut audit).is_ok());

    shard_manager
        .save_runtime_state(&state_path)
        .expect("state should save");

    // Simulate restart
    let mut recovered_manager = ShardManager::new(LinuxNamespaceStub);
    recovered_manager
        .load_runtime_state(&state_path)
        .expect("state should load");

    // States should be preserved
    assert_eq!(
        recovered_manager.state_of("work"),
        Some(ShardState::Running)
    );
    assert_eq!(
        recovered_manager.state_of("anon"),
        Some(ShardState::Created)
    );
    assert_eq!(recovered_manager.transitions().len(), 3);
}

#[test]
fn restart_recovery_airlockd_state_preserved() {
    let temp = TempDir::new().expect("tempdir should create");
    let state_path = temp.path().join("airlockd-state.json");

    // Initial session
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("recovery-test");

    let _ = airlock_service.scan_session(&session_id, artifact.clone(), &mut audit);
    let _ = airlock_service.approve_session(&session_id, &mut audit);
    let _ = airlock_service.commit_session(&session_id, &mut audit);

    airlock_service
        .save_runtime_state(&state_path)
        .expect("state should save");

    // Simulate restart
    let mut recovered_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    recovered_service
        .load_runtime_state(&state_path)
        .expect("state should load");

    // Transfer should still be allowed
    let result =
        recovered_service.request_direct_transfer("work", "anon", "recovery-test", &mut audit);
    assert!(result.is_ok());
}

#[test]
fn restart_recovery_audit_chain_preserved() {
    let temp = TempDir::new().expect("tempdir should create");
    let audit_path = temp.path().join("audit-chain.json");

    // Initial session with multiple services
    let mut audit = AuditChain::default();

    // Policy operations
    let mut policy_service = PolicyService::new("audit-recovery-key");
    policy_service.allow_rule(CapabilityRule::new(
        "app://test",
        "work",
        "network",
        "connect",
    ));
    let request = CapabilityRequest {
        subject: "app://test".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 60,
    };
    let _ = policy_service.issue_token(&request, 100, &mut audit);

    // Shard operations
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    let _ = shard_manager.create_shard("work", 100, &mut audit);
    let _ = shard_manager.start_shard("work", 101, &mut audit);

    // Netd operations
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let _ = net_service.apply_profile("work", RouteProfile::Tor, &mut audit);

    // Airlock operations
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let _ = airlock_service.scan_session(&session_id, pdf_artifact("audit-test"), &mut audit);

    // Save audit chain
    save_audit_chain(&audit_path, &audit).expect("audit chain should save");

    // Simulate restart - load and verify
    let recovered_audit = load_audit_chain(&audit_path).expect("audit chain should load");

    // Verify chain integrity
    assert!(recovered_audit.verify());
    assert_eq!(recovered_audit.len(), audit.len());

    // Verify events from all services present
    let event_types: HashSet<_> = recovered_audit
        .events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();

    assert!(event_types.iter().any(|t| t.starts_with("policyd.")));
    assert!(event_types.iter().any(|t| t.starts_with("shardd.")));
    assert!(event_types.iter().any(|t| t.starts_with("netd.")));
    assert!(event_types.iter().any(|t| t.starts_with("airlockd.")));
}

// ============================================================================
// Direct Cross-Shard Copy Denial
// ============================================================================

#[test]
fn cross_shard_copy_denied_without_airlock_approval() {
    let airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    // Attempt direct transfer without going through airlock
    let result =
        airlock_service.request_direct_transfer("work", "anon", "sensitive-data", &mut audit);

    assert!(matches!(
        result,
        Err(phantomkernel_airlockd::AirlockError::DirectTransferDenied)
    ));

    // Verify audit event recorded
    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.transfer.denied")
        .collect();
    assert_eq!(events.len(), 1);
}

#[test]
fn cross_shard_copy_allowed_only_after_airlock_session() {
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    // First attempt without approval - should fail
    let result = airlock_service.request_direct_transfer("work", "anon", "artifact-1", &mut audit);
    assert!(matches!(
        result,
        Err(phantomkernel_airlockd::AirlockError::DirectTransferDenied)
    ));

    // Go through proper airlock flow
    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("artifact-1");

    let _ = airlock_service
        .scan_session(&session_id, artifact.clone(), &mut audit)
        .expect("scan should pass");
    airlock_service
        .approve_session(&session_id, &mut audit)
        .expect("approve should pass");
    airlock_service
        .commit_session(&session_id, &mut audit)
        .expect("commit should pass");

    // Now direct transfer should be allowed
    let result = airlock_service.request_direct_transfer("work", "anon", "artifact-1", &mut audit);
    assert!(result.is_ok());
}

#[test]
fn cross_shard_copy_different_artifacts_independent() {
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    // Approve only artifact-1
    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("artifact-1");

    let _ = airlock_service
        .scan_session(&session_id, artifact.clone(), &mut audit)
        .unwrap();
    airlock_service
        .approve_session(&session_id, &mut audit)
        .unwrap();
    airlock_service
        .commit_session(&session_id, &mut audit)
        .unwrap();

    // artifact-1 should be allowed
    assert!(airlock_service
        .request_direct_transfer("work", "anon", "artifact-1", &mut audit)
        .is_ok());

    // artifact-2 should still be denied
    assert!(matches!(
        airlock_service.request_direct_transfer("work", "anon", "artifact-2", &mut audit),
        Err(phantomkernel_airlockd::AirlockError::DirectTransferDenied)
    ));
}

// ============================================================================
// Kill-Switch Enforcement
// ============================================================================

#[test]
fn kill_switch_active_blocks_all_network_operations() {
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let mut audit = AuditChain::default();

    // Set up normal routing
    assert!(net_service
        .apply_profile("work", RouteProfile::Direct, &mut audit)
        .is_ok());
    assert!(net_service
        .apply_profile("anon", RouteProfile::Tor, &mut audit)
        .is_ok());

    // Verify routing works before kill-switch
    assert!(net_service.can_route("work", &mut audit).is_ok());
    assert!(net_service.can_route("anon", &mut audit).is_ok());

    // Activate kill-switch
    assert!(net_service.set_kill_switch(true, &mut audit).is_ok());

    // All routing should now be blocked
    assert!(matches!(
        net_service.can_route("work", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));
    assert!(matches!(
        net_service.can_route("anon", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));

    // Leak check should report clean (no leaks possible)
    let report = net_service.run_leak_check("work", &mut audit);
    assert!(report.clean);
    assert_eq!(report.risk_score, 0);
}

#[test]
fn kill_switch_persists_across_operations() {
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let mut audit = AuditChain::default();

    // Activate kill-switch
    assert!(net_service.set_kill_switch(true, &mut audit).is_ok());

    // Apply new profile - should still be blocked
    assert!(net_service
        .apply_profile("new-shard", RouteProfile::Direct, &mut audit)
        .is_ok());

    // New shard should also be blocked
    assert!(matches!(
        net_service.can_route("new-shard", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));
}

#[test]
fn kill_switch_deactivation_restores_operations() {
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let mut audit = AuditChain::default();

    // Set up routing
    assert!(net_service
        .apply_profile("work", RouteProfile::Tor, &mut audit)
        .is_ok());

    // Activate and verify blocked
    assert!(net_service.set_kill_switch(true, &mut audit).is_ok());
    assert!(matches!(
        net_service.can_route("work", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));

    // Deactivate
    assert!(net_service.set_kill_switch(false, &mut audit).is_ok());

    // Routing should be restored
    assert!(net_service.can_route("work", &mut audit).is_ok());
}

// ============================================================================
// High-Risk File Rejection
// ============================================================================

#[test]
fn high_risk_file_rejected_and_audit_event_created() {
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = airlock_service.open_session("work", "anon", &mut audit);

    // High-risk artifact: executable content
    let artifact = ArtifactDescriptor {
        artifact_id: "malicious".to_string(),
        path: "/tmp/dropper.elf".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: b"\x7fELFbinary".to_vec(),
    };

    let result = airlock_service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(
        result,
        Err(phantomkernel_airlockd::AirlockError::UnknownMimeRejected { .. })
    ));

    // Verify audit event recorded
    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.rejected")
        .collect();
    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("unknown-mime"));
}

#[test]
fn high_risk_score_rejected_and_audit_event_created() {
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = airlock_service.open_session("work", "anon", &mut audit);

    // High-risk artifact: ZIP archive with MIME mismatch and many metadata entries
    // Risk calculation: ZIP baseline (35) + metadata (20*2=40, capped) + MIME mismatch (35) = 110 -> capped at 100
    // But metadata gets stripped first, so: ZIP baseline (35) + metadata_stripped (10) + MIME mismatch (35) = 80
    let artifact = ArtifactDescriptor {
        artifact_id: "suspicious".to_string(),
        path: "/tmp/suspicious.zip".to_string(),
        metadata_entries: 20,
        declared_mime: "text/plain".to_string(),
        content_bytes: b"PK\x03\x04\x14\x00\x00\x00\x08\x00test.zip content data here".to_vec(),
    };

    let result = airlock_service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(
        result,
        Err(phantomkernel_airlockd::AirlockError::HighRiskRejected { .. })
    ));

    // Verify audit event recorded
    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.rejected")
        .collect();
    assert_eq!(events.len(), 1);
}

#[test]
fn high_risk_session_state_set_to_rejected() {
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = airlock_service.open_session("work", "anon", &mut audit);

    let artifact = ArtifactDescriptor {
        artifact_id: "elf".to_string(),
        path: "/tmp/elf".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: b"\x7fELF".to_vec(),
    };

    let _ = airlock_service.scan_session(&session_id, artifact, &mut audit);

    assert_eq!(
        airlock_service.session_state(&session_id),
        Some(TransferSessionState::Rejected)
    );
}

// ============================================================================
// Audit Chain Tamper Detection + Recovery
// ============================================================================

#[test]
fn audit_chain_tamper_payload_modification_detected() {
    let mut audit = AuditChain::default();

    // Record operations from multiple services
    audit.append("policyd.token.issued", "tok-001");
    audit.append("shardd.transition", "work:Created->Running");
    audit.append("netd.profile.applied", "work:Tor");
    audit.append("airlockd.session.opened", "session-1");

    // Tamper with payload
    let mut events = audit.snapshot();
    events[1].payload = "tampered".to_string();

    // Recovery should fail
    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::EventHashMismatch { .. })
    ));
}

#[test]
fn audit_chain_tamper_event_insertion_detected() {
    let mut audit = AuditChain::default();

    audit.append("event1", "payload1");
    audit.append("event3", "payload3");

    let mut events = audit.snapshot();

    // Insert fake event
    let fake_event = gk_audit::SignedAuditEvent {
        sequence: 2,
        event_type: "event2".to_string(),
        payload: "fake".to_string(),
        previous_hash: events[0].event_hash.clone(),
        event_hash: "fake-hash".to_string(),
    };
    events.insert(1, fake_event);

    let result = AuditChain::recover(events);
    assert!(result.is_err());
}

#[test]
fn audit_chain_tamper_event_deletion_detected() {
    let mut audit = AuditChain::default();

    audit.append("event1", "payload1");
    audit.append("event2", "payload2");
    audit.append("event3", "payload3");

    let mut events = audit.snapshot();
    events.remove(1); // Delete middle event

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::SequenceMismatch { .. })
    ));
}

#[test]
fn audit_chain_recovery_from_valid_snapshot() {
    let mut audit = AuditChain::default();

    audit.append("policyd.token.issued", "tok-001");
    audit.append("shardd.transition", "work:Created->Running");
    audit.append("netd.profile.applied", "work:Tor");

    let snapshot = audit.snapshot();
    let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");

    assert!(recovered.verify());
    assert_eq!(recovered.len(), 3);
}

#[test]
fn audit_chain_persistence_and_integrity_verification() {
    let temp = TempDir::new().expect("tempdir should create");
    let audit_path = temp.path().join("audit-chain.json");

    let mut audit = AuditChain::default();
    audit.append("policyd.decision", "allow");
    audit.append("shardd.start", "work");
    audit.append("netd.route", "work:Tor");

    // Save
    save_audit_chain(&audit_path, &audit).expect("save should succeed");

    // Load and verify
    let loaded = load_audit_chain(&audit_path).expect("load should succeed");
    assert!(loaded.verify());
    assert_eq!(loaded.len(), 3);
}

// ============================================================================
// Cross-Service Integration
// ============================================================================

#[test]
fn full_request_flow_policy_shard_net_airlock() {
    let mut audit = AuditChain::default();

    // 1. Policy: Issue token for network access
    let mut policy_service = PolicyService::new("integration-key");
    policy_service.allow_rule(CapabilityRule::new(
        "app://browser",
        "work",
        "network",
        "connect",
    ));

    let request = CapabilityRequest {
        subject: "app://browser".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 300,
    };
    let token = policy_service
        .issue_token(&request, 1000, &mut audit)
        .expect("token should issue");

    // 2. Shard: Create and start work shard
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    assert!(shard_manager.create_shard("work", 1000, &mut audit).is_ok());
    assert!(shard_manager.start_shard("work", 1001, &mut audit).is_ok());

    // 3. Net: Apply network profile
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    assert!(net_service
        .apply_profile("work", RouteProfile::Tor, &mut audit)
        .is_ok());

    // Validate token and check routing
    assert!(policy_service
        .validate_token(&token, "work", 1002, &mut audit)
        .is_ok());
    assert!(net_service.can_route("work", &mut audit).is_ok());

    // 4. Airlock: Transfer artifact
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("browser-download");

    let _ = airlock_service
        .scan_session(&session_id, artifact.clone(), &mut audit)
        .expect("scan should pass");
    airlock_service
        .approve_session(&session_id, &mut audit)
        .expect("approve should pass");
    airlock_service
        .commit_session(&session_id, &mut audit)
        .expect("commit should pass");

    // Verify all services recorded audit events
    let event_types: HashSet<_> = audit
        .events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();

    assert!(event_types.iter().any(|t| t.starts_with("policyd.")));
    assert!(event_types.iter().any(|t| t.starts_with("shardd.")));
    assert!(event_types.iter().any(|t| t.starts_with("netd.")));
    assert!(event_types.iter().any(|t| t.starts_with("airlockd.")));
}

// ============================================================================
// Helper Functions
// ============================================================================

fn pdf_artifact(artifact_id: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: artifact_id.to_string(),
        path: format!("/tmp/{artifact_id}.pdf"),
        metadata_entries: 2,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFpayload".to_vec(),
    }
}

fn save_audit_chain(path: &std::path::Path, audit: &AuditChain) -> Result<(), std::io::Error> {
    use std::fs::File;
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let events = audit.snapshot();
    let json = serde_json::to_string_pretty(&events)
        .map_err(|e| std::io::Error::other(format!("serialization error: {e}")))?;

    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn load_audit_chain(path: &std::path::Path) -> Result<AuditChain, std::io::Error> {
    use std::fs;

    let content = fs::read_to_string(path)?;
    let events: Vec<gk_audit::SignedAuditEvent> = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::other(format!("parse error: {e}")))?;

    AuditChain::recover(events).map_err(|e| std::io::Error::other(format!("recovery error: {e}")))
}

// ============================================================================
// Guardian Emergency Modes
// ============================================================================

#[test]
fn guardian_panic_mode_kills_network_quickly() {
    use phantomkernel_guardian::GuardianService;
    use std::time::Instant;

    // Initialize components
    let network_backend = NftablesRouteBackend::new_staged();
    let shard_manager = ShardManager::new(LinuxNamespaceStub);
    let mut guardian = GuardianService::new(network_backend, shard_manager);
    let mut audit_chain = AuditChain::default();

    // Measure time to execute panic mode
    let start_time = Instant::now();
    let result = guardian.panic(&mut audit_chain);
    let duration = start_time.elapsed();

    // Verify panic mode completed successfully
    assert!(result.is_ok());
    
    // Verify it completed within 100ms (requirement from task)
    assert!(duration.as_millis() < 100, "Panic mode took {}ms, expected < 100ms", duration.as_millis());
    
    // Verify audit events were created
    assert!(audit_chain.len() > 0);
    
    // Verify network kill switch was activated
    let has_network_kill_event = audit_chain.events().iter()
        .any(|event| event.event_type == "guardian.panic.network_killed");
    assert!(has_network_kill_event, "Network kill event not found in audit chain");
}

#[test]
fn guardian_mask_mode_workspace_switching() {
    use phantomkernel_guardian::GuardianService;

    let network_backend = NftablesRouteBackend::new_staged();
    let shard_manager = ShardManager::new(LinuxNamespaceStub);
    let mut guardian = GuardianService::new(network_backend, shard_manager);
    let mut audit_chain = AuditChain::default();

    // Activate mask mode
    let result = guardian.mask("decoy", &mut audit_chain);
    assert!(result.is_ok());

    // Verify mask mode was activated
    let has_mask_event = audit_chain.events().iter()
        .any(|event| event.event_type == "guardian.mask.activated");
    assert!(has_mask_event, "Mask mode activation event not found");
}

#[test]
fn guardian_travel_mode_toggle() {
    use phantomkernel_guardian::GuardianService;

    let network_backend = NftablesRouteBackend::new_staged();
    let shard_manager = ShardManager::new(LinuxNamespaceStub);
    let mut guardian = GuardianService::new(network_backend, shard_manager);
    let mut audit_chain = AuditChain::default();

    // Initially disabled
    assert!(!guardian.is_travel_mode_enabled());

    // Enable travel mode
    guardian.set_travel_mode(true, &mut audit_chain);
    assert!(guardian.is_travel_mode_enabled());

    // Disable travel mode
    guardian.set_travel_mode(false, &mut audit_chain);
    assert!(!guardian.is_travel_mode_enabled());

    // Verify audit events
    let travel_mode_events: Vec<_> = audit_chain.events().iter()
        .filter(|event| event.event_type == "guardian.travel_mode")
        .collect();
    assert_eq!(travel_mode_events.len(), 2);
}
