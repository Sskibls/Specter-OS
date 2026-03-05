// Integration tests for PhantomKernel Guardian
//
// Tests emergency modes functionality

use phantomkernel_guardian::GuardianService;
use phantomkernel_netd::{MockCommandExecutor, NftablesRouteBackend};
use phantomkernel_shardd::{LinuxNamespaceStub, ShardManager};
use gk_audit::AuditChain;
use std::time::Instant;

#[test]
fn test_panic_mode_kills_network_quickly() {
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
    
    // Verify network kill switch was activated (would be checked in audit events)
    let has_network_kill_event = audit_chain.events().iter()
        .any(|event| event.event_type == "guardian.panic.network_killed");
    assert!(has_network_kill_event, "Network kill event not found in audit chain");
}

#[test]
fn test_mask_mode_workspace_switching() {
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
fn test_travel_mode_toggle() {
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