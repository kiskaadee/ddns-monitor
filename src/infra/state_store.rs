use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::errors::StateError;
use crate::domain::models::MonitorState;
use crate::domain::traits::StateRepository;

/// Persists monitor state to a local JSON file using atomic rename writes.
pub struct JsonStateRepository {
    path: PathBuf,
}

impl JsonStateRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl StateRepository for JsonStateRepository {
    fn load(&self) -> Result<MonitorState, StateError> {
        if !self.path.exists() {
            log::debug!(
                "State file '{}' does not exist; returning default empty state",
                self.path.display()
            );
            return Ok(MonitorState::default());
        }

        let content = fs::read_to_string(&self.path).map_err(|source| StateError::Io {
            path: self.path.display().to_string(),
            source,
        })?;

        let state = serde_json::from_str(&content)?;
        Ok(state)
    }

    fn save(&self, state: &MonitorState) -> Result<(), StateError> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| StateError::Io {
            path: parent.display().to_string(),
            source,
        })?;

        let json_bytes = serde_json::to_vec_pretty(state)?;

        // Write to temporary file in the same directory, then atomically rename
        let pid = std::process::id();
        let tmp_path = self.path.with_extension(format!("tmp.{}", pid));

        let mut file = File::create(&tmp_path).map_err(|source| StateError::Io {
            path: tmp_path.display().to_string(),
            source,
        })?;

        file.write_all(&json_bytes)
            .map_err(|source| StateError::Io {
                path: tmp_path.display().to_string(),
                source,
            })?;

        file.sync_all().map_err(|source| StateError::Io {
            path: tmp_path.display().to_string(),
            source,
        })?;

        fs::rename(&tmp_path, &self.path).map_err(|source| StateError::Io {
            path: self.path.display().to_string(),
            source,
        })?;

        log::debug!("Atomically saved state to '{}'", self.path.display());
        Ok(())
    }
}
