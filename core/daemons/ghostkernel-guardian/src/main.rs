use phantomkernel_guardian::GuardianService;
use phantomkernel_netd::NftablesRouteBackend;
use phantomkernel_shardd::ShardManager;

fn main() {
    println!("phantomkernel-guardian daemon");

    // Initialize components
    let network_backend = NftablesRouteBackend::new_staged();
    let shard_manager = ShardManager::new(phantomkernel_shardd::LinuxNamespaceStub);
    let _guardian = GuardianService::new(network_backend, shard_manager);

    // Example usage (would be triggered by IPC in real implementation)
    // guardian.panic(&mut audit_chain);
    // guardian.mask("decoy", &mut audit_chain);
    // guardian.set_travel_mode(true, &mut audit_chain);

    println!("Guardian service initialized and ready");
}
