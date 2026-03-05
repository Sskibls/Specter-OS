//! Unit tests for phantomkernel-airlockd
//!
//! Coverage:
//! - MIME sniff mismatch rejection
//! - Sanitizer adapter failure handling
//! - Transfer session state machine
//! - High-risk artifact rejection
//! - Persistence and recovery

use phantomkernel_airlockd::{
    AirlockError, AirlockService, ArtifactDescriptor, MetadataStripAdapter, PluggableSanitizerChain,
    RiskScoringAdapter, SanitizerAdapter, SanitizerPipeline, SanitizerReport, SanitizationContext,
    SniffedMime, TransferSessionState,
};
use gk_audit::AuditChain;
use gk_persistence::PersistedState;

// ============================================================================
// MIME Sniffing
// ============================================================================

#[test]
fn mime_sniff_pdf_detected() {
    let bytes = b"%PDF-1.7\nSome PDF content";
    let sniffed = sniff_mime_helper(bytes);
    assert_eq!(sniffed, SniffedMime::Pdf);
}

#[test]
fn mime_sniff_png_detected() {
    let bytes = b"\x89PNG\r\n\x1a\nSome PNG content";
    let sniffed = sniff_mime_helper(bytes);
    assert_eq!(sniffed, SniffedMime::Png);
}

#[test]
fn mime_sniff_jpeg_detected() {
    let bytes = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10];
    let sniffed = sniff_mime_helper(&bytes);
    assert_eq!(sniffed, SniffedMime::Jpeg);
}

#[test]
fn mime_sniff_executable_detected() {
    let bytes = b"\x7fELF\x02\x01\x01\x00";
    let sniffed = sniff_mime_helper(bytes);
    assert_eq!(sniffed, SniffedMime::Executable);
}

#[test]
fn mime_sniff_plain_text_detected() {
    let bytes = b"This is plain text content\nwith newlines.";
    let sniffed = sniff_mime_helper(bytes);
    assert_eq!(sniffed, SniffedMime::PlainText);
}

#[test]
fn mime_sniff_unknown_binary() {
    let bytes = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
    let sniffed = sniff_mime_helper(&bytes);
    assert_eq!(sniffed, SniffedMime::Unknown);
}

#[test]
fn mime_sniff_empty_bytes_unknown() {
    let bytes: &[u8] = &[];
    let sniffed = sniff_mime_helper(bytes);
    assert_eq!(sniffed, SniffedMime::Unknown);
}

// Helper to access sniff_mime via public API
fn sniff_mime_helper(bytes: &[u8]) -> SniffedMime {
    // Create a test artifact and use the sanitizer to sniff
    let artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: bytes.to_vec(),
    };

    // Use default chain which will sniff internally
    let chain = PluggableSanitizerChain::default_chain();
    let report = chain.sanitize(&mut artifact.clone());
    
    // Map report sniffed_mime back to SniffedMime
    match report.sniffed_mime.as_str() {
        "application/pdf" => SniffedMime::Pdf,
        "image/png" => SniffedMime::Png,
        "image/jpeg" => SniffedMime::Jpeg,
        "text/plain" => SniffedMime::PlainText,
        "application/executable" => SniffedMime::Executable,
        _ => SniffedMime::Unknown,
    }
}

// ============================================================================
// MIME Mismatch Rejection
// ============================================================================

#[test]
fn mime_mismatch_declared_pdf_actual_text_increases_risk() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    // Declare as PDF but content is plain text
    let artifact = ArtifactDescriptor {
        artifact_id: "mismatch".to_string(),
        path: "/tmp/fake.pdf".to_string(),
        metadata_entries: 0,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"Just plain text content".to_vec(),
    };

    let report = service
        .scan_session(&session_id, artifact, &mut audit)
        .expect("scan should complete");

    // Risk should be elevated due to mismatch
    assert!(report.risk_score > 10);
}

#[test]
fn mime_mismatch_high_risk_threshold_rejection() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    // Declare as text but content is PDF with many metadata entries
    let artifact = ArtifactDescriptor {
        artifact_id: "risk-mismatch".to_string(),
        path: "/tmp/fake.txt".to_string(),
        metadata_entries: 20, // Max metadata risk
        declared_mime: "text/plain".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFpayload with mismatch".to_vec(),
    };

    let result = service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(result, Err(AirlockError::HighRiskRejected { .. })));
}

// ============================================================================
// Unknown MIME Rejection
// ============================================================================

#[test]
fn unknown_mime_executable_rejected() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    let artifact = ArtifactDescriptor {
        artifact_id: "elf-binary".to_string(),
        path: "/tmp/dropper".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: b"\x7fELFbinary content".to_vec(),
    };

    let result = service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(
        result,
        Err(AirlockError::UnknownMimeRejected { .. })
    ));
}

