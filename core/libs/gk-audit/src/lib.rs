use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedAuditEvent {
    pub sequence: u64,
    pub event_type: String,
    pub payload: String,
    pub previous_hash: String,
    pub event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditIntegrityError {
    SequenceMismatch { expected: u64, actual: u64 },
    PreviousHashMismatch { sequence: u64 },
    EventHashMismatch { sequence: u64 },
}

impl Display for AuditIntegrityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditIntegrityError::SequenceMismatch { expected, actual } => {
                write!(
                    formatter,
                    "audit sequence mismatch: expected {expected}, actual {actual}"
                )
            }
            AuditIntegrityError::PreviousHashMismatch { sequence } => {
                write!(
                    formatter,
                    "audit previous hash mismatch at sequence {sequence}"
                )
            }
            AuditIntegrityError::EventHashMismatch { sequence } => {
                write!(
                    formatter,
                    "audit event hash mismatch at sequence {sequence}"
                )
            }
        }
    }
}

impl Error for AuditIntegrityError {}

#[derive(Debug, Default)]
pub struct AuditChain {
    events: Vec<SignedAuditEvent>,
    last_hash: String,
}

impl AuditChain {
    pub fn append(&mut self, event_type: impl Into<String>, payload: impl Into<String>) -> u64 {
        let next_sequence = self.events.len() as u64 + 1;
        let event_type = event_type.into();
        let payload = payload.into();

        let previous_hash = self.last_hash.clone();
        let event_hash = compute_event_hash(next_sequence, &previous_hash, &event_type, &payload);
        let event = SignedAuditEvent {
            sequence: next_sequence,
            event_type,
            payload,
            previous_hash,
            event_hash: event_hash.clone(),
        };

        self.events.push(event);
        self.last_hash = event_hash;
        next_sequence
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn verify(&self) -> bool {
        self.verify_detailed().is_ok()
    }

    pub fn verify_detailed(&self) -> Result<(), AuditIntegrityError> {
        let mut expected_previous_hash = String::new();

        for (index, event) in self.events.iter().enumerate() {
            let expected_sequence = index as u64 + 1;
            if event.sequence != expected_sequence {
                return Err(AuditIntegrityError::SequenceMismatch {
                    expected: expected_sequence,
                    actual: event.sequence,
                });
            }

            if event.previous_hash != expected_previous_hash {
                return Err(AuditIntegrityError::PreviousHashMismatch {
                    sequence: event.sequence,
                });
            }

            let expected_hash = compute_event_hash(
                event.sequence,
                &event.previous_hash,
                &event.event_type,
                &event.payload,
            );
            if event.event_hash != expected_hash {
                return Err(AuditIntegrityError::EventHashMismatch {
                    sequence: event.sequence,
                });
            }

            expected_previous_hash = event.event_hash.clone();
        }

        Ok(())
    }

    pub fn recover(events: Vec<SignedAuditEvent>) -> Result<Self, AuditIntegrityError> {
        let mut chain = Self {
            events,
            last_hash: String::new(),
        };
        chain.verify_detailed()?;

        chain.last_hash = chain
            .events
            .last()
            .map(|event| event.event_hash.clone())
            .unwrap_or_default();

        Ok(chain)
    }

    pub fn events(&self) -> &[SignedAuditEvent] {
        &self.events
    }

    pub fn snapshot(&self) -> Vec<SignedAuditEvent> {
        self.events.clone()
    }
}

#[derive(Debug)]
pub enum AuditStoreError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Integrity(AuditIntegrityError),
    Corrupt(String),
}

impl Display for AuditStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditStoreError::Io(error) => write!(formatter, "I/O failure: {error}"),
            AuditStoreError::Serde(error) => write!(formatter, "serialization failure: {error}"),
            AuditStoreError::Integrity(error) => write!(formatter, "integrity failure: {error}"),
            AuditStoreError::Corrupt(message) => {
                write!(formatter, "corrupt audit store: {message}")
            }
        }
    }
}

impl Error for AuditStoreError {}

pub struct AuditStore {
    path: PathBuf,
}

