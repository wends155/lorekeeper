//! Project-local configuration for Lorekeeper.
//!
//! Configuration is loaded from `.lorekeeper/config.toml` at server startup.
//! If the file does not exist, it is created with default values.

use std::path::Path;
use tracing::warn;

/// Top-level Lorekeeper configuration.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct LoreConfig {
    /// Configuration for the `lorekeeper_reflect` tool.
    pub reflect: ReflectConfig,
    /// Configuration for the `lorekeeper_store` tool.
    pub store: StoreConfig,
}

/// Configuration for the memory-health reflection tool.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct ReflectConfig {
    /// Number of days without an update before an entry is considered stale.
    pub stale_days: u32,
    /// Number of days since creation without any access before an entry is considered dead.
    pub dead_entry_days: u32,
    /// Minimum access count to classify an entry as "hot".
    pub hot_access_threshold: u32,
}

/// Configuration for the store operation.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct StoreConfig {
    /// FTS5 BM25 similarity threshold for duplicate detection (0.0–1.0).
    pub similarity_threshold: f64,
}

impl Default for ReflectConfig {
    fn default() -> Self {
        Self { stale_days: 30, dead_entry_days: 7, hot_access_threshold: 5 }
    }
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self { similarity_threshold: 0.7 }
    }
}

impl LoreConfig {
    /// Returns the content for a freshly generated default `config.toml`.
    #[must_use]
    pub const fn default_toml_content() -> &'static str {
        r"# Lorekeeper Configuration
# Defaults are shown. Uncomment and modify to override.

[reflect]
# stale_days = 30
# dead_entry_days = 7
# hot_access_threshold = 5

[store]
# similarity_threshold = 0.7
"
    }

    /// Loads configuration from `<dir>/config.toml`.
    ///
    /// If the file does not exist, a default file is created and the compiled-in
    /// defaults are returned. Parse errors fall back to defaults with a warning.
    ///
    /// # Errors
    ///
    /// This function is infallible — errors produce a warning log and fall back
    /// to `LoreConfig::default()`.
    #[must_use]
    pub fn load(dir: &Path) -> Self {
        let config_path = dir.join("config.toml");

        if !config_path.exists() {
            if let Err(e) = std::fs::write(&config_path, Self::default_toml_content()) {
                warn!("Failed to write default config.toml: {e}");
            }
            return Self::default();
        }

        let content = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to read config.toml, using defaults: {e}");
                return Self::default();
            }
        };

        match toml::from_str::<Self>(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!("Failed to parse config.toml, using defaults: {e}");
                Self::default()
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn load_creates_default_when_missing() {
        let dir = tmp();
        let cfg = LoreConfig::load(dir.path());
        assert!(dir.path().join("config.toml").exists(), "config.toml should be created");
        assert_eq!(cfg.reflect.stale_days, 30);
        assert_eq!(cfg.reflect.dead_entry_days, 7);
        assert_eq!(cfg.reflect.hot_access_threshold, 5);
        assert!((cfg.store.similarity_threshold - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn load_reads_existing_config() {
        let dir = tmp();
        let content = "[reflect]\nstale_days = 14\n[store]\nsimilarity_threshold = 0.5\n";
        std::fs::write(dir.path().join("config.toml"), content).expect("write");
        let cfg = LoreConfig::load(dir.path());
        assert_eq!(cfg.reflect.stale_days, 14);
        assert!((cfg.store.similarity_threshold - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn load_falls_back_to_defaults_on_empty_file() {
        let dir = tmp();
        std::fs::write(dir.path().join("config.toml"), "").expect("write");
        let cfg = LoreConfig::load(dir.path());
        assert_eq!(cfg.reflect.stale_days, 30);
    }

    #[test]
    fn load_falls_back_on_invalid_toml() {
        let dir = tmp();
        std::fs::write(dir.path().join("config.toml"), "this is not toml!!!").expect("write");
        let cfg = LoreConfig::load(dir.path());
        assert_eq!(cfg.reflect.stale_days, 30);
    }

    #[test]
    fn default_toml_content_is_valid_toml() {
        let result = toml::from_str::<LoreConfig>(LoreConfig::default_toml_content());
        assert!(result.is_ok(), "Default TOML content should parse cleanly");
    }
}
