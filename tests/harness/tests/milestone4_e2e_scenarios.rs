//! E2E harness scenarios for Milestone 4
//!
//! Coverage:
//! - boot -> start services -> run core request flow -> restart -> verify continuity
//! - run panic/kill-switch workflow and verify fail-closed results
//! - verify audit replay CLI reports valid chain after operations
//!
//! These scenarios are designed to be:
//! - Deterministic (no flaky timing assumptions)
//! - CI-friendly and non-interactive
//! - Debian + Fedora smoke compatible

use phantomkernel_airlockd::{AirlockService, ArtifactDescriptor, PluggableSanitizerChain};
use phantomkernel_netd::{
    CommandExecutor, DeterministicLeakChecker, NetworkBackendError, NetworkBackendMode,
    NetworkPolicyError, NetworkPolicyService, NftablesRouteBackend, RouteProfile,
};
use phantomkernel_policyd::{CapabilityRequest, CapabilityRule, PolicyError, PolicyService};
use phantomkernel_shardd::{LinuxNamespaceStub, ShardManager, ShardState};
use gk_audit::{AuditChain, SignedAuditEvent};
use std::cell::RefCell;
use std::path::Path;
use tempfile::TempDir;

// Mock executor for E2E tests
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

// ============================================================================
// Scenario: Boot -> Start Services -> Core Request Flow -> Restart -> Continuity
// ============================================================================

/// E2E scenario: Full boot and restart continuity test
///
/// This simulates:
/// 1. Initial boot: Initialize all services
/// 2. Run core request flow: Policy token -> Shard lifecycle -> Network profile -> Airlock transfer
/// 3. Simulated restart: Persist state, create new service instances
/// 4. Verify continuity: All state preserved, operations can continue
#[test]
fn e2e_boot_restart_continuity() {
    let temp = TempDir::new().expect("tempdir should create");
    let state_dir = temp.path().join("state");
    std::fs::create_dir_all(&state_dir).expect("state dir should create");

    // =========================================================================
    // Phase 1: Boot and initialize services
    // =========================================================================
    println!("[E2E] Phase 1: Boot and initialize services");

    let mut policy_service = PolicyService::new("e2e-signing-key");
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    // =========================================================================
    // Phase 2: Run core request flow
    // =========================================================================
    println!("[E2E] Phase 2: Run core request flow");

    // 2a. Policy: Allow and issue token
    policy_service.allow_rule(CapabilityRule::new(
        "app://browser",
        "work",
        "network",
        "connect",
    ));

    let token_request = CapabilityRequest {
        subject: "app://browser".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 3600,
    };
    let token = policy_service
        .issue_token(&token_request, 1000, &mut audit)
        .expect("token should issue");

    // 2b. Shard: Create and start work shard
    assert!(shard_manager.create_shard("work", 1000, &mut audit).is_ok());
    assert!(shard_manager.start_shard("work", 1001, &mut audit).is_ok());

    // 2c. Network: Apply Tor profile
    assert!(net_service
        .apply_profile("work", RouteProfile::Tor, &mut audit)
        .is_ok());

    // 2d. Airlock: Transfer artifact
    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("download-1");
    let _ = airlock_service
        .scan_session(&session_id, artifact.clone(), &mut audit)
        .expect("scan should pass");
    airlock_service
        .approve_session(&session_id, &mut audit)
        .expect("approve should pass");
    airlock_service
        .commit_session(&session_id, &mut audit)
        .expect("commit should pass");

    // Verify initial state
    assert!(policy_service
        .validate_token(&token, "work", 1002, &mut audit)
        .is_ok());
    assert_eq!(shard_manager.state_of("work"), Some(ShardState::Running));
    assert_eq!(net_service.profile_of("work"), Some(RouteProfile::Tor));
    assert!(airlock_service
        .request_direct_transfer("work", "anon", "download-1", &mut audit)
        .is_ok());

    // =========================================================================
    // Phase 3: Persist state (simulating shutdown)
    // =========================================================================
    println!("[E2E] Phase 3: Persist state");

    let policyd_path = state_dir.join("policyd.json");
    let shardd_path = state_dir.join("shardd.json");
    let netd_path = state_dir.join("netd.json");
    let airlockd_path = state_dir.join("airlockd.json");
    let audit_path = state_dir.join("audit.json");

    policy_service
        .save_runtime_state(&policyd_path)
        .expect("policyd state should save");
    shard_manager
        .save_runtime_state(&shardd_path)
        .expect("shardd state should save");
    net_service
        .save_runtime_state(&netd_path)
        .expect("netd state should save");
    airlock_service
        .save_runtime_state(&airlockd_path)
        .expect("airlockd state should save");
    save_audit_chain(&audit_path, &audit).expect("audit chain should save");

    let pre_restart_audit_len = audit.len();

    // =========================================================================
    // Phase 4: Restart (create new service instances)
    // =========================================================================
    println!("[E2E] Phase 4: Restart services");

    let mut policy_service = PolicyService::new("e2e-signing-key");
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = load_audit_chain(&audit_path).expect("audit chain should load");

    // Load persisted state
    policy_service
        .load_runtime_state(&policyd_path)
        .expect("policyd state should load");
    shard_manager
        .load_runtime_state(&shardd_path)
        .expect("shardd state should load");
    net_service
        .load_runtime_state(&netd_path, &mut audit)
        .expect("netd state should load");
    airlock_service
        .load_runtime_state(&airlockd_path)
        .expect("airlockd state should load");

    // =========================================================================
    // Phase 5: Verify continuity
    // =========================================================================
    println!("[E2E] Phase 5: Verify continuity");

    // Token should still be valid
    assert!(policy_service
        .validate_token(&token, "work", 1003, &mut audit)
        .is_ok());

    // Shard state preserved
    assert_eq!(shard_manager.state_of("work"), Some(ShardState::Running));

    // Network profile preserved
    assert_eq!(net_service.profile_of("work"), Some(RouteProfile::Tor));
    assert!(net_service.can_route("work", &mut audit).is_ok());

    // Airlock transfer still allowed
    assert!(airlock_service
        .request_direct_transfer("work", "anon", "download-1", &mut audit)
        .is_ok());

    // Audit chain integrity preserved (note: can_route adds an event, so length increases)
    assert!(audit.verify());
    assert!(audit.len() >= pre_restart_audit_len);

    // =========================================================================
    // Phase 6: Continue operations after restart
    // =========================================================================
    println!("[E2E] Phase 6: Continue operations");

    // Issue new token
    let token_request2 = CapabilityRequest {
        subject: "app://browser".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 3600,
    };
    let token2 = policy_service
        .issue_token(&token_request2, 2000, &mut audit)
        .expect("new token should issue");
    assert!(policy_service
        .validate_token(&token2, "work", 2001, &mut audit)
        .is_ok());

    // New airlock session
    let session_id2 = airlock_service.open_session("work", "anon", &mut audit);
    let artifact2 = pdf_artifact("download-2");
    let _ = airlock_service
        .scan_session(&session_id2, artifact2.clone(), &mut audit)
        .expect("scan should pass");
    airlock_service
        .approve_session(&session_id2, &mut audit)
        .expect("approve should pass");
    airlock_service
        .commit_session(&session_id2, &mut audit)
        .expect("commit should pass");

    assert!(airlock_service
        .request_direct_transfer("work", "anon", "download-2", &mut audit)
        .is_ok());

    println!("[E2E] Boot-restart continuity test PASSED");
}

