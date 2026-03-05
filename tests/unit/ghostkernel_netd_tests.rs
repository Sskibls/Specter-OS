//! Unit tests for phantomkernel-netd
//!
//! Coverage:
//! - Network executor boundary behavior
//! - Fail-closed on network errors
//! - Kill-switch enforcement
//! - Route profile management
//! - Leak checker determinism

use phantomkernel_netd::{
    CommandExecutor, DeterministicLeakChecker, LeakCheckReport, LeakChecker, NetworkBackend,
    NetworkBackendError, NetworkBackendMode, NetworkPolicyError, NetworkPolicyService,
    NftablesRouteBackend, RouteProfile,
};
use gk_audit::AuditChain;
use std::cell::RefCell;

// Mock executor for deterministic testing
struct MockExecutor {
    should_fail: bool,
    calls: RefCell<Vec<String>>,
}

impl MockExecutor {
    fn successful() -> Self {
        Self {
            should_fail: false,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn failing() -> Self {
        Self {
            should_fail: true,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn run(&self, program: &str, args: &[String]) -> Result<String, NetworkBackendError> {
        self.calls
            .borrow_mut()
            .push(format!("{program} {}", args.join(" ")));

        if self.should_fail {
            return Err(NetworkBackendError {
                message: "mock execution failure".to_string(),
            });
        }

        Ok("ok".to_string())
    }
}

// ============================================================================
// Route Profile Management
// ============================================================================

#[test]
fn profile_apply_persisted_and_retrievable() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Tor, &mut audit)
        .is_ok());

    assert_eq!(service.profile_of("work"), Some(RouteProfile::Tor));
}

#[test]
fn profile_multiple_shards_independent() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Tor, &mut audit)
        .is_ok());
    assert!(service
        .apply_profile("anon", RouteProfile::Vpn, &mut audit)
        .is_ok());
    assert!(service
        .apply_profile("sandbox", RouteProfile::Direct, &mut audit)
        .is_ok());

    assert_eq!(service.profile_of("work"), Some(RouteProfile::Tor));
    assert_eq!(service.profile_of("anon"), Some(RouteProfile::Vpn));
    assert_eq!(service.profile_of("sandbox"), Some(RouteProfile::Direct));
}

#[test]
fn profile_overwrite_updates_state() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Direct, &mut audit)
        .is_ok());
    assert_eq!(service.profile_of("work"), Some(RouteProfile::Direct));

    assert!(service
        .apply_profile("work", RouteProfile::Tor, &mut audit)
        .is_ok());
    assert_eq!(service.profile_of("work"), Some(RouteProfile::Tor));
}

#[test]
fn profile_no_profile_returns_no_profile_error() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let result = service.can_route("unknown", &mut audit);
    assert!(matches!(result, Err(NetworkPolicyError::NoProfile)));
}

// ============================================================================
// Kill-Switch Enforcement
// ============================================================================

#[test]
fn kill_switch_blocks_all_routing() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Direct, &mut audit)
        .is_ok());
    assert!(service.set_kill_switch(true, &mut audit).is_ok());

    let result = service.can_route("work", &mut audit);
    assert!(matches!(
        result,
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));
}

#[test]
fn kill_switch_disabled_allows_routing() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Direct, &mut audit)
        .is_ok());
    assert!(service.set_kill_switch(false, &mut audit).is_ok());

    let result = service.can_route("work", &mut audit);
    assert!(result.is_ok());
}

#[test]
fn kill_switch_state_queryable() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);

    assert!(!service.kill_switch_enabled());

    let mut audit = AuditChain::default();
    let _ = service.set_kill_switch(true, &mut audit);

    assert!(service.kill_switch_enabled());
}

#[test]
fn kill_switch_audit_event_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.set_kill_switch(true, &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.kill-switch")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("enabled=true"));
}

// ============================================================================
// Fail-Closed Behavior
// ============================================================================

#[test]
fn fail_closed_backend_failure_triggers_kill_switch() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Enforcing,
        MockExecutor::failing(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let result = service.apply_profile("work", RouteProfile::Tor, &mut audit);
    assert!(matches!(
        result,
        Err(NetworkPolicyError::BackendFailure(_))
    ));

    // Kill switch should be activated on failure
    assert!(service.kill_switch_enabled());
}

