//! Global defaults stored at `~/Rudder/config.json` (ARCHITECTURE §6).
//!
//! Holds generation defaults (`quality`, `thinking`, `n`), the non-sensitive
//! `base_url` override, and the last-used project path. Secrets are NEVER
//! stored here — the API key lives in the OS keychain only
//! ([`credential`]); resolution priority is env → keychain (AGENTS.md v2).

use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod credential;

pub use credential::{
    clear_api_key, resolve_api_key, set_api_key, test_connection, ApiKeyResolution,
    ConnectionTestReport, KeySource,
};

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
    /// Non-sensitive API base override. Resolution: `OPENAI_BASE_URL` env →
    /// this field → official endpoint (`credential::resolve_base_url`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
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
            "base_url" => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    // Reset: fall back to env/default resolution.
                    self.base_url = None;
                } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    self.base_url = Some(trimmed.trim_end_matches('/').to_string());
                } else {
                    return Err(arg_err(
                        "base_url must start with http:// or https:// (or be empty to reset)"
                            .to_string(),
                    ));
                }
            }
            other => {
                if other == "api-key" || other == "api_key" {
                    return Err(arg_err(
                        "secrets never go into config.json; use `rudder config set api-key` \
                         (value via stdin) to store in the OS keychain"
                            .to_string(),
                    ));
                }
                return Err(arg_err(format!(
                    "unknown config key `{other}`; valid keys: quality, thinking, n, base_url"
                )));
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
            "base_url" => Ok(resolve_base_url()),
            other => Err(RudderError::NotFound {
                what: format!("config key `{other}`"),
            }),
        }
    }
}

/// Effective API base URL: `OPENAI_BASE_URL` env → `config.json` `base_url`
/// → official endpoint (docs/ARCHITECTURE.md §4). Trailing slashes stripped.
pub fn resolve_base_url() -> String {
    if let Some(env_base) = credential::env_trimmed("OPENAI_BASE_URL") {
        return env_base.trim_end_matches('/').to_string();
    }
    if let Some(config_base) = Config::load()
        .base_url
        .map(|base| base.trim().trim_end_matches('/').to_string())
        .filter(|base| !base.is_empty())
    {
        return config_base;
    }
    crate::image::DEFAULT_BASE_URL.to_string()
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

    #[test]
    fn base_url_set_validates_scheme_and_resets_on_empty() {
        let mut cfg = Config::default();
        cfg.set("base_url", "https://proxy.example.com/").unwrap();
        assert_eq!(cfg.base_url.as_deref(), Some("https://proxy.example.com"));
        cfg.set("base_url", "  ").unwrap();
        assert_eq!(cfg.base_url, None, "empty resets to env/default resolution");
        assert!(cfg.set("base_url", "ftp://x").is_err());
        assert_eq!(cfg.set("base_url", "proxy.example.com").unwrap_err().code(), "INVALID_ARG");
        // Secrets stay out of config.json no matter the spelling.
        assert!(cfg.set("api-key", "x").is_err());
        assert!(cfg.set("api_key", "x").is_err());
        assert!(cfg.get("api-key").is_err());
    }

    #[test]
    fn base_url_survives_disk_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rudder-cfg-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("config.json");
        let mut cfg = Config::default();
        cfg.set("base_url", "https://proxy.example.com").unwrap();
        Config::save_to(Some(&path), &cfg).unwrap();
        assert_eq!(Config::load_from(Some(&path)).base_url.as_deref(), Some("https://proxy.example.com"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Restores OPENAI_BASE_URL / RUDDER_HOME on drop (credential tests own a
    /// different env lock, and these two keys never overlap with it).
    struct BaseUrlEnvGuard {
        saved: Vec<(String, Option<String>)>,
    }

    impl BaseUrlEnvGuard {
        fn clear() -> BaseUrlEnvGuard {
            let mut saved = Vec::new();
            for name in ["OPENAI_BASE_URL", "RUDDER_HOME"] {
                saved.push((name.to_string(), std::env::var(name).ok()));
                std::env::remove_var(name);
            }
            BaseUrlEnvGuard { saved }
        }

        fn set(name: &str, value: &str) {
            std::env::set_var(name, value);
        }
    }

    impl Drop for BaseUrlEnvGuard {
        fn drop(&mut self) {
            for (name, value) in self.saved.drain(..) {
                match value {
                    Some(v) => std::env::set_var(&name, v),
                    None => std::env::remove_var(&name),
                }
            }
        }
    }

    #[test]
    fn resolve_base_url_env_then_config_then_default() {
        let _guard = BaseUrlEnvGuard::clear();

        // Neither env nor config → official endpoint.
        assert_eq!(resolve_base_url(), crate::image::DEFAULT_BASE_URL);

        // config.json override applies when env is absent/empty.
        let dir = std::env::temp_dir().join(format!("rudder-cfg-res-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), r#"{"base_url":"https://proxy.example.com"}"#)
            .unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &dir.display().to_string());
        assert_eq!(resolve_base_url(), "https://proxy.example.com");

        // Env wins over config; trailing slashes are stripped.
        BaseUrlEnvGuard::set("OPENAI_BASE_URL", "https://env.example.com///");
        assert_eq!(resolve_base_url(), "https://env.example.com");
        BaseUrlEnvGuard::set("OPENAI_BASE_URL", "   ");
        assert_eq!(resolve_base_url(), "https://proxy.example.com");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_base_url_reports_effective_value() {
        let _guard = BaseUrlEnvGuard::clear();
        BaseUrlEnvGuard::set("OPENAI_BASE_URL", "https://env.example.com");
        let cfg = Config::default();
        assert_eq!(cfg.get("base_url").unwrap(), "https://env.example.com");
    }
}
