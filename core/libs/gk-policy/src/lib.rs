use gk_types::CapabilityToken;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub subject: String,
    pub resource: String,
    pub duration_seconds: u64,
    pub grant: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow(CapabilityToken),
    Deny(String),
}

pub fn evaluate_request(request: &CapabilityRequest, now_epoch_s: u64) -> PolicyDecision {
    if !request.grant {
        return PolicyDecision::Deny("deny-by-default".to_string());
    }

    let token = CapabilityToken {
        token_id: "stub-token".to_string(),
        subject: request.subject.clone(),
        resource: request.resource.clone(),
        expires_at_epoch_s: now_epoch_s.saturating_add(request.duration_seconds),
    };

    PolicyDecision::Allow(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_denies_when_not_granted() {
        let request = CapabilityRequest {
            subject: "app://test".to_string(),
            resource: "network".to_string(),
            duration_seconds: 120,
            grant: false,
        };

        let decision = evaluate_request(&request, 1000);
        assert!(matches!(decision, PolicyDecision::Deny(_)));
    }

    #[test]
    fn allow_path_generates_expiring_token() {
        let request = CapabilityRequest {
            subject: "app://test".to_string(),
            resource: "microphone".to_string(),
            duration_seconds: 30,
            grant: true,
        };

        let decision = evaluate_request(&request, 500);
        match decision {
            PolicyDecision::Allow(token) => assert_eq!(token.expires_at_epoch_s, 530),
            PolicyDecision::Deny(_) => panic!("allow request should not deny"),
        }
    }
}
