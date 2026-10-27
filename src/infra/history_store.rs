use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::errors::HistoryError;
use crate::domain::models::HistoryEntry;
use crate::domain::traits::HistoryRepository;

/// Persists an append-only JSONL execution history log.
pub struct JsonlHistoryRepository {
    path: PathBuf,
}

impl JsonlHistoryRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl HistoryRepository for JsonlHistoryRepository {
    fn append(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| HistoryError::Io {
            path: parent.display().to_string(),
            source,
        })?;

        let mut line = serde_json::to_string(entry)?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| HistoryError::Io {
                path: self.path.display().to_string(),
                source,
            })?;

        file.write_all(line.as_bytes())
            .map_err(|source| HistoryError::Io {
                path: self.path.display().to_string(),
                source,
            })?;

        file.sync_data().map_err(|source| HistoryError::Io {
            path: self.path.display().to_string(),
            source,
        })?;

        log::debug!("Appended record to history log '{}'", self.path.display());
        Ok(())
    }
}
