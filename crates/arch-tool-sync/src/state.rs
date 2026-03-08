use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Tracks the last-synced state of managed files.
/// Stored at `.arch-tool/sync-state.yaml` in the downstream repo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncState {
    /// Map from relative file path to its last-synced SHA-256 hash.
    pub files: HashMap<String, FileState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// SHA-256 of the file content when it was last synced.
    pub hash: String,
    /// ISO 8601 timestamp of last sync.
    pub synced_at: String,
}

const STATE_DIR: &str = ".arch-tool";
const STATE_FILE: &str = ".arch-tool/sync-state.yaml";

impl SyncState {
    /// Load existing sync state, or return empty state if not found.
    pub fn load(repo_root: &Path) -> Self {
        let path = repo_root.join(STATE_FILE);
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_yaml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save sync state to disk.
    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let dir = repo_root.join(STATE_DIR);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating {}", dir.display()))?;

        let path = repo_root.join(STATE_FILE);
        let yaml = serde_yaml::to_string(self).context("serializing sync state")?;
        std::fs::write(&path, yaml)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Record a file as synced with the given hash.
    pub fn record(&mut self, rel_path: &str, hash: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        self.files.insert(
            rel_path.to_string(),
            FileState {
                hash: hash.to_string(),
                synced_at: now,
            },
        );
    }

    /// Get the last-synced hash for a file, if any.
    pub fn last_hash(&self, rel_path: &str) -> Option<&str> {
        self.files.get(rel_path).map(|s| s.hash.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_empty() {
        let dir = tempdir().unwrap();
        let state = SyncState::load(dir.path());
        assert!(state.files.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempdir().unwrap();
        let mut state = SyncState::default();
        state.record("AGENTS.md", "abc123");
        state.save(dir.path()).unwrap();

        let loaded = SyncState::load(dir.path());
        assert_eq!(loaded.last_hash("AGENTS.md"), Some("abc123"));
    }
}