impl AuditStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AuditStoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(AuditStoreError::Io)?;
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append_event(
        &self,
        event_type: impl Into<String>,
        payload: impl Into<String>,
    ) -> Result<SignedAuditEvent, AuditStoreError> {
        let mut chain = self.load_chain()?;
        let _ = chain.append(event_type, payload);

        let event = chain
            .events()
            .last()
            .cloned()
            .ok_or_else(|| AuditStoreError::Corrupt("appended event missing".to_string()))?;

        self.append_raw_event(&event)?;
        Ok(event)
    }

    pub fn load_chain(&self) -> Result<AuditChain, AuditStoreError> {
        let events = self.read_events()?;
        AuditChain::recover(events).map_err(AuditStoreError::Integrity)
    }

    pub fn replay_and_verify(&self) -> Result<Vec<SignedAuditEvent>, AuditStoreError> {
        self.load_chain().map(|chain| chain.snapshot())
    }

    pub fn recover_truncated_tail(&self) -> Result<usize, AuditStoreError> {
        if !self.path.exists() {
            return Ok(0);
        }

        let content = fs::read_to_string(&self.path).map_err(AuditStoreError::Io)?;
        let lines = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();

        let mut parsed_events = Vec::new();
        let mut valid_events = Vec::new();
        let mut invalid_found = false;

        for line in &lines {
            match serde_json::from_str::<SignedAuditEvent>(line) {
                Ok(event) => parsed_events.push(event),
                Err(_) => {
                    invalid_found = true;
                    break;
                }
            }
        }

        for event in parsed_events {
            let mut candidate = valid_events.clone();
            candidate.push(event);
            if AuditChain::recover(candidate.clone()).is_ok() {
                valid_events = candidate;
            } else {
                invalid_found = true;
                break;
            }
        }

        if !invalid_found && valid_events.len() == lines.len() {
            return Ok(0);
        }

        self.rewrite_events(&valid_events)?;
        Ok(lines.len().saturating_sub(valid_events.len()))
    }

    fn append_raw_event(&self, event: &SignedAuditEvent) -> Result<(), AuditStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(AuditStoreError::Io)?;
        }

        let encoded = serde_json::to_string(event).map_err(AuditStoreError::Serde)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(AuditStoreError::Io)?;
        writeln!(file, "{encoded}").map_err(AuditStoreError::Io)?;
        file.sync_data().map_err(AuditStoreError::Io)?;
        Ok(())
    }

    fn read_events(&self) -> Result<Vec<SignedAuditEvent>, AuditStoreError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.path).map_err(AuditStoreError::Io)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(AuditStoreError::Io)?;
            if line.trim().is_empty() {
                continue;
            }

            let event =
                serde_json::from_str::<SignedAuditEvent>(&line).map_err(AuditStoreError::Serde)?;
            events.push(event);
        }

        Ok(events)
    }

    fn rewrite_events(&self, events: &[SignedAuditEvent]) -> Result<(), AuditStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(AuditStoreError::Io)?;
        }

        let mut payload = String::new();
        for event in events {
            payload.push_str(&serde_json::to_string(event).map_err(AuditStoreError::Serde)?);
            payload.push('\n');
        }

        let tmp_path = tmp_path(&self.path);
        fs::write(&tmp_path, payload).map_err(AuditStoreError::Io)?;
        fs::rename(tmp_path, &self.path).map_err(AuditStoreError::Io)
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "audit".to_string());
    extension.push_str(".tmp");

    path.with_extension(extension)
}

fn compute_event_hash(
    sequence: u64,
    previous_hash: &str,
    event_type: &str,
    payload: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sequence.to_le_bytes());
    hasher.update(previous_hash.as_bytes());
    hasher.update(event_type.as_bytes());
    hasher.update(payload.as_bytes());
    let bytes = hasher.finalize();

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_verify_chain() {
        let mut chain = AuditChain::default();
        let first_seq = chain.append("policy.decision", "allow");
        let second_seq = chain.append("shard.start", "work");

        assert_eq!(first_seq, 1);
        assert_eq!(second_seq, 2);
        assert_eq!(chain.len(), 2);
        assert!(chain.verify());
    }

    #[test]
    fn tamper_detection_rejects_modified_events() {
        let mut chain = AuditChain::default();
        let _ = chain.append("policy.decision", "allow");
        let _ = chain.append("shard.start", "work");

        let mut events = chain.snapshot();
        events[1].payload = "tampered".to_string();

        let recovered = AuditChain::recover(events);
        assert!(matches!(
            recovered,
            Err(AuditIntegrityError::EventHashMismatch { .. })
        ));
    }

    #[test]
    fn recovery_succeeds_for_valid_snapshots() {
        let mut chain = AuditChain::default();
        let _ = chain.append("policy.decision", "allow");
        let _ = chain.append("shard.start", "work");

        let snapshot = chain.snapshot();
        let recovered = AuditChain::recover(snapshot).expect("recovery should succeed");
        assert_eq!(recovered.len(), 2);
        assert!(recovered.verify());
    }

    #[test]
    fn store_append_and_replay_verifies() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("audit.chain");
        let store = AuditStore::open(&path).expect("store should open");

        let first = store
            .append_event("policy.decision", "allow")
            .expect("first append should succeed");
        let second = store
            .append_event("shard.start", "work")
            .expect("second append should succeed");

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);

        let replay = store.replay_and_verify().expect("replay should verify");
        assert_eq!(replay.len(), 2);
    }

    #[test]
    fn store_detects_tampering() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("audit.chain");
        let store = AuditStore::open(&path).expect("store should open");

        let _ = store
            .append_event("policy.decision", "allow")
            .expect("append should succeed");
        let _ = store
            .append_event("shard.start", "work")
            .expect("append should succeed");

        let content = fs::read_to_string(&path).expect("audit file should read");
        let mut lines = content.lines().map(ToString::to_string).collect::<Vec<_>>();
        let mut second_event: SignedAuditEvent =
            serde_json::from_str(&lines[1]).expect("event should decode");
        second_event.payload = "tampered".to_string();
        lines[1] = serde_json::to_string(&second_event).expect("event should encode");

        let mut payload = String::new();
        for line in lines {
            payload.push_str(&line);
            payload.push('\n');
        }
        fs::write(&path, payload).expect("tampered content should write");

        let replay = store.replay_and_verify();
        assert!(matches!(replay, Err(AuditStoreError::Integrity(_))));
    }

    #[test]
    fn store_recovers_truncated_tail() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let path = temp.path().join("audit.chain");
        let store = AuditStore::open(&path).expect("store should open");

        let _ = store
            .append_event("policy.decision", "allow")
            .expect("append should succeed");
        let _ = store
            .append_event("shard.start", "work")
            .expect("append should succeed");

        let mut content = fs::read_to_string(&path).expect("audit file should read");
        content.push_str("{\"sequence\": invalid-json\n");
        fs::write(&path, content).expect("corrupt tail should write");

        let removed = store
            .recover_truncated_tail()
            .expect("recovery should succeed");
        assert_eq!(removed, 1);

        let replay = store.replay_and_verify().expect("replay should verify");
        assert_eq!(replay.len(), 2);
    }
}
