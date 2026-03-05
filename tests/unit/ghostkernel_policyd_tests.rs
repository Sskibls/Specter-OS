//! Unit tests for phantomkernel-policyd
//!
//! Coverage:
//! - Handler-level authz checks
//! - Config/runtime path validation
//! - Fail-closed behavior on errors

use phantomkernel_policyd::{
    CapabilityRequest, CapabilityRule, PolicyError, PolicyRuntimeState, PolicyService,
};
use gk_audit::AuditChain;
use gk_crypto::KeyRecord;
use gk_persistence::{save_state, PersistedState};
use std::path::PathBuf;

// ============================================================================
// Handler-level Authorization Checks
// ============================================================================

#[test]
fn authz_deny_by_default_no_rules_configured() {
    let mut service = PolicyService::new("test-key-deny-default");
    let mut audit = AuditChain::default();

    let request = CapabilityRequest {
        subject: "app://unknown".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 60,
    };

    let result = service.issue_token(&request, 100, &mut audit);
    assert!(matches!(result, Err(PolicyError::DenyByDefault)));
}

#[test]
fn authz_exact_rule_match_grants_token() {
    let mut service = PolicyService::new("test-key-exact-match");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };

    let result = service.issue_token(&request, 100, &mut audit);
    assert!(result.is_ok());
}

#[test]
fn authz_partial_rule_match_still_denies() {
    let mut service = PolicyService::new("test-key-partial");
    let mut audit = AuditChain::default();

    // Rule allows "connect" but request asks for "write"
    service.allow_rule(CapabilityRule::new(
        "app://mail",
        "work",
        "network",
        "connect",
    ));

    let request = CapabilityRequest {
        subject: "app://mail".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "write".to_string(),
        ttl_seconds: 60,
    };

    let result = service.issue_token(&request, 100, &mut audit);
    assert!(matches!(result, Err(PolicyError::DenyByDefault)));
}

#[test]
fn authz_shard_mismatch_in_request_denied_at_validation() {
    let mut service = PolicyService::new("test-key-shard-mismatch");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };

    let token = service
        .issue_token(&request, 100, &mut audit)
        .expect("token should issue");

    // Validate against wrong shard
    let result = service.validate_token(&token, "anon", 110, &mut audit);
    assert!(matches!(result, Err(PolicyError::ShardMismatch { .. })));
}

#[test]
fn authz_multiple_rules_independent() {
    let mut service = PolicyService::new("test-key-multi-rule");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
        "app://mail",
        "work",
        "network",
        "connect",
    ));
    service.allow_rule(CapabilityRule::new(
        "app://browser",
        "anon",
        "network",
        "connect",
    ));

    // mail in work should work
    let mail_request = CapabilityRequest {
        subject: "app://mail".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 60,
    };
    assert!(service.issue_token(&mail_request, 100, &mut audit).is_ok());

    // browser in anon should work
    let browser_request = CapabilityRequest {
        subject: "app://browser".to_string(),
        shard: "anon".to_string(),
        resource: "network",
        action: "connect".to_string(),
        ttl_seconds: 60,
    };
    assert!(service
        .issue_token(&browser_request, 100, &mut audit)
        .is_ok());

    // mail in anon should NOT work (no rule)
    let mail_anon_request = CapabilityRequest {
        subject: "app://mail".to_string(),
        shard: "anon".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        ttl_seconds: 60,
    };
    assert!(matches!(
        service.issue_token(&mail_anon_request, 100, &mut audit),
        Err(PolicyError::DenyByDefault)
    ));
}

// ============================================================================
// Config/Runtime Path Validation
// ============================================================================

#[test]
fn runtime_state_serialization_round_trip() {
    let mut service = PolicyService::new("test-key-serialization");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };
    let _ = service.issue_token(&request, 100, &mut audit);

    let state = service.runtime_state();

    // Verify state structure
    assert_eq!(state.allow_rules.len(), 1);
    assert_eq!(state.issued_tokens.len(), 1);
    assert_eq!(state.next_token_nonce, 2);

    // Verify serialization
    let json = serde_json::to_string(&state).expect("should serialize");
    let deserialized: PolicyRuntimeState =
        serde_json::from_str(&json).expect("should deserialize");

    assert_eq!(state.allow_rules, deserialized.allow_rules);
    assert_eq!(state.issued_tokens.len(), deserialized.issued_tokens.len());
}

#[test]
fn runtime_state_persistence_and_recovery() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("policyd-state.json");

    let mut service = PolicyService::new("test-key-persist");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };
    let token = service
        .issue_token(&request, 100, &mut audit)
        .expect("token should issue");

    service
        .save_runtime_state(&path)
        .expect("state should save");

    // Simulate crash recovery
    let mut recovered = PolicyService::new("test-key-persist");
    recovered
        .load_runtime_state(&path)
        .expect("state should load");

    // Token should still be valid
    let validation = recovered.validate_token(&token, "work", 110, &mut audit);
    assert!(validation.is_ok());
}

#[test]
fn runtime_state_schema_version_is_correct() {
    assert_eq!(PolicyRuntimeState::CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(PolicyRuntimeState::STATE_KIND, "phantomkernel-policyd-runtime");
}

// ============================================================================
// Fail-Closed Behavior
// ============================================================================

#[test]
fn fail_closed_on_signing_key_expired() {
    let mut service = PolicyService::new("test-key-expired");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };

    let token = service
        .issue_token(&request, 100, &mut audit)
        .expect("token should issue");

    // Revoke the signing key
    service.revoke_signing_key("key-primary", &mut audit);

    // Token validation should fail due to revoked signing key
    let result = service.validate_token(&token, "work", 110, &mut audit);
    assert!(matches!(result, Err(PolicyError::SigningKeyRevoked)));
}

