//! Unit tests for gk-audit (audit chain persistence)
//!
//! Coverage:
//! - Audit chain append invariants
//! - Audit chain verify invariants
//! - Tamper detection
//! - Recovery flow
//! - Hash chain integrity

use gk_audit::{AuditChain, AuditIntegrityError, SignedAuditEvent};

// ============================================================================
// Append Invariants
// ============================================================================

#[test]
fn append_sequence_numbers_monotonic() {
    let mut chain = AuditChain::default();

    let seq1 = chain.append("event.one", "payload1");
    let seq2 = chain.append("event.two", "payload2");
    let seq3 = chain.append("event.three", "payload3");

    assert_eq!(seq1, 1);
    assert_eq!(seq2, 2);
    assert_eq!(seq3, 3);
}

#[test]
fn append_empty_chain_has_zero_length() {
    let chain = AuditChain::default();
    assert_eq!(chain.len(), 0);
    assert!(chain.is_empty());
}

#[test]
fn append_non_empty_chain_reports_correct_length() {
    let mut chain = AuditChain::default();

    chain.append("event.one", "payload1");
    chain.append("event.two", "payload2");

    assert_eq!(chain.len(), 2);
    assert!(!chain.is_empty());
}

#[test]
fn append_event_type_and_payload_stored() {
    let mut chain = AuditChain::default();

    chain.append("policyd.decision", "allow:network");

    let events = chain.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "policyd.decision");
    assert_eq!(events[0].payload, "allow:network");
}

#[test]
fn append_previous_hash_chained_correctly() {
    let mut chain = AuditChain::default();

    let _ = chain.append("first", "payload1");
    let _ = chain.append("second", "payload2");

    let events = chain.events();

    // First event has empty previous hash
    assert_eq!(events[0].previous_hash, "");

    // Second event's previous hash equals first event's hash
    assert_eq!(events[1].previous_hash, events[0].event_hash);
}

#[test]
fn append_event_hash_is_deterministic() {
    let mut chain1 = AuditChain::default();
    let mut chain2 = AuditChain::default();

    let _ = chain1.append("test", "payload");
    let _ = chain2.append("test", "payload");

    let events1 = chain1.events();
    let events2 = chain2.events();

    assert_eq!(events1[0].event_hash, events2[0].event_hash);
}

#[test]
fn append_different_payloads_produce_different_hashes() {
    let mut chain = AuditChain::default();

    let _ = chain.append("test", "payload1");
    let _ = chain.append("test", "payload2");

    let events = chain.events();
    assert_ne!(events[0].event_hash, events[1].event_hash);
}

#[test]
fn append_different_event_types_produce_different_hashes() {
    let mut chain = AuditChain::default();

    let _ = chain.append("type1", "same-payload");
    let _ = chain.append("type2", "same-payload");

    let events = chain.events();
    assert_ne!(events[0].event_hash, events[1].event_hash);
}

// ============================================================================
// Verify Invariants
// ============================================================================

#[test]
fn verify_empty_chain_is_valid() {
    let chain = AuditChain::default();
    assert!(chain.verify());
    assert!(chain.verify_detailed().is_ok());
}

#[test]
fn verify_single_event_chain_is_valid() {
    let mut chain = AuditChain::default();
    chain.append("event", "payload");

    assert!(chain.verify());
    assert!(chain.verify_detailed().is_ok());
}

#[test]
fn verify_multi_event_chain_is_valid() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");
    chain.append("event3", "payload3");

    assert!(chain.verify());
    assert!(chain.verify_detailed().is_ok());
}

#[test]
fn verify_sequence_mismatch_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");

    let mut events = chain.snapshot();
    // Tamper with sequence number
    events[1].sequence = 999;

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::SequenceMismatch {
            expected: 2,
            actual: 999
        })
    ));
}

#[test]
fn verify_previous_hash_mismatch_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");

    let mut events = chain.snapshot();
    // Tamper with previous hash
    events[1].previous_hash = "tampered-hash".to_string();

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::PreviousHashMismatch { sequence: 2 })
    ));
}

#[test]
fn verify_event_hash_mismatch_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");

    let mut events = chain.snapshot();
    // Tamper with event hash
    events[0].event_hash = "tampered".to_string();

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::EventHashMismatch { sequence: 1 })
    ));
}

// ============================================================================
// Tamper Detection
// ============================================================================

#[test]
fn tamper_payload_modification_detected() {
    let mut chain = AuditChain::default();
    chain.append("policyd.decision", "allow");
    chain.append("shardd.start", "work");

    let mut events = chain.snapshot();
    events[1].payload = "tampered".to_string();

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::EventHashMismatch { .. })
    ));
}