#[test]
fn unknown_mime_truly_unknown_rejected() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    let artifact = ArtifactDescriptor {
        artifact_id: "unknown-blob".to_string(),
        path: "/tmp/blob".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05],
    };

    let result = service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(
        result,
        Err(AirlockError::UnknownMimeRejected { .. })
    ));
}

#[test]
fn unknown_mime_session_state_set_to_rejected() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    let artifact = ArtifactDescriptor {
        artifact_id: "elf".to_string(),
        path: "/tmp/elf".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: b"\x7fELF".to_vec(),
    };

    let _ = service.scan_session(&session_id, artifact, &mut audit);

    assert_eq!(
        service.session_state(&session_id),
        Some(TransferSessionState::Rejected)
    );
}

// ============================================================================
// Sanitizer Adapter Behavior
// ============================================================================

#[test]
fn sanitizer_metadata_strip_removes_exif() {
    let adapter = MetadataStripAdapter;
    let mut artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test".to_string(),
        metadata_entries: 5,
        declared_mime: "image/jpeg".to_string(),
        content_bytes: b"headerEXIFmetadataEXIFmore".to_vec(),
    };

    let mut context = SanitizationContext::new(SniffedMime::PlainText);
    adapter.process(&mut artifact, &mut context);

    assert!(context.metadata_stripped);
    assert!(!artifact.content_bytes.windows(4).any(|w| w == b"EXIF"));
    assert_eq!(artifact.metadata_entries, 0);
}

#[test]
fn sanitizer_metadata_strip_no_op_without_markers() {
    let adapter = MetadataStripAdapter;
    let mut artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test".to_string(),
        metadata_entries: 0,
        declared_mime: "text/plain".to_string(),
        content_bytes: b"Just plain content without markers".to_vec(),
    };

    let mut context = SanitizationContext::new(SniffedMime::PlainText);
    adapter.process(&mut artifact, &mut context);

    assert!(!context.metadata_stripped);
    assert_eq!(artifact.metadata_entries, 0);
}

#[test]
fn sanitizer_risk_scoring_metadata_contribution() {
    let adapter = RiskScoringAdapter;
    let mut artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test".to_string(),
        metadata_entries: 10,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"%PDF-1.7".to_vec(),
    };

    let mut context = SanitizationContext::new(SniffedMime::Pdf);
    context.metadata_stripped = true; // Simulate metadata was stripped
    adapter.process(&mut artifact, &mut context);

    // Risk from metadata: min(10, 20) * 2 = 20
    // Risk from PDF: 10
    // Risk from stripped metadata: 30
    // Total: 60
    assert_eq!(context.risk_score, 60);
}

#[test]
fn sanitizer_risk_scoring_executable_max_risk() {
    let adapter = RiskScoringAdapter;
    let mut artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test".to_string(),
        metadata_entries: 0,
        declared_mime: "application/octet-stream".to_string(),
        content_bytes: b"\x7fELF".to_vec(),
    };

    let mut context = SanitizationContext::new(SniffedMime::Executable);
    adapter.process(&mut artifact, &mut context);

    // Executable risk: 95
    assert_eq!(context.risk_score, 95);
}

#[test]
fn sanitizer_risk_scoring_mismatch_contribution() {
    let adapter = RiskScoringAdapter;
    let mut artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test".to_string(),
        metadata_entries: 0,
        declared_mime: "text/plain".to_string(),
        content_bytes: b"%PDF-1.7".to_vec(),
    };

    let mut context = SanitizationContext::new(SniffedMime::Pdf);
    adapter.process(&mut artifact, &mut context);

    // PDF risk: 10
    // Mismatch risk: 35
    // Total: 45
    assert_eq!(context.risk_score, 45);
    assert!(context.notes.iter().any(|n| n.contains("mismatch")));
}

#[test]
fn sanitizer_chain_applies_all_adapters() {
    let chain = PluggableSanitizerChain::default_chain();
    let mut artifact = ArtifactDescriptor {
        artifact_id: "test".to_string(),
        path: "/tmp/test.pdf".to_string(),
        metadata_entries: 5,
        declared_mime: "application/pdf".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFmetadata".to_vec(),
    };

    let report = chain.sanitize(&mut artifact);

    assert!(report.metadata_stripped);
    assert!(!report.applied_steps.is_empty());
    assert!(report.applied_steps.contains(&"metadata-strip".to_string()));
    assert!(report.applied_steps.contains(&"risk-scoring".to_string()));
}

// ============================================================================
// Transfer Session State Machine
// ============================================================================

