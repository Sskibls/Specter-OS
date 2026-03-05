use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Provisioning,
    Active,
    Degraded,
    Panic,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub token_id: String,
    pub subject: String,
    pub resource: String,
    pub expires_at_epoch_s: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_and_token_are_constructible() {
        let state = ServiceState::Active;
        let token = CapabilityToken {
            token_id: "tok-001".to_string(),
            subject: "app://demo".to_string(),
            resource: "network".to_string(),
            expires_at_epoch_s: 120,
        };

        assert_eq!(state, ServiceState::Active);
        assert_eq!(token.resource, "network");
    }
}
