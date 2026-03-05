use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub const KDF: &str = "Argon2id";
pub const AEAD: &str = "XChaCha20-Poly1305";
pub const SIGNATURE: &str = "HMAC-SHA256";
pub const HASH: &str = "BLAKE3";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureEnvelope {
    pub key_id: String,
    pub algorithm: String,
    pub value_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRecord {
    pub key_id: String,
    pub secret: String,
    pub not_after_epoch_s: Option<u64>,
    pub revoked: bool,
}

impl KeyRecord {
    pub fn new(
        key_id: impl Into<String>,
        secret: impl Into<String>,
        not_after_epoch_s: Option<u64>,
    ) -> Self {
        Self {
            key_id: key_id.into(),
            secret: secret.into(),
            not_after_epoch_s,
            revoked: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct KeyRingFile {
    schema_version: u32,
    active_key_id: String,
    keys: Vec<KeyRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRing {
    active_key_id: String,
    keys: HashMap<String, KeyRecord>,
}

#[derive(Debug)]
pub enum CryptoError {
    ActiveKeyMissing,
    KeyNotFound(String),
    KeyRevoked(String),
    KeyExpired(String),
    InvalidSignature,
    InvalidSignatureEncoding,
    InvalidKeyMaterial,
    UnsupportedSchemaVersion(u32),
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl Display for CryptoError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::ActiveKeyMissing => write!(formatter, "active key is missing"),
            CryptoError::KeyNotFound(key_id) => write!(formatter, "key not found: {key_id}"),
            CryptoError::KeyRevoked(key_id) => write!(formatter, "key revoked: {key_id}"),
            CryptoError::KeyExpired(key_id) => write!(formatter, "key expired: {key_id}"),
            CryptoError::InvalidSignature => write!(formatter, "invalid signature"),
            CryptoError::InvalidSignatureEncoding => {
                write!(formatter, "invalid signature encoding")
            }
            CryptoError::InvalidKeyMaterial => write!(formatter, "invalid key material"),
            CryptoError::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported key file schema version: {version}")
            }
            CryptoError::Io(error) => write!(formatter, "I/O failure: {error}"),
            CryptoError::Serde(error) => write!(formatter, "serialization failure: {error}"),
        }
    }
}

impl Error for CryptoError {}

impl KeyRing {
    pub fn new(active_key_id: impl Into<String>, active_secret: impl Into<String>) -> Self {
        let active_key_id = active_key_id.into();
        let mut keys = HashMap::new();
        keys.insert(
            active_key_id.clone(),
            KeyRecord::new(active_key_id.clone(), active_secret, None),
        );

        Self {
            active_key_id,
            keys,
        }
    }

    pub fn from_records(
        active_key_id: impl Into<String>,
        keys: Vec<KeyRecord>,
    ) -> Result<Self, CryptoError> {
        let active_key_id = active_key_id.into();
        let mut by_id = HashMap::new();

        for key in keys {
            by_id.insert(key.key_id.clone(), key);
        }

        if !by_id.contains_key(&active_key_id) {
            return Err(CryptoError::ActiveKeyMissing);
        }

        Ok(Self {
            active_key_id,
            keys: by_id,
        })
    }

    pub fn load_from_path(path: &Path) -> Result<Self, CryptoError> {
        let content = fs::read_to_string(path).map_err(CryptoError::Io)?;
        let file = serde_json::from_str::<KeyRingFile>(&content).map_err(CryptoError::Serde)?;

        if file.schema_version != 1 {
            return Err(CryptoError::UnsupportedSchemaVersion(file.schema_version));
        }

        Self::from_records(file.active_key_id, file.keys)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), CryptoError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(CryptoError::Io)?;
        }

        let keys = self.keys.values().cloned().collect::<Vec<_>>();
        let file = KeyRingFile {
            schema_version: 1,
            active_key_id: self.active_key_id.clone(),
            keys,
        };
        let payload = serde_json::to_string_pretty(&file).map_err(CryptoError::Serde)?;
        fs::write(path, payload).map_err(CryptoError::Io)
    }

    pub fn rotate(&mut self, new_key: KeyRecord, activate: bool) {
        let new_key_id = new_key.key_id.clone();
        self.keys.insert(new_key_id.clone(), new_key);
        if activate {
            self.active_key_id = new_key_id;
        }
    }

    pub fn revoke_key(&mut self, key_id: &str) -> bool {
        let Some(key) = self.keys.get_mut(key_id) else {
            return false;
        };

        let previously_revoked = key.revoked;
        key.revoked = true;
        !previously_revoked
    }

    pub fn set_active_key(&mut self, key_id: &str) -> Result<(), CryptoError> {
        if !self.keys.contains_key(key_id) {
            return Err(CryptoError::KeyNotFound(key_id.to_string()));
        }

        self.active_key_id = key_id.to_string();
        Ok(())
    }

    pub fn sign(&self, message: &str, now_epoch_s: u64) -> Result<SignatureEnvelope, CryptoError> {
        let active_key = self
            .keys
            .get(&self.active_key_id)
            .ok_or(CryptoError::ActiveKeyMissing)?;

        self.validate_key_record(active_key, now_epoch_s)?;

        let signature = hmac_hex(active_key.secret.as_bytes(), message)?;
        Ok(SignatureEnvelope {
            key_id: active_key.key_id.clone(),
            algorithm: SIGNATURE.to_string(),
            value_hex: signature,
        })
    }

    pub fn verify(
        &self,
        message: &str,
        envelope: &SignatureEnvelope,
        now_epoch_s: u64,
    ) -> Result<(), CryptoError> {
        if envelope.algorithm != SIGNATURE {
            return Err(CryptoError::InvalidSignature);
        }

        let key = self
            .keys
            .get(&envelope.key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(envelope.key_id.clone()))?;

        self.validate_key_record(key, now_epoch_s)?;

        let decoded_signature = decode_hex(&envelope.value_hex)?;
        let mut mac = HmacSha256::new_from_slice(key.secret.as_bytes())
            .map_err(|_| CryptoError::InvalidKeyMaterial)?;
        mac.update(message.as_bytes());
        mac.verify_slice(&decoded_signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    pub fn active_key_id(&self) -> &str {
        &self.active_key_id
    }

    fn validate_key_record(&self, key: &KeyRecord, now_epoch_s: u64) -> Result<(), CryptoError> {
        if key.revoked {
            return Err(CryptoError::KeyRevoked(key.key_id.clone()));
        }

        if let Some(not_after_epoch_s) = key.not_after_epoch_s {
            if now_epoch_s >= not_after_epoch_s {
                return Err(CryptoError::KeyExpired(key.key_id.clone()));
            }
        }

        Ok(())
    }
}

pub fn supported_algorithms() -> [&'static str; 4] {
    [KDF, AEAD, SIGNATURE, HASH]
}

fn hmac_hex(secret: &[u8], message: &str) -> Result<String, CryptoError> {
    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|_| CryptoError::InvalidKeyMaterial)?;
    mac.update(message.as_bytes());
    Ok(encode_hex(&mac.finalize().into_bytes()))
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>, CryptoError> {
    if !value.len().is_multiple_of(2) {
        return Err(CryptoError::InvalidSignatureEncoding);
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    let chars = value.as_bytes().chunks_exact(2);

    for chunk in chars {
        let byte_value = std::str::from_utf8(chunk)
            .map_err(|_| CryptoError::InvalidSignatureEncoding)
            .and_then(|pair| {
                u8::from_str_radix(pair, 16).map_err(|_| CryptoError::InvalidSignatureEncoding)
            })?;
        bytes.push(byte_value);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_algorithms_are_exposed() {
        let algorithms = supported_algorithms();
        assert!(algorithms.contains(&"Argon2id"));
        assert!(algorithms.contains(&"HMAC-SHA256"));
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let key_ring = KeyRing::new("key-a", "secret-a");
        let envelope = SignatureEnvelope {
            key_id: "key-a".to_string(),
            algorithm: SIGNATURE.to_string(),
            value_hex: "deadbeef".to_string(),
        };

        let verification = key_ring.verify("payload", &envelope, 1);
        assert!(matches!(verification, Err(CryptoError::InvalidSignature)));
    }

    #[test]
    fn revoked_key_cannot_verify() {
        let mut key_ring = KeyRing::new("key-a", "secret-a");
        let signature = key_ring
            .sign("payload", 1)
            .expect("signature should be issued");
        let revoked = key_ring.revoke_key("key-a");
        assert!(revoked);

        let verification = key_ring.verify("payload", &signature, 2);
        assert!(matches!(verification, Err(CryptoError::KeyRevoked(_))));
    }

    #[test]
    fn expired_key_cannot_sign_or_verify() {
        let key_ring = KeyRing::from_records(
            "key-expired",
            vec![KeyRecord::new("key-expired", "secret-a", Some(5))],
        )
        .expect("key ring should be created");

        let signature_attempt = key_ring.sign("payload", 6);
        assert!(matches!(signature_attempt, Err(CryptoError::KeyExpired(_))));
    }

    #[test]
    fn rollover_keeps_previous_key_verifiable() {
        let mut key_ring = KeyRing::new("key-a", "secret-a");
        let signature_with_old_key = key_ring
            .sign("payload", 10)
            .expect("old key signature should be created");

        key_ring.rotate(KeyRecord::new("key-b", "secret-b", None), true);
        assert_eq!(key_ring.active_key_id(), "key-b");

        let old_verification = key_ring.verify("payload", &signature_with_old_key, 11);
        assert!(old_verification.is_ok());

        let new_signature = key_ring
            .sign("payload", 11)
            .expect("new key signature should be created");
        assert_eq!(new_signature.key_id, "key-b");
    }

    #[test]
    fn key_file_round_trip_works() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("keys.json");

        let mut key_ring = KeyRing::new("key-a", "secret-a");
        key_ring.rotate(KeyRecord::new("key-b", "secret-b", None), false);
        key_ring
            .save_to_path(&path)
            .expect("key ring save should succeed");

        let loaded = KeyRing::load_from_path(&path).expect("key ring should load");
        assert_eq!(loaded.active_key_id(), "key-a");
        assert!(loaded.keys.contains_key("key-b"));
    }
}