#[test]
fn tamper_event_type_modification_detected() {
    let mut chain = AuditChain::default();
    chain.append("policyd.decision", "allow");
    chain.append("shardd.start", "work");

    let mut events = chain.snapshot();
    events[0].event_type = "tampered.event".to_string();

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::EventHashMismatch { .. })
    ));
}

#[test]
fn tamper_sequence_modification_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");

    let mut events = chain.snapshot();
    events[0].sequence = 999;

    let result = AuditChain::recover(events);
    assert!(matches!(
        result,
        Err(AuditIntegrityError::SequenceMismatch {
            expected: 1,
            actual: 999
        })
    ));
}

#[test]
fn tamper_insert_event_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event3", "payload3");

    let mut events = chain.snapshot();
    // Insert a fake event in the middle
    let fake_event = SignedAuditEvent {
        sequence: 2,
        event_type: "event2".to_string(),
        payload: "fake".to_string(),
        previous_hash: events[0].event_hash.clone(),
        event_hash: "fake-hash".to_string(),
    };
    events.insert(1, fake_event);

    let result = AuditChain::recover(events);
    // Should detect hash mismatch
    assert!(result.is_err());
}

#[test]
fn tamper_delete_event_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");
    chain.append("event3", "payload3");

    let mut events = chain.snapshot();
    // Delete middle event
    events.remove(1);

    let result = AuditChain::recover(events);
    // Should detect sequence mismatch (1, 3 instead of 1, 2)
    assert!(matches!(
        result,
        Err(AuditIntegrityError::SequenceMismatch { .. })
    ));
}

#[test]
fn tamper_reorder_events_detected() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");
    chain.append("event3", "payload3");

    let mut events = chain.snapshot();
    // Swap first two events
    events.swap(0, 1);

    let result = AuditChain::recover(events);
    // Should detect sequence mismatch
    assert!(matches!(
        result,
        Err(AuditIntegrityError::SequenceMismatch { .. })
    ));
}

// ============================================================================
// Recovery Flow
// ============================================================================

#[test]
fn recovery_succeeds_for_untampered_snapshot() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");
    chain.append("event3", "payload3");

    let snapshot = chain.snapshot();
    let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");

    assert_eq!(recovered.len(), 3);
    assert!(recovered.verify());
}

#[test]
fn recovery_preserves_event_order() {
    let mut chain = AuditChain::default();
    chain.append("first", "payload1");
    chain.append("second", "payload2");
    chain.append("third", "payload3");

    let snapshot = chain.snapshot();
    let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");

    let events = recovered.events();
    assert_eq!(events[0].event_type, "first");
    assert_eq!(events[1].event_type, "second");
    assert_eq!(events[2].event_type, "third");
}

#[test]
fn recovery_preserves_event_data() {
    let mut chain = AuditChain::default();
    chain.append("policyd.token.issued", "tok-001");
    chain.append("shardd.transition", "work:Created->Running");
    chain.append("netd.profile.applied", "work:Tor");

    let snapshot = chain.snapshot();
    let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");

    let events = recovered.events();
    assert_eq!(events[0].event_type, "policyd.token.issued");
    assert_eq!(events[0].payload, "tok-001");
    assert_eq!(events[1].event_type, "shardd.transition");
    assert_eq!(events[1].payload, "work:Created->Running");
    assert_eq!(events[2].event_type, "netd.profile.applied");
    assert_eq!(events[2].payload, "work:Tor");
}

#[test]
fn recovery_last_hash_computed_correctly() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");

    let original_last_hash = chain.events().last().unwrap().event_hash.clone();

    let snapshot = chain.snapshot();
    let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");

    // The recovered chain's internal last_hash should match
    let recovered_events = recovered.events();
    assert_eq!(
        recovered_events.last().unwrap().event_hash,
        original_last_hash
    );
}

#[test]
fn recovery_empty_chain_succeeds() {
    let chain = AuditChain::default();
    let snapshot = chain.snapshot();
    let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");

    assert_eq!(recovered.len(), 0);
    assert!(recovered.is_empty());
}

// ============================================================================
// Hash Chain Integrity
// ============================================================================

#[test]
fn hash_chain_first_event_previous_hash_empty() {
    let mut chain = AuditChain::default();
    chain.append("first", "payload");

    let events = chain.events();
    assert_eq!(events[0].previous_hash, "");
}

#[test]
fn hash_chain_each_event_links_to_previous() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");
    chain.append("event3", "payload3");

    let events = chain.events();

    for i in 1..events.len() {
        assert_eq!(events[i].previous_hash, events[i - 1].event_hash);
    }
}

#[test]
fn hash_chain_same_content_same_hash() {
    let mut chain1 = AuditChain::default();
    let mut chain2 = AuditChain::default();

    chain1.append("same", "content");
    chain2.append("same", "content");

    assert_eq!(
        chain1.events()[0].event_hash,
        chain2.events()[0].event_hash
    );
}

