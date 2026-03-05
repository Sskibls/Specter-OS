use std::path::PathBuf;

use phantomkernel_airlockd::{AirlockService, ArtifactDescriptor, PluggableSanitizerChain};
use phantomkernel_auditd::AuditDaemon;
use phantomkernel_netd::{
    DeterministicLeakChecker, NetworkPolicyService, NftablesRouteBackend, RouteProfile,
};
use phantomkernel_policyd::{CapabilityRequest, CapabilityRule, PolicyService};
use phantomkernel_shardd::{LinuxNamespaceStub, ShardManager, ShardState};
use gk_audit::AuditChain;
use gk_config::{ensure_runtime_layout, load_layered, validate_runtime_layout, RuntimePaths};

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn low_risk_pdf_artifact(artifact_id: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_id: artifact_id.to_string(),
        path: format!("/tmp/{artifact_id}.pdf"),
        metadata_entries: 2,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFsmoke".to_vec(),
    }
}

fn run_edition_smoke(edition_layer: &str, expected_edition: &str, profile: RouteProfile) {
    let runtime_root = tempfile::tempdir().expect("tempdir should be created");
    let runtime_paths = RuntimePaths::from_root(runtime_root.path());
    ensure_runtime_layout(&runtime_paths).expect("runtime layout should be created");
    validate_runtime_layout(&runtime_paths).expect("runtime layout should be writable");

    let config = load_layered(&[
        workspace_path("editions/shared/defaults/default.toml"),
        workspace_path(edition_layer),
    ])
    .expect("layered config should load");
    assert_eq!(config.edition.name, expected_edition);

    let audit_daemon = AuditDaemon::open(&runtime_paths.data_dir.join("auditd/chain.log"))
        .expect("audit daemon should start");
    let _ = audit_daemon
        .append_event("smoke.start", expected_edition)
        .expect("startup audit event should append");

    let mut policy_service = PolicyService::new("smoke-signing-key");
    let mut audit_chain = AuditChain::default();
    let _ = policy_service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 30,
    };
    let token = policy_service
        .issue_token(&request, 100, &mut audit_chain)
        .expect("policy token should issue");

    let policy_state_path = runtime_paths.data_dir.join("policyd/state.json");
    policy_service
        .save_runtime_state(&policy_state_path)
        .expect("policy state should save");

    let mut policy_recovered = PolicyService::new("smoke-signing-key");
    policy_recovered
        .load_runtime_state(&policy_state_path)
        .expect("policy state should load");
    policy_recovered
        .validate_token(&token, "work", 101, &mut audit_chain)
        .expect("recovered policy should validate token");
    let _ = audit_daemon
        .append_event("smoke.policy", "ok")
        .expect("policy audit should append");

    let mut shard_manager = ShardManager::new(LinuxNamespaceStub);
    shard_manager
        .create_shard("work", 102, &mut audit_chain)
        .expect("shard should create");
    shard_manager
        .start_shard("work", 103, &mut audit_chain)
        .expect("shard should start");
    let shard_state_path = runtime_paths.data_dir.join("shardd/state.json");
    shard_manager
        .save_runtime_state(&shard_state_path)
        .expect("shard state should save");

    let mut shard_recovered = ShardManager::new(LinuxNamespaceStub);
    shard_recovered
        .load_runtime_state(&shard_state_path)
        .expect("shard state should load");
    assert_eq!(shard_recovered.state_of("work"), Some(ShardState::Running));
    let _ = audit_daemon
        .append_event("smoke.shard", "ok")
        .expect("shard audit should append");

    let mut net_service =
        NetworkPolicyService::new(DeterministicLeakChecker, NftablesRouteBackend::new_staged());
    net_service
        .apply_profile("work", profile, &mut audit_chain)
        .expect("network profile should apply");
    let net_state_path = runtime_paths.data_dir.join("netd/state.json");
    net_service
        .save_runtime_state(&net_state_path)
        .expect("network state should save");

    let mut net_recovered =
        NetworkPolicyService::new(DeterministicLeakChecker, NftablesRouteBackend::new_staged());
    net_recovered
        .load_runtime_state(&net_state_path, &mut audit_chain)
        .expect("network state should load");
    assert_eq!(net_recovered.profile_of("work"), Some(profile));
    assert!(net_recovered.can_route("work", &mut audit_chain).is_ok());
    let _ = audit_daemon
        .append_event("smoke.net", "ok")
        .expect("net audit should append");

    let mut airlock_service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let session_id = airlock_service.open_session("work", "anon", &mut audit_chain);
    let _ = airlock_service
        .scan_session(
            &session_id,
            low_risk_pdf_artifact("smoke-artifact"),
            &mut audit_chain,
        )
        .expect("airlock scan should pass");
    airlock_service
        .approve_session(&session_id, &mut audit_chain)
        .expect("airlock session should approve");
    airlock_service
        .commit_session(&session_id, &mut audit_chain)
        .expect("airlock session should commit");
    let airlock_state_path = runtime_paths.data_dir.join("airlockd/state.json");
    airlock_service
        .save_runtime_state(&airlock_state_path)
        .expect("airlock state should save");

    let mut airlock_recovered = AirlockService::new(PluggableSanitizerChain::default_chain());
    airlock_recovered
        .load_runtime_state(&airlock_state_path)
        .expect("airlock state should load");
    assert!(airlock_recovered
        .request_direct_transfer("work", "anon", "smoke-artifact", &mut audit_chain)
        .is_ok());
    let _ = audit_daemon
        .append_event("smoke.airlock", "ok")
        .expect("airlock audit should append");

    let verify_result = audit_daemon
        .verify_chain()
        .expect("audit verify should run");
    assert!(verify_result.valid);
}

#[test]
fn debian_runtime_smoke_startup_and_core_workflow() {
    run_edition_smoke(
        "editions/debian/defaults/debian.toml",
        "debian",
        RouteProfile::Tor,
    );
}

#[test]
fn fedora_runtime_smoke_startup_and_core_workflow() {
    run_edition_smoke(
        "editions/fedora/defaults/fedora.toml",
        "fedora",
        RouteProfile::Vpn,
    );
}