#[test]
fn fail_closed_no_profile_fail_closed_leak_report() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let report = service.run_leak_check("unknown", &mut audit);

    // Should report fail-closed state
    assert!(!report.clean);
    assert_eq!(report.risk_score, 100);
    assert!(report.summary.contains("fail-closed-no-profile"));
}

#[test]
fn fail_closed_kill_switch_active_clean_leak_report() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Direct, &mut audit);
    let _ = service.set_kill_switch(true, &mut audit);

    let report = service.run_leak_check("work", &mut audit);

    // Kill switch active means clean (no leaks possible)
    assert!(report.clean);
    assert_eq!(report.risk_score, 0);
    assert!(report.summary.contains("kill-switch-active"));
}

#[test]
fn fail_closed_offline_profile_blocked() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Offline, &mut audit)
        .is_ok());

    let result = service.can_route("work", &mut audit);
    assert!(matches!(
        result,
        Err(NetworkPolicyError::OfflineProfileBlocked)
    ));
}

// ============================================================================
// Executor Boundary (Backend Operations)
// ============================================================================

#[test]
fn executor_backend_operations_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Tor, &mut audit);
    let _ = service.set_kill_switch(true, &mut audit);

    let ops = service.backend().operations();
    assert!(!ops.is_empty());
}

#[test]
fn executor_mock_calls_recorded() {
    let executor = MockExecutor::successful();
    let backend = NftablesRouteBackend::with_executor(NetworkBackendMode::Staged, executor);
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Tor, &mut audit);

    // Staged mode doesn't execute commands, just records operations
    let ops = service.backend().operations();
    assert!(!ops.is_empty());
}

#[test]
fn executor_backend_failure_mode_propagated() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Enforcing,
        MockExecutor::failing(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let profile_result = service.apply_profile("work", RouteProfile::Tor, &mut audit);
    assert!(matches!(
        profile_result,
        Err(NetworkPolicyError::BackendFailure(_))
    ));

    // Kill switch set should also fail
    let kill_result = service.set_kill_switch(false, &mut audit);
    assert!(matches!(
        kill_result,
        Err(NetworkPolicyError::BackendFailure(_))
    ));
}

// ============================================================================
// Deterministic Leak Checker
// ============================================================================

#[test]
fn leak_checker_offline_reports_clean() {
    let checker = DeterministicLeakChecker;
    let report = checker.run_check("work", RouteProfile::Offline);

    assert!(report.clean);
    assert_eq!(report.risk_score, 0);
    assert!(report.summary.contains("offline-no-egress"));
}

#[test]
fn leak_checker_direct_reports_leak() {
    let checker = DeterministicLeakChecker;
    let report = checker.run_check("work", RouteProfile::Direct);

    assert!(!report.clean);
    assert_eq!(report.risk_score, 85);
    assert!(report.summary.contains("direct-egress-detected"));
}

#[test]
fn leak_checker_tor_reports_isolated() {
    let checker = DeterministicLeakChecker;
    let report = checker.run_check("work", RouteProfile::Tor);

    assert!(report.clean);
    assert_eq!(report.risk_score, 15);
    assert!(report.summary.contains("tor-route-isolated"));
}

#[test]
fn leak_checker_vpn_reports_isolated() {
    let checker = DeterministicLeakChecker;
    let report = checker.run_check("work", RouteProfile::Vpn);

    assert!(report.clean);
    assert_eq!(report.risk_score, 20);
    assert!(report.summary.contains("vpn-route-isolated"));
}

#[test]
fn leak_checker_deterministic_multiple_calls() {
    let checker = DeterministicLeakChecker;

    let report1 = checker.run_check("work", RouteProfile::Direct);
    let report2 = checker.run_check("work", RouteProfile::Direct);
    let report3 = checker.run_check("work", RouteProfile::Direct);

    assert_eq!(report1, report2);
    assert_eq!(report2, report3);
}