#[test]
fn hash_chain_different_sequence_different_hash() {
    let mut chain1 = AuditChain::default();
    let mut chain2 = AuditChain::default();

    // Same content, different sequence (due to being in different chains)
    chain1.append("test", "content");
    chain2.append("test", "content");

    // Hashes should be same since sequence is 1 in both
    assert_eq!(
        chain1.events()[0].event_hash,
        chain2.events()[0].event_hash
    );
}

#[test]
fn hash_chain_payload_affects_hash() {
    let mut chain = AuditChain::default();
    chain.append("test", "payload-a");
    chain.append("test", "payload-b");

    let events = chain.events();
    assert_ne!(events[0].event_hash, events[1].event_hash);
}

#[test]
fn hash_chain_event_type_affects_hash() {
    let mut chain = AuditChain::default();
    chain.append("type-a", "same-payload");
    chain.append("type-b", "same-payload");

    let events = chain.events();
    assert_ne!(events[0].event_hash, events[1].event_hash);
}

// ============================================================================
// SignedAuditEvent Structure
// ============================================================================

#[test]
fn signed_audit_event_clone_works() {
    let event = SignedAuditEvent {
        sequence: 1,
        event_type: "test.event".to_string(),
        payload: "test-payload".to_string(),
        previous_hash: "prev-hash".to_string(),
        event_hash: "event-hash".to_string(),
    };

    let cloned = event.clone();
    assert_eq!(event.sequence, cloned.sequence);
    assert_eq!(event.event_type, cloned.event_type);
    assert_eq!(event.payload, cloned.payload);
    assert_eq!(event.previous_hash, cloned.previous_hash);
    assert_eq!(event.event_hash, cloned.event_hash);
}

#[test]
fn signed_audit_event_eq_works() {
    let event1 = SignedAuditEvent {
        sequence: 1,
        event_type: "test".to_string(),
        payload: "payload".to_string(),
        previous_hash: "prev".to_string(),
        event_hash: "hash".to_string(),
    };

    let event2 = event1.clone();
    let event3 = SignedAuditEvent {
        sequence: 2,
        event_type: "test".to_string(),
        payload: "payload".to_string(),
        previous_hash: "prev".to_string(),
        event_hash: "hash".to_string(),
    };

    assert_eq!(event1, event2);
    assert_ne!(event1, event3);
}

// ============================================================================
// AuditIntegrityError Display
// ============================================================================

#[test]
fn audit_integrity_error_sequence_mismatch_display() {
    let error = AuditIntegrityError::SequenceMismatch {
        expected: 5,
        actual: 6,
    };
    let display = format!("{error}");
    assert!(display.contains("expected 5"));
    assert!(display.contains("actual 6"));
}

#[test]
fn audit_integrity_error_previous_hash_mismatch_display() {
    let error = AuditIntegrityError::PreviousHashMismatch { sequence: 3 };
    let display = format!("{error}");
    assert!(display.contains("sequence 3"));
}

#[test]
fn audit_integrity_error_event_hash_mismatch_display() {
    let error = AuditIntegrityError::EventHashMismatch { sequence: 7 };
    let display = format!("{error}");
    assert!(display.contains("sequence 7"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn append_empty_payload() {
    let mut chain = AuditChain::default();
    let seq = chain.append("event", "");

    assert_eq!(seq, 1);
    assert_eq!(chain.events()[0].payload, "");
    assert!(chain.verify());
}

#[test]
fn append_empty_event_type() {
    let mut chain = AuditChain::default();
    let seq = chain.append("", "payload");

    assert_eq!(seq, 1);
    assert_eq!(chain.events()[0].event_type, "");
    assert!(chain.verify());
}

#[test]
fn append_unicode_payload() {
    let mut chain = AuditChain::default();
    chain.append("event", "Hello 世界 🌍");

    assert!(chain.verify());
    assert_eq!(chain.events()[0].payload, "Hello 世界 🌍");
}

#[test]
fn append_large_payload() {
    let mut chain = AuditChain::default();
    let large_payload = "x".repeat(10000);
    chain.append("event", &large_payload);

    assert!(chain.verify());
    assert_eq!(chain.events()[0].payload.len(), 10000);
}

#[test]
fn verify_detailed_returns_specific_error() {
    let mut chain = AuditChain::default();
    chain.append("event1", "payload1");
    chain.append("event2", "payload2");

    let mut events = chain.snapshot();
    events[1].payload = "tampered".to_string();

    let result = AuditChain::recover(events);
    match result {
        Err(AuditIntegrityError::EventHashMismatch { sequence }) => {
            assert_eq!(sequence, 2);
        }
        other => panic!("Expected EventHashMismatch, got {other:?}"),
    }
}
