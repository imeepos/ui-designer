//! Global defaults stored at `~/Rudder/config.json` (ARCHITECTURE §6).
//!
//! Holds generation defaults (`quality`, `thinking`, `n`) and the last-used
//! project path. Secrets are NEVER stored here — credentials come from the
//! environment only (AGENTS.md hard rule).

use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Quality levels accepted by gpt-image-2 (docs/PRD.md §3.2).
pub const QUALITY_LEVELS: [&str; 3] = ["low", "medium", "high"];
/// Thinking effort levels (optional passthrough field; default `medium`).
pub const THINKING_LEVELS: [&str; 3] = ["low", "medium", "high"];

/// Generation defaults + last-used project pointer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Default `--quality` when a generate flag is omitted (default `low` —
    /// UI-REVIEW 缺陷 5：high must be opted into explicitly).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    /// Default `--thinking` (default `medium`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Default `--n` for page/component generate (board defaults to 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,
    /// Last successfully resolved project directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_project: Option<PathBuf>,
}

impl Config {
    /// `~/Rudder/config.json` path; `None` when no home directory is known.
    ///
    /// The directory can be overridden with the `RUDDER_HOME` environment
    /// variable (used by tests and portable setups); it never stores secrets.
    pub fn path() -> Option<PathBuf> {
        if let Ok(home) = std::env::var("RUDDER_HOME") {
            if !home.trim().is_empty() {
                return Some(PathBuf::from(home).join("config.json"));
            }
        }
        dirs::home_dir().map(|home| home.join("Rudder").join("config.json"))
    }

    /// Load the config; a missing file yields the default config.
    pub fn load() -> Config {
        Self::load_from(Self::path().as_deref())
    }

    fn load_from(path: Option<&Path>) -> Config {
        let Some(path) = path else { return Config::default() };
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    /// Persist atomically (tempfile + rename).
    pub fn save(&self) -> Result<()> {
        Self::save_to(Self::path().as_deref(), self)
    }

    fn save_to(path: Option<&Path>, config: &Config) -> Result<()> {
        let Some(path) = path else {
            return Err(RudderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no home directory known; cannot locate ~/Rudder/config.json",
            )));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::store::atomic_write_json(path, config)
    }

    /// Effective default quality: config override, else `low` (探索档；
    /// `high` 需显式 `--quality high`，UI-REVIEW 缺陷 5 裁决).
    pub fn effective_quality(&self) -> &str {
        self.quality.as_deref().unwrap_or("low")
    }

    /// Effective default thinking: config override, else `medium`.
    pub fn effective_thinking(&self) -> &str {
        self.thinking.as_deref().unwrap_or("medium")
    }

    /// Effective default n for page/component generate (board uses 4).
    pub fn effective_n(&self) -> u8 {
        self.n.unwrap_or(1).clamp(1, 4)
    }

    /// Validate and set a key (`quality` | `thinking` | `n`).
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "quality" => {
                if !QUALITY_LEVELS.contains(&value) {
                    return Err(arg_err(format!(
                        "quality must be one of {}",
                        QUALITY_LEVELS.join("|")
                    )));
                }
                self.quality = Some(value.to_string());
            }
            "thinking" => {
                if !THINKING_LEVELS.contains(&value) {
                    return Err(arg_err(format!(
                        "thinking must be one of {}",
                        THINKING_LEVELS.join("|")
                    )));
                }
                self.thinking = Some(value.to_string());
            }
            "n" => {
                let n: u8 = value.parse().map_err(|_| {
                    arg_err("n must be an integer in 1..=4".to_string())
                })?;
                if !(1..=4).contains(&n) {
                    return Err(arg_err("n must be an integer in 1..=4".to_string()));
                }
                self.n = Some(n);
            }
            other => {
                return Err(arg_err(format!(
                    "unknown config key `{other}`; valid keys: quality, thinking, n"
                )))
            }
        }
        Ok(())
    }

    /// Read a key's effective value (for `config get`).
    pub fn get(&self, key: &str) -> Result<String> {
        match key {
            "quality" => Ok(self.effective_quality().to_string()),
            "thinking" => Ok(self.effective_thinking().to_string()),
            "n" => Ok(self.n.map(|v| v.to_string()).unwrap_or_else(|| {
                // Distinguish "unset" from board/page defaults at the CLI layer.
                "1".to_string()
            })),
            other => Err(RudderError::NotFound {
                what: format!("config key `{other}`"),
            }),
        }
    }
}

fn arg_err(detail: String) -> RudderError {
    RudderError::InvalidArg { detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_documented_values() {
        let cfg = Config::default();
        assert_eq!(cfg.effective_quality(), "low", "exploration by default (UI-REVIEW #5)");
        assert_eq!(cfg.effective_thinking(), "medium");
        assert_eq!(cfg.effective_n(), 1);
    }

    #[test]
    fn set_validates_enums() {
        let mut cfg = Config::default();
        cfg.set("quality", "low").unwrap();
        assert_eq!(cfg.quality.as_deref(), Some("low"));
        assert!(cfg.set("quality", "ultra").is_err());
        assert!(cfg.set("thinking", "medium").is_ok());
        assert!(cfg.set("thinking", "max").is_err());
        cfg.set("n", "4").unwrap();
        assert_eq!(cfg.n, Some(4));
        assert!(cfg.set("n", "5").is_err());
        assert!(cfg.set("n", "x").is_err());
        assert!(cfg.set("secret", "x").is_err(), "no secret keys accepted");
        assert_eq!(
            cfg.set("api_key", "x").unwrap_err().code(),
            "INVALID_ARG"
        );
    }

    #[test]
    fn roundtrip_through_disk() {
        let dir = std::env::temp_dir().join(format!("rudder-cfg-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("config.json");
        let mut cfg = Config::default();
        cfg.set("quality", "low").unwrap();
        cfg.last_project = Some(PathBuf::from("/tmp/proj"));
        Config::save_to(Some(&path), &cfg).unwrap();
        let loaded = Config::load_from(Some(&path));
        assert_eq!(loaded, cfg);
        assert!(Config::load_from(Some(&dir.join("missing.json"))).quality.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
