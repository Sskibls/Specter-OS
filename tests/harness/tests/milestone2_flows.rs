use phantomkernel_airlockd::{AirlockService, ArtifactDescriptor, PluggableSanitizerChain};
use phantomkernel_netd::{
    DeterministicLeakChecker, NetworkPolicyError, NetworkPolicyService, NftablesRouteBackend,
    RouteProfile,
};
use phantomkernel_policyd::{CapabilityRequest, CapabilityRule, PolicyError, PolicyService};
use phantomkernel_shardd::{LinuxNamespaceStub, ShardManager};
use gk_audit::AuditChain;

fn low_risk_artifact(artifact_id: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: artifact_id.to_string(),
        path: format!("/tmp/{artifact_id}.pdf"),
        metadata_entries: 2,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFintegration".to_vec(),
    }
}

#[test]
fn deny_by_default_e2e_request_rejection() {
    let mut policy_service = PolicyService::new("integration-signing-key");
    let mut audit_chain = AuditChain::default();
    let request = CapabilityRequest {
        subject: "app://browser".to_string(),
        shard: "anon".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 30,
    };

    let result = policy_service.issue_token(&request, 100, &mut audit_chain);
    assert!(matches!(result, Err(PolicyError::DenyByDefault)));
}

#[test]
fn token_expiry_rejects_operation() {
    let mut policy_service = PolicyService::new("integration-signing-key");
    let mut audit_chain = AuditChain::default();

    let _ = policy_service.allow_rule(CapabilityRule::new(
        "app://browser",
        "anon",
        "network",
        "connect",
    ));

    let request = CapabilityRequest {
        subject: "app://browser".to_string(),
        shard: "anon".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 10,
    };

    let token = match policy_service.issue_token(&request, 200, &mut audit_chain) {
        Ok(token) => token,
        Err(error) => panic!("token issue unexpectedly failed: {error}"),
    };

    let result =
        policy_service.validate_token(&token, "anon", token.expires_at_epoch_s, &mut audit_chain);
    assert!(matches!(result, Err(PolicyError::ExpiredToken)));
}

#[test]
fn direct_cross_shard_copy_denied_unless_airlock_approved() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit_chain = AuditChain::default();

    let denied = service.request_direct_transfer("work", "anon", "artifact-copy", &mut audit_chain);
    assert!(denied.is_err());

    let session_id = service.open_session("work", "anon", &mut audit_chain);
    assert!(service
        .scan_session(
            &session_id,
            low_risk_artifact("artifact-copy"),
            &mut audit_chain
        )
        .is_ok());
    assert!(service
        .approve_session(&session_id, &mut audit_chain)
        .is_ok());
    assert!(service
        .commit_session(&session_id, &mut audit_chain)
        .is_ok());

    let allowed =
        service.request_direct_transfer("work", "anon", "artifact-copy", &mut audit_chain);
    assert!(allowed.is_ok());
}

#[test]
fn kill_switch_blocks_all_network_operations() {
    let backend = NftablesRouteBackend::new_staged();
    let mut service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    let mut audit_chain = AuditChain::default();

    assert!(service
        .apply_profile("work", RouteProfile::Direct, &mut audit_chain)
        .is_ok());
    assert!(service.set_kill_switch(true, &mut audit_chain).is_ok());

    let route_result = service.can_route("work", &mut audit_chain);
    assert!(matches!(
        route_result,
        Err(NetworkPolicyError::KillSwitchEnabled)
    ));
}

#[test]
fn audit_chain_receives_records_from_all_core_services() {
    let mut audit_chain = AuditChain::default();

    let mut policy_service = PolicyService::new("integration-signing-key");
    let _ = policy_service.allow_rule(CapabilityRule::new(
        "app://mail",
        "work",
        "network",
        "connect",
    ));
    let policy_request = CapabilityRequest {
        subject: "app://mail".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 30,
    };
    let token = match policy_service.issue_token(&policy_request, 10, &mut audit_chain) {
        Ok(token) => token,
        Err(error) => panic!("policy token issue failed: {error}"),
    };
    assert!(policy_service
        .validate_token(&token, "work", 20, &mut audit_chain)
        .is_ok());

    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    assert!(shard_manager
        .create_shard("work", 11, &mut audit_chain)
        .is_ok());
    assert!(shard_manager
        .start_shard("work", 12, &mut audit_chain)
        .is_ok());
    assert!(shard_manager
        .stop_shard("work", 13, &mut audit_chain)
        .is_ok());

    let backend = NftablesRouteBackend::new_staged();
    let mut net_service = NetworkPolicyService::new(DeterministicLeakChecker, backend);
    assert!(net_service
        .apply_profile("work", RouteProfile::Tor, &mut audit_chain)
        .is_ok());
    assert!(net_service.can_route("work", &mut audit_chain).is_ok());
    let leak_report = net_service.run_leak_check("work", &mut audit_chain);
    assert!(leak_report.clean);

    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let session_id = airlock_service.open_session("work", "anon", &mut audit_chain);
    assert!(airlock_service
        .scan_session(
            &session_id,
            low_risk_artifact("artifact-audit"),
            &mut audit_chain
        )
        .is_ok());
    assert!(airlock_service
        .approve_session(&session_id, &mut audit_chain)
        .is_ok());
    assert!(airlock_service
        .commit_session(&session_id, &mut audit_chain)
        .is_ok());

    let mut saw_policyd = false;
    let mut saw_shardd = false;
    let mut saw_netd = false;
    let mut saw_airlockd = false;

    for event in audit_chain.events() {
        saw_policyd |= event.event_type.starts_with("policyd.");
        saw_shardd |= event.event_type.starts_with("shardd.");
        saw_netd |= event.event_type.starts_with("netd.");
        saw_airlockd |= event.event_type.starts_with("airlockd.");
    }

    assert!(saw_policyd);
    assert!(saw_shardd);
    assert!(saw_netd);
    assert!(saw_airlockd);
}
