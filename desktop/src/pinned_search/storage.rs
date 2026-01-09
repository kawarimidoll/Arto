use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::types::PinnedSearch;

/// Storage for pinned searches (persisted to disk)
pub struct PinnedSearchStorage;

impl PinnedSearchStorage {
    /// Get the path to the pinned searches file
    ///
    /// e.g., ~/Library/Application Support/arto/pinned-searches.json on macOS
    fn file_path() -> Option<PathBuf> {
        dirs::config_local_dir().map(|p| p.join("arto").join("pinned-searches.json"))
    }

    /// Load all pinned searches from disk
    ///
    /// Returns an empty vector if the file doesn't exist or is invalid.
    pub fn load() -> Vec<PinnedSearch> {
        let path = match Self::file_path() {
            Some(p) => p,
            None => {
                tracing::debug!("Could not determine config directory for pinned searches");
                return Vec::new();
            }
        };

        if !path.exists() {
            tracing::debug!("Pinned searches file does not exist, returning empty list");
            return Vec::new();
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(?e, ?path, "Failed to read pinned searches file");
                return Vec::new();
            }
        };

        let file_data: PinnedSearchFile = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(?e, ?path, "Failed to parse pinned searches file");
                return Vec::new();
            }
        };

        tracing::debug!(
            count = file_data.pinned_searches.len(),
            ?path,
            "Loaded pinned searches"
        );

        file_data.pinned_searches
    }

    /// Save all pinned searches to disk
    ///
    /// If the list is empty, the file is deleted to avoid clutter.
    pub fn save(pinned_searches: Vec<PinnedSearch>) -> Result<(), std::io::Error> {
        let path = Self::file_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory",
            )
        })?;

        // Create arto directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // If no pinned searches, delete the file
        if pinned_searches.is_empty() {
            if path.exists() {
                tracing::debug!(?path, "Deleting empty pinned searches file");
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }

        let file_data = PinnedSearchFile {
            version: 1,
            pinned_searches: pinned_searches.clone(),
        };

        let content = serde_json::to_string_pretty(&file_data)?;
        std::fs::write(&path, content)?;

        tracing::debug!(
            count = pinned_searches.len(),
            ?path,
            "Saved pinned searches"
        );

        Ok(())
    }
}

/// File format for pinned searches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinnedSearchFile {
    version: u32,
    pinned_searches: Vec<PinnedSearch>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pinned_search::types::HighlightColor;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        // Mock the file path
        std::env::set_var("HOME", temp_dir.path());

        // Create pinned searches with explicit IDs to avoid timestamp collision
        let pinned = vec![
            PinnedSearch {
                id: "ps_test_1".to_string(),
                pattern: "TODO".to_string(),
                color: HighlightColor::Orange,
                case_sensitive: false,
                disabled: false,
                created_at: chrono::Utc::now(),
            },
            PinnedSearch {
                id: "ps_test_2".to_string(),
                pattern: "FIXME".to_string(),
                color: HighlightColor::Pink,
                case_sensitive: false,
                disabled: false,
                created_at: chrono::Utc::now(),
            },
        ];

        // Save should create the file
        PinnedSearchStorage::save(pinned.clone()).unwrap();

        // Load should return the same data
        let loaded = PinnedSearchStorage::load();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].pattern, "TODO");
        assert_eq!(loaded[1].pattern, "FIXME");
    }

    #[test]
    fn test_empty_save_deletes_file() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path());

        // Create a file first
        let pinned = vec![PinnedSearch::new("TODO", HighlightColor::Orange, false)];
        PinnedSearchStorage::save(pinned).unwrap();

        // Save empty list should delete the file
        PinnedSearchStorage::save(Vec::new()).unwrap();

        let loaded = PinnedSearchStorage::load();
        assert_eq!(loaded.len(), 0);
    }
}
