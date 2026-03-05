use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedEnvelope {
    schema_version: u32,
    payload: serde_json::Value,
}

#[derive(Debug)]
pub enum PersistenceError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    UnsupportedSchemaVersion {
        state_kind: &'static str,
        version: u32,
    },
    MigrationFailure {
        state_kind: &'static str,
        version: u32,
        reason: String,
    },
}

impl Display for PersistenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::Io(error) => write!(formatter, "I/O failure: {error}"),
            PersistenceError::Serde(error) => write!(formatter, "serialization failure: {error}"),
            PersistenceError::UnsupportedSchemaVersion {
                state_kind,
                version,
            } => {
                write!(
                    formatter,
                    "unsupported schema version for {state_kind}: {version}"
                )
            }
            PersistenceError::MigrationFailure {
                state_kind,
                version,
                reason,
            } => {
                write!(
                    formatter,
                    "failed migration for {state_kind} from v{version}: {reason}"
                )
            }
        }
    }
}

impl Error for PersistenceError {}

pub trait PersistedState: Serialize + DeserializeOwned + Sized {
    const STATE_KIND: &'static str;
    const CURRENT_SCHEMA_VERSION: u32;

    fn migrate_from(
        from_version: u32,
        raw_payload: serde_json::Value,
    ) -> Result<Self, PersistenceError> {
        let reason = format!("no migration path from schema version {from_version}");
        let _ = raw_payload;
        Err(PersistenceError::MigrationFailure {
            state_kind: Self::STATE_KIND,
            version: from_version,
            reason,
        })
    }
}

pub fn save_state<T: PersistedState>(path: &Path, state: &T) -> Result<(), PersistenceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(PersistenceError::Io)?;
    }

    let envelope = PersistedEnvelope {
        schema_version: T::CURRENT_SCHEMA_VERSION,
        payload: serde_json::to_value(state).map_err(PersistenceError::Serde)?,
    };

    let tmp_path = tmp_path(path);
    let serialized = serde_json::to_vec_pretty(&envelope).map_err(PersistenceError::Serde)?;

    {
        let mut file = fs::File::create(&tmp_path).map_err(PersistenceError::Io)?;
        file.write_all(&serialized).map_err(PersistenceError::Io)?;
        file.sync_all().map_err(PersistenceError::Io)?;
    }

    fs::rename(&tmp_path, path).map_err(PersistenceError::Io)?;
    Ok(())
}

pub fn load_state<T: PersistedState>(path: &Path) -> Result<Option<T>, PersistenceError> {
    recover_if_needed(path)?;

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path).map_err(PersistenceError::Io)?;
    let envelope: PersistedEnvelope =
        serde_json::from_str(&content).map_err(PersistenceError::Serde)?;

    if envelope.schema_version == T::CURRENT_SCHEMA_VERSION {
        let state = serde_json::from_value(envelope.payload).map_err(PersistenceError::Serde)?;
        return Ok(Some(state));
    }

    let migrated = T::migrate_from(envelope.schema_version, envelope.payload)?;
    Ok(Some(migrated))
}

pub fn recover_if_needed(path: &Path) -> Result<(), PersistenceError> {
    let tmp = tmp_path(path);
    match (path.exists(), tmp.exists()) {
        (false, true) => {
            fs::rename(tmp, path).map_err(PersistenceError::Io)?;
        }
        (true, true) => {
            fs::remove_file(tmp).map_err(PersistenceError::Io)?;
        }
        _ => {}
    }

    Ok(())
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "state".to_string());
    extension.push_str(".tmp");

    path.with_extension(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct ExampleState {
        value: String,
    }

    impl PersistedState for ExampleState {
        const STATE_KIND: &'static str = "example-state";
        const CURRENT_SCHEMA_VERSION: u32 = 1;
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct MigratingState {
        current_value: String,
    }

    impl PersistedState for MigratingState {
        const STATE_KIND: &'static str = "migrating-state";
        const CURRENT_SCHEMA_VERSION: u32 = 2;

        fn migrate_from(
            from_version: u32,
            raw_payload: serde_json::Value,
        ) -> Result<Self, PersistenceError> {
            if from_version == 1 {
                let old_value = raw_payload
                    .get("legacy_value")
                    .and_then(|value| value.as_str())
                    .ok_or(PersistenceError::MigrationFailure {
                        state_kind: Self::STATE_KIND,
                        version: from_version,
                        reason: "missing legacy_value".to_string(),
                    })?;

                return Ok(Self {
                    current_value: old_value.to_string(),
                });
            }

            Err(PersistenceError::UnsupportedSchemaVersion {
                state_kind: Self::STATE_KIND,
                version: from_version,
            })
        }
    }

    #[test]
    fn save_and_load_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir must be created");
        let path = temp.path().join("state.json");
        let state = ExampleState {
            value: "persisted".to_string(),
        };

        save_state(&path, &state).expect("state save must succeed");
        let loaded = load_state::<ExampleState>(&path).expect("state load must succeed");
        assert_eq!(loaded, Some(state));
    }

    #[test]
    fn recovers_from_crash_tmp_file() {
        let temp = tempfile::tempdir().expect("tempdir must be created");
        let path = temp.path().join("state.json");
        let tmp = path.with_extension("json.tmp");

        let envelope = PersistedEnvelope {
            schema_version: ExampleState::CURRENT_SCHEMA_VERSION,
            payload: serde_json::json!({"value": "recovered"}),
        };
        let payload =
            serde_json::to_string_pretty(&envelope).expect("envelope serialization must work");
        fs::write(&tmp, payload).expect("tmp state should be written");

        let loaded = load_state::<ExampleState>(&path).expect("state load must recover");
        assert_eq!(
            loaded,
            Some(ExampleState {
                value: "recovered".to_string()
            })
        );
        assert!(path.exists());
    }

    #[test]
    fn migration_hook_is_used_for_older_schema() {
        let temp = tempfile::tempdir().expect("tempdir must be created");
        let path = temp.path().join("state.json");

        let envelope = PersistedEnvelope {
            schema_version: 1,
            payload: serde_json::json!({"legacy_value": "legacy"}),
        };
        let payload =
            serde_json::to_string_pretty(&envelope).expect("envelope serialization must work");
        fs::write(&path, payload).expect("state should be written");

        let loaded = load_state::<MigratingState>(&path).expect("migration load must succeed");
        assert_eq!(
            loaded,
            Some(MigratingState {
                current_value: "legacy".to_string(),
            })
        );
    }
}
