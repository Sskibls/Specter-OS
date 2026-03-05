use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcRequest {
    pub method: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcResponse {
    pub ok: bool,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub caller_id: String,
    pub roles: Vec<String>,
}

impl AuthContext {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|candidate| candidate == role)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    Unauthorized { required_role: String },
    UnknownMethod(String),
    InvalidPayload(String),
    Internal(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::Unauthorized { required_role } => {
                write!(formatter, "unauthorized: required role {required_role}")
            }
            IpcError::UnknownMethod(method) => write!(formatter, "unknown IPC method: {method}"),
            IpcError::InvalidPayload(message) => write!(formatter, "invalid payload: {message}"),
            IpcError::Internal(message) => write!(formatter, "internal IPC error: {message}"),
        }
    }
}

impl std::error::Error for IpcError {}

pub trait IpcTransport {
    fn call(&self, request: IpcRequest) -> Result<IpcResponse>;
}

pub trait IpcMethodHandler {
    fn handle(
        &mut self,
        auth_context: &AuthContext,
        request: IpcRequest,
    ) -> Result<IpcResponse, IpcError>;
}

pub fn require_role(auth_context: &AuthContext, role: &str) -> Result<(), IpcError> {
    if auth_context.has_role(role) {
        return Ok(());
    }

    Err(IpcError::Unauthorized {
        required_role: role.to_string(),
    })
}

pub fn decode_payload<T: DeserializeOwned>(request: &IpcRequest) -> Result<T, IpcError> {
    serde_json::from_str::<T>(&request.payload)
        .map_err(|error| IpcError::InvalidPayload(error.to_string()))
}

pub fn success_payload<T: Serialize>(payload: &T) -> Result<IpcResponse, IpcError> {
    let encoded =
        serde_json::to_string(payload).map_err(|error| IpcError::Internal(error.to_string()))?;
    Ok(IpcResponse {
        ok: true,
        payload: encoded,
    })
}

pub fn error_payload(message: impl Into<String>) -> IpcResponse {
    IpcResponse {
        ok: false,
        payload: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTransport;

    impl IpcTransport for EchoTransport {
        fn call(&self, request: IpcRequest) -> Result<IpcResponse> {
            Ok(IpcResponse {
                ok: true,
                payload: request.payload,
            })
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct ExamplePayload {
        message: String,
    }

    #[test]
    fn transport_trait_round_trip_works() {
        let transport = EchoTransport;
        let response = transport
            .call(IpcRequest {
                method: "Ping".to_string(),
                payload: "pong".to_string(),
            })
            .expect("call should return response");

        assert!(response.ok);
        assert_eq!(response.payload, "pong");
    }

    #[test]
    fn role_requirements_are_enforced() {
        let context = AuthContext {
            caller_id: "app://test".to_string(),
            roles: vec!["policy-client".to_string()],
        };

        assert!(require_role(&context, "policy-client").is_ok());
        let denied = require_role(&context, "policy-admin");
        assert!(matches!(denied, Err(IpcError::Unauthorized { .. })));
    }

    #[test]
    fn payload_helpers_encode_and_decode() {
        let payload = ExamplePayload {
            message: "hello".to_string(),
        };
        let response = success_payload(&payload).expect("response should encode");

        let request = IpcRequest {
            method: "Echo".to_string(),
            payload: response.payload,
        };
        let decoded = decode_payload::<ExamplePayload>(&request).expect("payload should decode");
        assert_eq!(decoded, payload);
    }
}