#[test]
fn leak_checker_different_shards_same_profile() {
    let checker = DeterministicLeakChecker;

    let work_report = checker.run_check("work", RouteProfile::Tor);
    let anon_report = checker.run_check("anon", RouteProfile::Tor);

    // Risk scores should be same, summaries different
    assert_eq!(work_report.risk_score, anon_report.risk_score);
    assert!(work_report.summary.contains("work"));
    assert!(anon_report.summary.contains("anon"));
}

// ============================================================================
// Route Profile Enum Behavior
// ============================================================================

#[test]
fn route_profile_hash_eq_works() {
    use std::collections::HashSet;

    let mut set: HashSet<RouteProfile> = HashSet::new();
    set.insert(RouteProfile::Offline);
    set.insert(RouteProfile::Direct);
    set.insert(RouteProfile::Tor);
    set.insert(RouteProfile::Vpn);
    set.insert(RouteProfile::Offline); // Duplicate

    assert_eq!(set.len(), 4);
    assert!(set.contains(&RouteProfile::Tor));
}

#[test]
fn route_profile_clone_copy_works() {
    let profile = RouteProfile::Tor;
    let cloned = profile.clone();
    let copied = profile;

    assert_eq!(profile, cloned);
    assert_eq!(profile, copied);
}

#[test]
fn route_profile_as_str() {
    assert_eq!(RouteProfile::Offline.as_str(), "Offline");
    assert_eq!(RouteProfile::Direct.as_str(), "Direct");
    assert_eq!(RouteProfile::Tor.as_str(), "Tor");
    assert_eq!(RouteProfile::Vpn.as_str(), "Vpn");
}

#[test]
fn route_profile_from_str() {
    assert_eq!("Offline".parse::<RouteProfile>(), Ok(RouteProfile::Offline));
    assert_eq!("Direct".parse::<RouteProfile>(), Ok(RouteProfile::Direct));
    assert_eq!("Tor".parse::<RouteProfile>(), Ok(RouteProfile::Tor));
    assert_eq!("Vpn".parse::<RouteProfile>(), Ok(RouteProfile::Vpn));
    assert!("Invalid".parse::<RouteProfile>().is_err());
}

// ============================================================================
// Audit Chain Integration
// ============================================================================

#[test]
fn audit_profile_applied_event_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Tor, &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.profile.applied")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("work"));
}

#[test]
fn audit_route_denied_event_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Direct, &mut audit);
    let _ = service.set_kill_switch(true, &mut audit);
    let _ = service.can_route("work", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.route.denied")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("kill-switch-enabled"));
}

#[test]
fn audit_route_allowed_event_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Tor, &mut audit);
    let _ = service.can_route("work", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.route.allowed")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("work"));
}

#[test]
fn audit_leak_check_event_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Direct, &mut audit);
    let _ = service.run_leak_check("work", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.leak-check")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("work"));
}

#[test]
fn audit_backend_failure_event_recorded() {
    let backend = NftablesRouteBackend::with_executor(
        NetworkBackendMode::Enforcing,
        MockExecutor::failing(),
    );
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit = AuditChain::default();

    let _ = service.apply_profile("work", RouteProfile::Tor, &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "netd.backend.failure")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("mock execution failure"));
}

// ============================================================================
// Network Backend Mode
// ============================================================================

#[test]
fn backend_mode_staged_constructor() {
    let backend = NftablesRouteBackend::<MockExecutor>::with_executor(
        NetworkBackendMode::Staged,
        MockExecutor::successful(),
    );
    // Staged mode doesn't execute commands immediately
}

#[test]
fn backend_mode_enforcing_constructor() {
    let backend = NftablesRouteBackend::<MockExecutor>::with_executor(
        NetworkBackendMode::Enforcing,
        MockExecutor::successful(),
    );
    // Enforcing mode would execute commands
}

// ============================================================================
// NetworkRuntimeState Persistence
// ============================================================================

#[test]
fn runtime_state_schema_version_is_correct() {
    assert_eq!(phantomkernel_netd::NetworkRuntimeState::CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(
        phantomkernel_netd::NetworkRuntimeState::STATE_KIND,
        "phantomkernel-netd-runtime"
    );
}