#[test]
fn session_state_machine_valid_flow() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    assert_eq!(
        service.session_state(&session_id),
        Some(TransferSessionState::Open)
    );

    let artifact = pdf_artifact("test");
    let _ = service.scan_session(&session_id, artifact, &mut audit);
    assert_eq!(
        service.session_state(&session_id),
        Some(TransferSessionState::Scanned)
    );

    let _ = service.approve_session(&session_id, &mut audit);
    assert_eq!(
        service.session_state(&session_id),
        Some(TransferSessionState::Approved)
    );

    let _ = service.commit_session(&session_id, &mut audit);
    assert_eq!(
        service.session_state(&session_id),
        Some(TransferSessionState::Committed)
    );
}

#[test]
fn session_state_machine_scan_on_non_open_fails() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("test");

    // First scan succeeds
    let _ = service.scan_session(&session_id, artifact.clone(), &mut audit);

    // Second scan should fail (state is Scanned, not Open)
    let result = service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(
        result,
        Err(AirlockError::InvalidState {
            expected: TransferSessionState::Open,
            actual: TransferSessionState::Scanned
        })
    ));
}

#[test]
fn session_state_machine_approve_on_non_scanned_fails() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    // Approve without scan should fail
    let result = service.approve_session(&session_id, &mut audit);
    assert!(matches!(
        result,
        Err(AirlockError::InvalidState {
            expected: TransferSessionState::Scanned,
            actual: TransferSessionState::Open
        })
    ));
}

#[test]
fn session_state_machine_commit_on_non_approved_fails() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("test");

    let _ = service.scan_session(&session_id, artifact, &mut audit);

    // Commit without approve should fail
    let result = service.commit_session(&session_id, &mut audit);
    assert!(matches!(
        result,
        Err(AirlockError::InvalidState {
            expected: TransferSessionState::Approved,
            actual: TransferSessionState::Scanned
        })
    ));
}

#[test]
fn session_state_machine_reject_valid_from_multiple_states() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    // Reject from Open state
    let session_id1 = service.open_session("work", "anon", &mut audit);
    let result = service.reject_session(&session_id1, "policy-decision", &mut audit);
    assert!(result.is_ok());

    // Reject from Scanned state
    let session_id2 = service.open_session("work", "anon", &mut audit);
    let _ = service.scan_session(&session_id2, pdf_artifact("test2"), &mut audit);
    let result = service.reject_session(&session_id2, "scan-failed", &mut audit);
    assert!(result.is_ok());
}

#[test]
fn session_not_found_error() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let result = service.scan_session(
        "nonexistent-session",
        pdf_artifact("test"),
        &mut audit,
    );
    assert!(matches!(result, Err(AirlockError::SessionNotFound)));

    let result = service.approve_session("nonexistent-session", &mut audit);
    assert!(matches!(result, Err(AirlockError::SessionNotFound)));

    let result = service.commit_session("nonexistent-session", &mut audit);
    assert!(matches!(result, Err(AirlockError::SessionNotFound)));
}

// ============================================================================
// High-Risk Artifact Rejection
// ============================================================================

#[test]
fn high_risk_threshold_is_70() {
    // Verify the constant is as expected
    // This is implicitly tested through rejection behavior
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    // Create artifact that should score >= 70
    // PDF (10) + metadata stripped (30) + mismatch (35) + metadata entries (20*2=40, capped)
    let artifact = ArtifactDescriptor {
        artifact_id: "high-risk".to_string(),
        path: "/tmp/risk.pdf".to_string(),
        metadata_entries: 20,
        declared_mime: "text/plain".to_string(),
        content_bytes: b"%PDF-1.7\nEXIFpayload".to_vec(),
    };

    let result = service.scan_session(&session_id, artifact, &mut audit);
    assert!(matches!(result, Err(AirlockError::HighRiskRejected { risk_score }) if risk_score >= 70));
}

#[test]
fn low_risk_artifact_passes() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    let artifact = pdf_artifact("low-risk");
    let report = service
        .scan_session(&session_id, artifact, &mut audit)
        .expect("low risk should pass");

    assert!(report.risk_score < 70);
}

// ============================================================================
// Direct Transfer Authorization
// ============================================================================

#[test]
fn direct_transfer_denied_without_approval() {
    let service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let result = service.request_direct_transfer("work", "anon", "artifact", &mut audit);
    assert!(matches!(result, Err(AirlockError::DirectTransferDenied)));
}

#[test]
fn direct_transfer_allowed_after_commit() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("approved");

    let _ = service.scan_session(&session_id, artifact.clone(), &mut audit);
    let _ = service.approve_session(&session_id, &mut audit);
    let _ = service.commit_session(&session_id, &mut audit);

    let result = service.request_direct_transfer("work", "anon", "approved", &mut audit);
    assert!(result.is_ok());
}

