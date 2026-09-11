//! Global defaults stored at `~/Rudder/config.json` (ARCHITECTURE §6).
//!
//! Holds generation defaults (`quality`, `thinking`, `n`), the non-sensitive
//! `base_url` override, the self-hosted backend override (`server_url`), and
//! the last-used project path. Secrets are NEVER stored here — the API key
//! and the backend session token live in the OS keychain only
//! ([`credential`]); resolution priority is env → keychain (AGENTS.md v2).
//! 0-配置模式：桌面端登录后用会话令牌直连后端，无需自配 baseUrl/apiKey。

use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod credential;

pub use credential::{
    clear_api_key, clear_cms_api_key, clear_cms_session, load_cms_api_key, load_cms_session,
    resolve_api_key, resolve_image_api_key, set_api_key, store_cms_api_key, store_cms_session,
    test_connection, ApiKeyResolution, ConnectionTestReport, KeySource,
};

/// Every non-secret key accepted by [`Config::set`] / [`Config::get`]
/// ( surfaced in error messages; `api-key` is keychain-only).
pub const CONFIG_KEYS: [&str; 5] = ["quality", "thinking", "n", "base_url", "model"];

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
    /// Image model name override. Resolution: `OPENAI_MODEL` env → this
    /// field → `image::MODEL` default (`resolve_model`). Old config.json
    /// files without this field keep working via the serde default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Non-sensitive cms base override (cms account auth rides the session
    /// cookie, not `base_url`). Resolution: `RUDDER_SERVER_URL` env →
    /// this field → `cms_auth::DEFAULT_CMS_BASE_URL`
    /// (`resolve_server_base_url`). Empty/blank values never take effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
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
            "model" => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    // Reset: fall back to env/default resolution.
                    self.model = None;
                } else {
                    self.model = Some(trimmed.to_string());
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
                    "unknown config key `{other}`; valid keys: {}",
                    CONFIG_KEYS.join(", ")
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
            "model" => Ok(resolve_model()),
            other => Err(RudderError::NotFound {
                what: format!("config key `{other}`"),
            }),
        }
    }
}

/// Effective API base URL: `OPENAI_BASE_URL` env → `config.json` `base_url`
/// → default image endpoint (`image::DEFAULT_BASE_URL`, the self-hosted
/// Rudder backend; docs/ARCHITECTURE.md §4). Trailing slashes stripped.
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

/// Effective cms base URL: `RUDDER_SERVER_URL` env → `config.json`
/// `server_url` → `cms_auth::DEFAULT_CMS_BASE_URL`
/// (`https://veren.top/api`). Trailing slashes stripped; blank values at any
/// level fall through to the next one. Mirrors [`resolve_base_url`] so the
/// cms account client and `config get` can all agree.
pub fn resolve_server_base_url() -> String {
    if let Some(env_server) = credential::env_trimmed("RUDDER_SERVER_URL") {
        return env_server.trim_end_matches('/').to_string();
    }
    if let Some(config_server) = Config::load()
        .server_url
        .map(|base| base.trim().trim_end_matches('/').to_string())
        .filter(|base| !base.is_empty())
    {
        return config_server;
    }
    crate::cms_auth::DEFAULT_CMS_BASE_URL.to_string()
}