// ============================================================================
// Scenario: Panic/Kill-Switch Workflow with Fail-Closed Verification
// ============================================================================

/// E2E scenario: Kill-switch activation and fail-closed verification
///
/// This simulates:
/// 1. Normal operations with network access
/// 2. Panic event triggers kill-switch
/// 3. Verify all network operations blocked (fail-closed)
/// 4. Verify audit trail of kill-switch activation
#[test]
fn e2e_kill_switch_fail_closed_workflow() {
    let temp = TempDir::new().expect("tempdir should create");
    let state_dir = temp.path().join("state");
    std::fs::create_dir_all(&state_dir).expect("state dir should create");

    // =========================================================================
    // Phase 1: Normal operations
    // =========================================================================
    println!("[E2E] Kill-switch test: Phase 1 - Normal operations");

    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let mut audit = AuditChain::default();

    // Set up multiple shards with network profiles
    assert!(net_service
        .apply_profile("work", RouteProfile::Direct, &mut audit)
        .is_ok());
    assert!(net_service
        .apply_profile("anon", RouteProfile::Tor, &mut audit)
        .is_ok());
    assert!(net_service
        .apply_profile("sandbox", RouteProfile::Vpn, &mut audit)
        .is_ok());

    // Verify all can route
    assert!(net_service.can_route("work", &mut audit).is_ok());
    assert!(net_service.can_route("anon", &mut audit).is_ok());
    assert!(net_service.can_route("sandbox", &mut audit).is_ok());

    // Leak checks show expected results
    let work_report = net_service.run_leak_check("work", &mut audit);
    assert!(!work_report.clean); // Direct has leaks
    assert_eq!(work_report.risk_score, 85);

    // =========================================================================
    // Phase 2: Panic event - activate kill-switch
    // =========================================================================
    println!("[E2E] Kill-switch test: Phase 2 - PANIC: Activating kill-switch");

    assert!(net_service.set_kill_switch(true, &mut audit).is_ok());

    // =========================================================================
    // Phase 3: Verify fail-closed state
    // =========================================================================
    println!("[E2E] Kill-switch test: Phase 3 - Verify fail-closed");

    // All routing blocked
    assert!(matches!(
        net_service.can_route("work", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));
    assert!(matches!(
        net_service.can_route("anon", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));
    assert!(matches!(
        net_service.can_route("sandbox", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));

    // New profile application doesn't help
    assert!(net_service
        .apply_profile("new-shard", RouteProfile::Direct, &mut audit)
        .is_ok());
    assert!(matches!(
        net_service.can_route("new-shard", &mut audit),
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));

    // Leak checks report clean (no leaks possible with kill-switch)
    let work_report = net_service.run_leak_check("work", &mut audit);
    assert!(work_report.clean);
    assert_eq!(work_report.risk_score, 0);
    assert!(work_report.summary.contains("kill-switch-active"));

    // =========================================================================
    // Phase 4: Verify audit trail
    // =========================================================================
    println!("[E2E] Kill-switch test: Phase 4 - Verify audit trail");

    let kill_switch_events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.kill-switch")
        .collect();
    assert_eq!(kill_switch_events.len(), 1);
    assert!(kill_switch_events[0].payload.contains("enabled=true"));

    let route_denied_events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.route.denied")
        .collect();
    assert!(route_denied_events.len() >= 3); // At least work, anon, sandbox, new-shard
    for event in route_denied_events {
        assert!(event.payload.contains("kill-switch-enabled"));
    }

    // =========================================================================
    // Phase 5: Recovery - deactivate kill-switch
    // =========================================================================
    println!("[E2E] Kill-switch test: Phase 5 - Recovery");

    assert!(net_service.set_kill_switch(false, &mut audit).is_ok());

    // Routing restored
    assert!(net_service.can_route("work", &mut audit).is_ok());
    assert!(net_service.can_route("anon", &mut audit).is_ok());
    assert!(net_service.can_route("sandbox", &mut audit).is_ok());

    println!("[E2E] Kill-switch fail-closed workflow test PASSED");
}

// ============================================================================
// Scenario: Audit Replay CLI - Verify Chain After Operations
// ============================================================================

/// E2E scenario: Audit chain replay and verification
///
/// This simulates:
/// 1. Run operations across all services
/// 2. Persist audit chain
/// 3. "Replay" audit chain (load and verify)
/// 4. Report chain validity and event summary
#[test]
fn e2e_audit_replay_verification() {
    let temp = TempDir::new().expect("tempdir should create");
    let audit_path = temp.path().join("audit-chain.json");

    // =========================================================================
    // Phase 1: Run operations across all services
    // =========================================================================
    println!("[E2E] Audit replay test: Phase 1 - Run operations");

    let mut audit = AuditChain::default();

    // Policy operations
    let mut policy_service = PolicyService::new("audit-replay-key");
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
    let token = policy_service
        .issue_token(&request, 100, &mut audit)
        .expect("token should issue");
    let _ = policy_service.validate_token(&token, "work", 101, &mut audit);

    // Shard operations
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    let _ = shard_manager.create_shard("work", 100, &mut audit);
    let _ = shard_manager.start_shard("work", 101, &mut audit);
    let _ = shard_manager.stop_shard("work", 102, &mut audit);

    // Network operations
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    let _ = net_service.apply_profile("work", RouteProfile::Tor, &mut audit);
    let _ = net_service.can_route("work", &mut audit);
    let _ = net_service.run_leak_check("work", &mut audit);

    // Airlock operations
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let session_id = airlock_service.open_session("work", "anon", &mut audit);
    let _ = airlock_service.scan_session(&session_id, pdf_artifact("test"), &mut audit);
    let _ = airlock_service.approve_session(&session_id, &mut audit);
    let _ = airlock_service.commit_session(&session_id, &mut audit);

    println!(
        "[E2E] Audit replay test: Recorded {} audit events",
        audit.len()
    );

    // =========================================================================
    // Phase 2: Persist audit chain
    // =========================================================================
    println!("[E2E] Audit replay test: Phase 2 - Persist audit chain");

    save_audit_chain(&audit_path, &audit).expect("audit chain should save");

    // =========================================================================
    // Phase 3: "Replay" - Load and verify chain
    // =========================================================================
    println!("[E2E] Audit replay test: Phase 3 - Replay and verify");

    let replayed_audit = load_audit_chain(&audit_path).expect("audit chain should load");

    // Verify chain integrity
    let verification_result = replayed_audit.verify_detailed();
    assert!(
        verification_result.is_ok(),
        "Audit chain verification failed: {verification_result:?}"
    );

    // =========================================================================
    // Phase 4: Report chain validity and event summary
    // =========================================================================
    println!("[E2E] Audit replay test: Phase 4 - Generate report");

    let audit_report = generate_audit_report(&replayed_audit);

    // Verify report contents
    assert!(audit_report.is_valid);
    assert_eq!(audit_report.total_events, audit.len());
    assert!(audit_report.events_by_service.contains_key("policyd"));
    assert!(audit_report.events_by_service.contains_key("shardd"));
    assert!(audit_report.events_by_service.contains_key("netd"));
    assert!(audit_report.events_by_service.contains_key("airlockd"));

    // Print report (simulating CLI output)
    println!("[AUDIT REPLAY REPORT]");
    println!("  Chain Valid: {}", audit_report.is_valid);
    println!("  Total Events: {}", audit_report.total_events);
    println!("  Events by Service:");
    for (service, count) in &audit_report.events_by_service {
        println!("    {service}: {count}");
    }

    println!("[E2E] Audit replay verification test PASSED");
}

// ============================================================================
// Scenario: Debian/Fedora Smoke Test Compatibility
// ============================================================================

/// E2E scenario: Distribution-agnostic smoke test
///
/// This test verifies core functionality without distribution-specific
/// dependencies, making it compatible with both Debian and Fedora backends.
#[test]
fn e2e_distribution_agnostic_smoke_test() {
    println!("[E2E] Smoke test: Running distribution-agnostic checks");

    let mut audit = AuditChain::default();

    // Test 1: Policy deny-by-default
    println!("[E2E] Smoke test: Policy deny-by-default");
    let mut policy_service = PolicyService::new("smoke-test-key");
    let request = CapabilityRequest {
        subject: "app://unknown".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 60,
    };
    let result = policy_service.issue_token(&request, 100, &mut audit);
    assert!(matches!(result, Err(PolicyError::DenyByDefault)));

    // Test 2: Shard lifecycle
    println!("[E2E] Smoke test: Shard lifecycle");
    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    assert!(shard_manager.create_shard("test", 100, &mut audit).is_ok());
    assert!(shard_manager.start_shard("test", 101, &mut audit).is_ok());
    assert!(shard_manager.stop_shard("test", 102, &mut audit).is_ok());
    assert!(shard_manager.destroy_shard("test", 103, &mut audit).is_ok());

    // Test 3: Network profiles
    println!("[E2E] Smoke test: Network profiles");
    let mut net_service = NetworkPolicyService::new(
        DeterministicLeakChecker,
        NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, MockExecutor::successful()),
    );
    assert!(net_service
        .apply_profile("test", RouteProfile::Tor, &mut audit)
        .is_ok());
    assert!(net_service.can_route("test", &mut audit).is_ok());

    // Test 4: Airlock transfer
    println!("[E2E] Smoke test: Airlock transfer");
    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let session_id = airlock_service.open_session("test", "anon", &mut audit);
    let artifact = pdf_artifact("smoke-test");
    let _ = airlock_service
        .scan_session(&session_id, artifact.clone(), &mut audit)
        .expect("scan should pass");
    airlock_service
        .approve_session(&session_id, &mut audit)
        .expect("approve should pass");
    airlock_service
        .commit_session(&session_id, &mut audit)
        .expect("commit should pass");
    assert!(airlock_service
        .request_direct_transfer("test", "anon", "smoke-test", &mut audit)
        .is_ok());

    // Test 5: Audit chain integrity
    println!("[E2E] Smoke test: Audit chain integrity");
    assert!(audit.verify());

    println!("[E2E] Distribution-agnostic smoke test PASSED");
}

// ============================================================================
// Helper Types and Functions
// ============================================================================

struct AuditReport {
    is_valid: bool,
    total_events: usize,
    events_by_service: std::collections::HashMap<String, usize>,
}

fn generate_audit_report(audit: &AuditChain) -> AuditReport {
    let mut events_by_service: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for event in audit.events() {
        let service = event
            .event_type
            .split('.')
            .next()
            .unwrap_or("unknown")
            .to_string();
        *events_by_service.entry(service).or_insert(0) += 1;
    }

    AuditReport {
        is_valid: audit.verify(),
        total_events: audit.len(),
        events_by_service,
    }
}

fn pdf_artifact(artifact_id: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: artifact_id.to_string(),
        path: format!("/tmp/{artifact_id}.pdf"),
        metadata_entries: 2,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFpayload".to_vec(),
    }
}

fn save_audit_chain(path: &Path, audit: &AuditChain) -> Result<(), std::io::Error> {
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

fn load_audit_chain(path: &Path) -> Result<AuditChain, std::io::Error> {
    use std::fs;

    let content = fs::read_to_string(path)?;
    let events: Vec<SignedAuditEvent> = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::other(format!("parse error: {e}")))?;

    AuditChain::recover(events).map_err(|e| std::io::Error::other(format!("recovery error: {e}")))
}