#[test]
fn direct_transfer_key_includes_artifact_id() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("specific-artifact");

    let _ = service.scan_session(&session_id, artifact.clone(), &mut audit);
    let _ = service.approve_session(&session_id, &mut audit);
    let _ = service.commit_session(&session_id, &mut audit);

    // Only this specific artifact should be allowed
    let result = service.request_direct_transfer("work", "anon", "specific-artifact", &mut audit);
    assert!(result.is_ok());

    // Different artifact should be denied
    let result = service.request_direct_transfer("work", "anon", "other-artifact", &mut audit);
    assert!(matches!(result, Err(AirlockError::DirectTransferDenied)));
}

// ============================================================================
// Persistence and Recovery
// ============================================================================

#[test]
fn runtime_state_schema_version_is_correct() {
    assert_eq!(
        phantomkernel_airlockd::AirlockRuntimeState::CURRENT_SCHEMA_VERSION,
        1
    );
    assert_eq!(
        phantomkernel_airlockd::AirlockRuntimeState::STATE_KIND,
        "phantomkernel-airlockd-runtime"
    );
}

#[test]
fn runtime_state_serialization_round_trip() {
    use phantomkernel_airlockd::AirlockRuntimeState;
    use std::collections::{HashMap, HashSet};

    let mut sessions = HashMap::new();
    sessions.insert(
        "session-1".to_string(),
        phantomkernel_airlockd::TransferSession {
            session_id: "session-1".to_string(),
            source_shard: "work".to_string(),
            target_shard: "anon".to_string(),
            state: TransferSessionState::Committed,
            artifact: None,
            scan_report: None,
        },
    );

    let mut approved = HashSet::new();
    approved.insert("work->anon:artifact-1".to_string());

    let state = AirlockRuntimeState {
        sessions,
        approved_transfers: approved,
        next_session_nonce: 42,
    };

    let json = serde_json::to_string(&state).expect("should serialize");
    let deserialized: AirlockRuntimeState = serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(state.next_session_nonce, deserialized.next_session_nonce);
    assert_eq!(
        state.approved_transfers,
        deserialized.approved_transfers
    );
}

#[test]
fn runtime_state_persistence_and_recovery() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("airlockd-state.json");

    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("persist-test");

    let _ = service.scan_session(&session_id, artifact.clone(), &mut audit);
    let _ = service.approve_session(&session_id, &mut audit);
    let _ = service.commit_session(&session_id, &mut audit);

    service
        .save_runtime_state(&path)
        .expect("state should save");

    let mut recovered = AirlockService::new(PluggableSanitizerChain::default_chain());
    recovered
        .load_runtime_state(&path)
        .expect("state should load");

    // Transfer should still be allowed after recovery
    let result =
        recovered.request_direct_transfer("work", "anon", "persist-test", &mut audit);
    assert!(result.is_ok());
}

// ============================================================================
// Audit Chain Integration
// ============================================================================

#[test]
fn audit_session_opened_event_recorded() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.opened")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains(&session_id));
    assert!(events[0].payload.contains("work->anon"));
}

#[test]
fn audit_session_scanned_event_recorded() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let _ = service.scan_session(&session_id, pdf_artifact("test"), &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.scanned")
        .collect();

    assert_eq!(events.len(), 1);
}

#[test]
fn audit_session_approved_event_recorded() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let _ = service.scan_session(&session_id, pdf_artifact("test"), &mut audit);
    let _ = service.approve_session(&session_id, &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.approved")
        .collect();

    assert_eq!(events.len(), 1);
}

#[test]
fn audit_session_committed_event_recorded() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let _ = service.scan_session(&session_id, pdf_artifact("test"), &mut audit);
    let _ = service.approve_session(&session_id, &mut audit);
    let _ = service.commit_session(&session_id, &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.committed")
        .collect();

    assert_eq!(events.len(), 1);
}

#[test]
fn audit_session_rejected_event_recorded() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let _ = service.reject_session(&session_id, "test-reason", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.session.rejected")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("test-reason"));
}

#[test]
fn audit_transfer_denied_event_recorded() {
    let service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let _ = service.request_direct_transfer("work", "anon", "artifact", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.transfer.denied")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("work->anon:artifact"));
}

#[test]
fn audit_transfer_allowed_event_recorded() {
    let mut service = AirlockService::new(PluggableSanitizerChain::default_chain());
    let mut audit = AuditChain::default();

    let session_id = service.open_session("work", "anon", &mut audit);
    let artifact = pdf_artifact("audit-test");

    let _ = service.scan_session(&session_id, artifact.clone(), &mut audit);
    let _ = service.approve_session(&session_id, &mut audit);
    let _ = service.commit_session(&session_id, &mut audit);
    let _ = service.request_direct_transfer("work", "anon", "audit-test", &mut audit);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "airlockd.transfer.allowed")
        .collect();

    assert_eq!(events.len(), 1);
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