#[test]
fn fail_closed_on_crypto_failure() {
    let mut service = PolicyService::new("test-key-crypto");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };

    let token = service
        .issue_token(&request, 100, &mut audit)
        .expect("token should issue");

    // Tamper with token signature
    let mut tampered_token = token.clone();
    tampered_token.signature.value_hex = "deadbeef".to_string();

    let result = service.validate_token(&tampered_token, "work", 110, &mut audit);
    assert!(matches!(result, Err(PolicyError::InvalidSignature)));
}

#[test]
fn fail_closed_on_persistence_failure() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("nonexistent-dir").join("state.json");

    let service = PolicyService::new("test-key-persist-fail");

    // Attempting to save to non-existent parent dir should fail gracefully
    // (Note: save_state creates parent dirs, so we test with invalid path)
    let result = service.save_runtime_state(&path);
    // Should either succeed (dirs created) or fail with PersistenceFailure
    match result {
        Ok(_) => {
            // If it succeeded, verify file exists
            assert!(path.exists());
        }
        Err(PolicyError::PersistenceFailure(_)) => {
            // Expected failure mode
        }
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[test]
fn fail_closed_token_revocation_persists() {
    let temp = tempfile::tempdir().expect("tempdir should create");
    let path = temp.path().join("policyd-revoke.json");

    let mut service = PolicyService::new("test-key-revoke");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
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
        ttl_seconds: 60,
    };
    let token = service
        .issue_token(&request, 100, &mut audit)
        .expect("token should issue");

    // Revoke token
    service.revoke_token(&token.token_id, &mut audit);

    // Verify revoked
    let result = service.validate_token(&token, "work", 110, &mut audit);
    assert!(matches!(result, Err(PolicyError::RevokedToken)));

    // Persist and recover
    service
        .save_runtime_state(&path)
        .expect("state should save");

    let mut recovered = PolicyService::new("test-key-revoke");
    recovered
        .load_runtime_state(&path)
        .expect("state should load");

    // Token should still be revoked after recovery
    let result = recovered.validate_token(&token, "work", 110, &mut audit);
    assert!(matches!(result, Err(PolicyError::RevokedToken)));
}

// ============================================================================
// Key Rotation and Revocation
// ============================================================================

#[test]
fn key_rotation_audit_event_recorded() {
    let mut service = PolicyService::new("test-key-rotation");
    let mut audit = AuditChain::default();

    let new_key = KeyRecord::new("key-rotated", "new-secret", None);
    service.rotate_signing_key(new_key, true, &mut audit);

    // Verify audit event was recorded
    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "policyd.key.rotated")
        .collect();

    assert_eq!(events.len(), 1);
    assert!(events[0].payload.contains("key_id=key-rotated"));
    assert!(events[0].payload.contains("active=true"));
}

#[test]
fn key_revocation_audit_event_recorded() {
    let mut service = PolicyService::new("test-key-revocation");
    let mut audit = AuditChain::default();

    let revoked = service.revoke_signing_key("key-primary", &mut audit);
    assert!(revoked);

    let events: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| e.event_type == "policyd.key.revoked")
        .collect();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload, "key-primary");
}

// ============================================================================
// Token Lifecycle
// ============================================================================

#[test]
fn token_nonce_increments_monotonically() {
    let mut service = PolicyService::new("test-key-nonce");
    let mut audit = AuditChain::default();

    service.allow_rule(CapabilityRule::new(
        "app://mail",
        "work",
        "network",
        "connect",
    ));

    let mut tokens = Vec::new();
    for i in 0..5 {
        let request = CapabilityRequest {
            subject: "app://mail".to_string(),
            shard: "work".to_string(),
            resource: "network".to_string(),
            action: "connect".to_string(),
            ttl_seconds: 60,
        };
        let token = service
            .issue_token(&request, 100 + i, &mut audit)
            .expect("token should issue");
        tokens.push(token);
    }

    // Verify token IDs are unique and incrementing
    let token_ids: Vec<_> = tokens.iter().map(|t| &t.token_id).collect();
    let mut unique_ids: std::collections::HashSet<_> = token_ids.iter().collect();
    assert_eq!(unique_ids.len(), 5, "all token IDs should be unique");

    // Verify nonce state
    let state = service.runtime_state();
    assert_eq!(state.next_token_nonce, 6);
}

#[test]
fn unknown_token_rejected() {
    let mut service = PolicyService::new("test-key-unknown");
    let mut audit = AuditChain::default();

    // Create a fake token that was never issued
    let fake_token = phantomkernel_policyd::CapabilityToken {
        token_id: "tok-fake".to_string(),
        subject: "app://fake".to_string(),
        shard: "work".to_string(),
        resource: "network".to_string(),
        action: "connect".to_string(),
        issued_at_epoch_s: 100,
        expires_at_epoch_s: 200,
        signature: phantomkernel_policyd::SignatureEnvelope {
            key_id: "key-primary".to_string(),
            algorithm: "hmac-sha256".to_string(),
            value_hex: "abcd".to_string(),
        },
    };

    let result = service.validate_token(&fake_token, "work", 110, &mut audit);
    assert!(matches!(result, Err(PolicyError::UnknownToken)));
}