/// Effective image model name: `OPENAI_MODEL` env → `config.json` `model`
/// → `image::MODEL` default (`gpt-image-2`). Mirrors [`resolve_base_url`]
/// so the client, the lineage records and `config test` all agree.
pub fn resolve_model() -> String {
    if let Some(env_model) = credential::env_trimmed("OPENAI_MODEL") {
        return env_model;
    }
    if let Some(config_model) = Config::load()
        .model
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
    {
        return config_model;
    }
    crate::image::MODEL.to_string()
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

    /// Restores OPENAI_BASE_URL / OPENAI_MODEL / RUDDER_SERVER_URL /
    /// RUDDER_HOME on drop. All config env tests share one lock: they mutate
    /// the same `RUDDER_HOME` key and must not run in parallel.
    struct BaseUrlEnvGuard {
        saved: Vec<(String, Option<String>)>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    static CONFIG_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    impl BaseUrlEnvGuard {
        fn clear() -> BaseUrlEnvGuard {
            let lock = CONFIG_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut saved = Vec::new();
            for name in ["OPENAI_BASE_URL", "OPENAI_MODEL", "RUDDER_SERVER_URL", "RUDDER_HOME"] {
                saved.push((name.to_string(), std::env::var(name).ok()));
                std::env::remove_var(name);
            }
            BaseUrlEnvGuard { saved, _lock: lock }
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

        // Hermetic home: neither env nor config → official endpoint.
        // (Never read the developer's real ~/Rudder/config.json here.)
        let empty = std::env::temp_dir().join(format!("rudder-cfg-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&empty).unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &empty.display().to_string());
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

    #[test]
    fn model_roundtrips_and_resets_on_empty() {
        let mut cfg = Config::default();
        assert_eq!(cfg.model, None, "old configs without `model` stay valid");
        cfg.set("model", "  gpt-image-2  ").unwrap();
        assert_eq!(cfg.model.as_deref(), Some("gpt-image-2"), "value is trimmed");
        cfg.set("model", "   ").unwrap();
        assert_eq!(cfg.model, None, "empty resets to env/default resolution");
    }

    #[test]
    fn model_survives_disk_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rudder-cfg-model-{}", uuid::Uuid::new_v4()));
        let path = dir.join("config.json");
        let mut cfg = Config::default();
        cfg.set("model", "my-image-model").unwrap();
        Config::save_to(Some(&path), &cfg).unwrap();
        assert_eq!(Config::load_from(Some(&path)).model.as_deref(), Some("my-image-model"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_config_without_model_field_loads_with_default() {
        let dir = std::env::temp_dir().join(format!("rudder-cfg-legacy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{"quality":"low","thinking":"medium","base_url":"https://proxy.example.com"}"#,
        )
        .unwrap();
        let loaded = Config::load_from(Some(&path));
        assert_eq!(loaded.model, None, "missing `model` field must not fail parsing");
        assert_eq!(loaded.base_url.as_deref(), Some("https://proxy.example.com"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_model_env_then_config_then_default() {
        let _guard = BaseUrlEnvGuard::clear();

        // Hermetic home: neither env nor config → the documented default.
        let empty = std::env::temp_dir().join(format!("rudder-cfg-mempty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&empty).unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &empty.display().to_string());
        assert_eq!(resolve_model(), crate::image::MODEL);

        // config.json override applies when env is absent/empty.
        let dir = std::env::temp_dir().join(format!("rudder-cfg-mres-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), r#"{"model":"config-model-1"}"#).unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &dir.display().to_string());
        assert_eq!(resolve_model(), "config-model-1");

        // Env wins over config; a blank env falls back to config.
        BaseUrlEnvGuard::set("OPENAI_MODEL", "  env-model-9  ");
        assert_eq!(resolve_model(), "env-model-9", "env value is trimmed");
        BaseUrlEnvGuard::set("OPENAI_MODEL", "   ");
        assert_eq!(resolve_model(), "config-model-1");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_model_reports_effective_value() {
        let _guard = BaseUrlEnvGuard::clear();
        BaseUrlEnvGuard::set("OPENAI_MODEL", "env-model-9");
        let cfg = Config::default();
        assert_eq!(cfg.get("model").unwrap(), "env-model-9");
    }

    #[test]
    fn resolve_server_base_url_env_then_config_then_default() {
        let _guard = BaseUrlEnvGuard::clear();

        // Hermetic home: neither env nor config → the cms default base.
        // (Never read the developer's real ~/Rudder/config.json here.)
        let empty = std::env::temp_dir().join(format!("rudder-srv-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&empty).unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &empty.display().to_string());
        assert_eq!(resolve_server_base_url(), crate::cms_auth::DEFAULT_CMS_BASE_URL);

        // config.json override applies when env is absent/empty.
        let dir = std::env::temp_dir().join(format!("rudder-srv-res-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), r#"{"server_url":"https://server.example.com"}"#)
            .unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &dir.display().to_string());
        assert_eq!(resolve_server_base_url(), "https://server.example.com");

        // Env wins over config; trailing slashes are stripped.
        BaseUrlEnvGuard::set("RUDDER_SERVER_URL", "https://env-server.example.com///");
        assert_eq!(resolve_server_base_url(), "https://env-server.example.com");
        BaseUrlEnvGuard::set("RUDDER_SERVER_URL", "   ");
        assert_eq!(resolve_server_base_url(), "https://server.example.com");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn server_url_blank_config_value_never_takes_effect() {
        let _guard = BaseUrlEnvGuard::clear();
        let dir = std::env::temp_dir().join(format!("rudder-srv-blank-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        // Blank/whitespace server_url in config.json must fall through to the
        // default, and old configs without the field must keep parsing.
        std::fs::write(dir.join("config.json"), r#"{"server_url":"   ","quality":"low"}"#).unwrap();
        BaseUrlEnvGuard::set("RUDDER_HOME", &dir.display().to_string());
        let cfg = Config::load();
        assert_eq!(cfg.quality.as_deref(), Some("low"), "legacy fields still load");
        assert_eq!(cfg.server_url.as_deref(), Some("   "));
        assert_eq!(resolve_server_base_url(), crate::cms_auth::DEFAULT_CMS_BASE_URL);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn server_url_survives_disk_roundtrip() {
        let dir = std::env::temp_dir().join(format!("rudder-srt-rt-{}", uuid::Uuid::new_v4()));
        let path = dir.join("config.json");
        let cfg = Config { server_url: Some("https://server.example.com".into()), ..Config::default() };
        Config::save_to(Some(&path), &cfg).unwrap();
        let loaded = Config::load_from(Some(&path));
        assert_eq!(loaded.server_url.as_deref(), Some("https://server.example.com"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
