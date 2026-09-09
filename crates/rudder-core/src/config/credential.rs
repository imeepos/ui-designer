//! Credential resolution + connectivity probe (docs/ARCHITECTURE.md §4,
//! AGENTS.md credential discipline v2).
//!
//! - API key priority: `OPENAI_API_KEY` env → OS keychain → none. Env wins so
//!   CI/proxy setups behave exactly as before; Dock-launched desktop apps
//!   fall back to the keychain.
//! - The keychain is the ONLY persistent secret store (service
//!   [`KEYCHAIN_SERVICE`], account [`KEYCHAIN_ACCOUNT`]); keys must never
//!   reach plain files, logs, or stdout. The one sanctioned display is the
//!   last 4 characters ([`tail4`]).
//! - Set `RUDDER_KEYCHAIN=0|off|false|no` to ignore the keychain entirely
//!   (hermetic tests, CI, shared machines).
//! - [`test_connection`] probes `GET {base}/v1/models` (free call, no image
//!   spend): HTTP 200 required and the model list must contain the effective
//!   image model (`config::resolve_model`).

use crate::error::{Result, RudderError};
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;

/// Keychain service name for stored Rudder credentials.
pub const KEYCHAIN_SERVICE: &str = "rudder";
/// Keychain account holding the gpt-image-2 API key.
pub const KEYCHAIN_ACCOUNT: &str = "openai-api-key";
/// The models probe is a cheap GET, not a generation — short timeout.
const TEST_TIMEOUT: Duration = Duration::from_secs(15);

// ---------------------------------------------------------------------------
// Resolution types
// ---------------------------------------------------------------------------

/// Where a resolved API key came from. Display-safe (never carries the key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// `OPENAI_API_KEY` environment variable (wins over the keychain).
    Env,
    /// OS keychain entry ([`KEYCHAIN_SERVICE`] / [`KEYCHAIN_ACCOUNT`]).
    Keychain,
    /// Nothing configured.
    None,
}

impl KeySource {
    /// Stable machine label (`env` | `keychain` | `none`) for CLI/desktop.
    pub fn as_str(&self) -> &'static str {
        match self {
            KeySource::Env => "env",
            KeySource::Keychain => "keychain",
            KeySource::None => "none",
        }
    }
}

/// A resolved API key plus its source. `Debug` is hand-written and redacted:
/// an accidental `{:?}` log line can never leak the secret.
pub struct ApiKeyResolution {
    pub key: Option<String>,
    pub source: KeySource,
}

impl std::fmt::Debug for ApiKeyResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyResolution")
            .field("source", &self.source.as_str())
            .field(
                "key",
                &if self.key.is_some() { "<redacted; set>" } else { "<none>" },
            )
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Secret store abstraction (real keychain + hermetic memory store)
// ---------------------------------------------------------------------------

/// Minimal keychain surface so tests and callers can run hermetically.
pub trait SecretStore: Send + Sync {
    fn get_password(&self) -> Result<Option<String>>;
    fn set_password(&self, key: &str) -> Result<()>;
    /// Delete must be idempotent: clearing an absent key is a no-op.
    fn delete_password(&self) -> Result<()>;
}

/// The real OS keychain: macOS Keychain / Windows Credential Manager /
/// Linux Secret Service (keyring crate, platform features per Cargo.toml).
pub struct KeyringStore;

impl KeyringStore {
    fn entry() -> Result<keyring::Entry> {
        keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|err| RudderError::KeychainAccess { detail: err.to_string() })
    }

    fn into_error(err: keyring::Error) -> RudderError {
        RudderError::KeychainAccess { detail: err.to_string() }
    }
}

impl SecretStore for KeyringStore {
    fn get_password(&self) -> Result<Option<String>> {
        match Self::entry()?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(Self::into_error(err)),
        }
    }

    fn set_password(&self, key: &str) -> Result<()> {
        Self::entry()?.set_password(key).map_err(Self::into_error)
    }

    fn delete_password(&self) -> Result<()> {
        match Self::entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(Self::into_error(err)),
        }
    }
}

/// In-memory [`SecretStore`] for tests and previews. `Debug` is redacted.
#[derive(Default)]
pub struct MemoryStore {
    secret: Mutex<Option<String>>,
}

impl MemoryStore {
    pub fn new() -> MemoryStore {
        MemoryStore::default()
    }

    pub fn with_key(key: &str) -> MemoryStore {
        let store = MemoryStore::new();
        *store.secret.lock().expect("memory store lock") = Some(key.to_string());
        store
    }
}

impl std::fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemoryStore(<redacted>)")
    }
}

impl SecretStore for MemoryStore {
    fn get_password(&self) -> Result<Option<String>> {
        Ok(self.secret.lock().expect("memory store lock").clone())
    }

    fn set_password(&self, key: &str) -> Result<()> {
        *self.secret.lock().expect("memory store lock") = Some(key.to_string());
        Ok(())
    }

    fn delete_password(&self) -> Result<()> {
        *self.secret.lock().expect("memory store lock") = None;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Resolution / storage API
// ---------------------------------------------------------------------------

/// True when `RUDDER_KEYCHAIN` opts out of the OS keychain entirely.
pub fn keychain_disabled() -> bool {
    matches!(std::env::var("RUDDER_KEYCHAIN"), Ok(value)
        if matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false" | "no"))
}

/// Resolve the API key: env → OS keychain → none (docs/ARCHITECTURE.md §4).
pub fn resolve_api_key() -> ApiKeyResolution {
    if keychain_disabled() {
        return env_only_resolution();
    }
    resolve_api_key_in(&KeyringStore)
}

/// [`resolve_api_key`] against an explicit store (hermetic tests).
pub fn resolve_api_key_in(store: &dyn SecretStore) -> ApiKeyResolution {
    if let Some(key) = non_empty_env("OPENAI_API_KEY") {
        return ApiKeyResolution { key: Some(key), source: KeySource::Env };
    }
    let keychain_key = store
        .get_password()
        .ok()
        .flatten()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());
    if let Some(key) = keychain_key {
        return ApiKeyResolution { key: Some(key), source: KeySource::Keychain };
    }
    ApiKeyResolution { key: None, source: KeySource::None }
}

fn env_only_resolution() -> ApiKeyResolution {
    match non_empty_env("OPENAI_API_KEY") {
        Some(key) => ApiKeyResolution { key: Some(key), source: KeySource::Env },
        None => ApiKeyResolution { key: None, source: KeySource::None },
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Trimmed, non-empty env lookup shared with base-URL resolution.
pub(crate) fn env_trimmed(name: &str) -> Option<String> {
    non_empty_env(name)
}

/// Store the API key in the OS keychain (the only sanctioned persistence).
pub fn set_api_key(key: &str) -> Result<()> {
    if keychain_disabled() {
        return Err(RudderError::InvalidArg {
            detail: "RUDDER_KEYCHAIN is disabled; export OPENAI_API_KEY instead".into(),
        });
    }
    set_api_key_in(&KeyringStore, key)
}

/// [`set_api_key`] against an explicit store (hermetic tests).
pub fn set_api_key_in(store: &dyn SecretStore, key: &str) -> Result<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(RudderError::InvalidArg {
            detail: "api key must not be empty".into(),
        });
    }
    store.set_password(key)
}

/// Remove the stored API key (idempotent; absent key is a no-op).
pub fn clear_api_key() -> Result<()> {
    if keychain_disabled() {
        return Ok(());
    }
    clear_api_key_in(&KeyringStore)
}

/// [`clear_api_key`] against an explicit store (hermetic tests).
pub fn clear_api_key_in(store: &dyn SecretStore) -> Result<()> {
    store.delete_password()
}

/// Last ≤4 characters of a key — the only sanctioned masked display.
pub fn tail4(key: &str) -> String {
    let chars: Vec<char> = key.trim().chars().collect();
    let start = chars.len().saturating_sub(4);
    chars[start..].iter().collect()
}

// ---------------------------------------------------------------------------
// Connectivity probe (free; no image spend)
// ---------------------------------------------------------------------------

/// Outcome of a successful [`test_connection`]. Never contains the key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionTestReport {
    /// Normalized base (trailing slashes stripped).
    pub base_url: String,
    /// HTTP status of the probe (always 2xx on Ok).
    pub http_status: u16,
    /// Number of models visible to this key.
    pub model_count: usize,
    /// Whether the requested model is among the visible models.
    pub has_image_model: bool,
    /// All visible model ids (settings dialog datalist; never a secret).
    pub models: Vec<String>,
}

/// Probe `GET {base}/v1/models` with `key`: free, validates reachability,
/// auth, and that the endpoint actually serves `model` (the effective
/// image model from [`crate::config::resolve_model`]).
pub async fn test_connection(
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<ConnectionTestReport> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(RudderError::InvalidArg { detail: "base url is empty".into() });
    }
    let key = api_key.trim();
    if key.is_empty() {
        return Err(RudderError::InvalidArg { detail: "api key is empty".into() });
    }
    let model = model.trim();
    if model.is_empty() {
        return Err(RudderError::InvalidArg { detail: "model is empty".into() });
    }
    let url = format!("{base}/v1/models");
    let http = reqwest::Client::builder()
        .timeout(TEST_TIMEOUT)
        .build()
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    let response = http
        .get(&url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(RudderError::ApiError {
            status: status.as_u16(),
            body_summary: probe_body_summary(&body),
        });
    }
    let value: Value = serde_json::from_str(&body).map_err(|e| RudderError::BadResponse {
        detail: format!("`/v1/models` body is not JSON: {e}"),
    })?;
    let models: Vec<String> = value
        .get("data")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|model| model.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if models.is_empty() {
        return Err(RudderError::BadResponse {
            detail: "`/v1/models` returned no `data[].id` entries".into(),
        });
    }
    let has_image_model = models.iter().any(|served| served == model);
    if !has_image_model {
        return Err(RudderError::BadResponse {
            detail: format!(
                "model `{model}` is not served at this endpoint ({} model(s) visible)",
                models.len()
            ),
        });
    }
    Ok(ConnectionTestReport {
        base_url: base.to_string(),
        http_status: status.as_u16(),
        model_count: models.len(),
        has_image_model: true,
        models,
    })
}

/// Truncated, newline-free body summary for probe errors (never echoes keys).
fn probe_body_summary(body: &str) -> String {
    let flat: String = body.chars().map(|c| if c.is_whitespace() { ' ' } else { c }).collect();
    let mut summary: String = flat.chars().take(200).collect();
    if flat.chars().count() > 200 {
        summary.push('…');
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // -- env hygiene --------------------------------------------------------

    /// Serializes env-mutating tests; restores prior values on drop.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        saved: Vec<(String, Option<String>)>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        /// Clears the credential env keys; callers set what they need after.
        fn clear() -> EnvGuard {
            let lock = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut saved = Vec::new();
            for name in ["OPENAI_API_KEY", "RUDDER_KEYCHAIN"] {
                saved.push((name.to_string(), std::env::var(name).ok()));
                std::env::remove_var(name);
            }
            EnvGuard { saved, _lock: lock }
        }

        fn set(name: &str, value: &str) {
            std::env::set_var(name, value);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (name, value) in self.saved.drain(..) {
                match value {
                    Some(v) => std::env::set_var(&name, v),
                    None => std::env::remove_var(&name),
                }
            }
        }
    }

    // -- helpers ------------------------------------------------------------

    /// One-shot mock `/v1/models` server; returns its base URL.
    fn spawn_models_server(status: u16, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let reason = if status == 200 { "OK" } else { "Unauthorized" };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{addr}")
    }

    // -- resolution chain ---------------------------------------------------

    #[test]
    fn env_key_wins_over_keychain() {
        let guard = EnvGuard::clear();
        EnvGuard::set("OPENAI_API_KEY", "env-key-0000");
        let store = MemoryStore::with_key("chain-key-9999");
        let resolved = resolve_api_key_in(&store);
        assert_eq!(resolved.source, KeySource::Env);
        assert_eq!(resolved.key.as_deref(), Some("env-key-0000"));
        drop(guard);
    }

    #[test]
    fn empty_env_falls_back_to_keychain() {
        let guard = EnvGuard::clear();
        EnvGuard::set("OPENAI_API_KEY", "   ");
        let store = MemoryStore::with_key("chain-key-9999");
        let resolved = resolve_api_key_in(&store);
        assert_eq!(resolved.source, KeySource::Keychain);
        assert_eq!(resolved.key.as_deref(), Some("chain-key-9999"));
        drop(guard);
    }

    #[test]
    fn missing_everywhere_reports_none() {
        let _guard = EnvGuard::clear();
        let resolved = resolve_api_key_in(&MemoryStore::new());
        assert_eq!(resolved.source, KeySource::None);
        assert!(resolved.key.is_none());
        assert_eq!(resolved.source.as_str(), "none");
    }

    #[test]
    fn keychain_values_are_trimmed() {
        let _guard = EnvGuard::clear();
        let store = MemoryStore::with_key("  padded-key-1  ");
        let resolved = resolve_api_key_in(&store);
        assert_eq!(resolved.key.as_deref(), Some("padded-key-1"));
    }

    #[test]
    fn kill_switch_skips_keychain_and_reports_none() {
        let guard = EnvGuard::clear();
        EnvGuard::set("RUDDER_KEYCHAIN", "0");
        // Public path with the kill switch on must NOT consult any store:
        // if it did, a developer-machine keychain entry would leak in here.
        let resolved = resolve_api_key();
        assert_eq!(resolved.source, KeySource::None);
        assert!(resolved.key.is_none());
        drop(guard);
    }

    #[test]
    fn kill_switch_keeps_env_priority() {
        let guard = EnvGuard::clear();
        EnvGuard::set("RUDDER_KEYCHAIN", "off");
        EnvGuard::set("OPENAI_API_KEY", "env-key-0000");
        let resolved = resolve_api_key();
        assert_eq!(resolved.source, KeySource::Env);
        drop(guard);
    }

    // -- store roundtrip ----------------------------------------------------

    #[test]
    fn set_get_clear_roundtrip_on_memory_store() {
        let store = MemoryStore::new();
        set_api_key_in(&store, " round-trip-key-9 ").unwrap();
        let resolved = resolve_api_key_in(&store);
        assert_eq!(resolved.source, KeySource::Keychain);
        assert_eq!(resolved.key.as_deref(), Some("round-trip-key-9"));

        clear_api_key_in(&store).unwrap();
        assert_eq!(resolve_api_key_in(&store).source, KeySource::None);
    }

    #[test]
    fn clear_is_idempotent_and_blank_keys_are_rejected() {
        let store = MemoryStore::new();
        clear_api_key_in(&store).unwrap(); // absent key: no-op, no error
        assert!(set_api_key_in(&store, "   ").is_err());
        let err = set_api_key_in(&store, "").unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
    }

    #[test]
    fn resolution_debug_is_redacted() {
        let resolved = ApiKeyResolution {
            key: Some("hidden-value-4321".into()),
            source: KeySource::Keychain,
        };
        let rendered = format!("{resolved:?}");
        assert!(!rendered.contains("hidden-value-4321"), "Debug leaked the key: {rendered}");
        assert!(rendered.contains("<redacted"));
    }

    #[test]
    fn tail4_slices_last_four_characters() {
        assert_eq!(tail4("abcdefgh"), "efgh");
        assert_eq!(tail4("abc"), "abc");
        assert_eq!(tail4("  xyz9  "), "xyz9");
        assert_eq!(tail4(""), "");
    }

    // -- connectivity probe (mock HTTP) --------------------------------------

    const MODELS_BODY: &str =
        r#"{"object":"list","data":[{"id":"gpt-image-2"},{"id":"gpt-4o"}]}"#;

    #[tokio::test]
    async fn probe_reports_model_count_on_200() {
        let base = spawn_models_server(200, MODELS_BODY);
        let report = test_connection(&base, "probe-key-1", crate::image::MODEL).await.unwrap();
        assert_eq!(report.http_status, 200);
        assert_eq!(report.model_count, 2);
        assert!(report.has_image_model);
        assert_eq!(report.base_url, base);
        assert_eq!(report.models, ["gpt-image-2", "gpt-4o"]);
    }

    #[tokio::test]
    async fn probe_checks_the_requested_model_not_a_hardcoded_one() {
        // Each probe gets its own server: the mock accepts one connection.
        let base = spawn_models_server(200, MODELS_BODY);
        // A custom configured model must be accepted when it is served…
        let report = test_connection(&base, "probe-key-1", "gpt-4o").await.unwrap();
        assert!(report.has_image_model);
        let base = spawn_models_server(200, MODELS_BODY);
        // …and rejected when it is not, whatever the default says.
        let err = test_connection(&base, "probe-key-1", "not-served-model").await.unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
        assert!(err.to_string().contains("not-served-model"));
    }

    #[tokio::test]
    async fn probe_normalizes_trailing_slashes() {
        let base = spawn_models_server(200, MODELS_BODY);
        let report = test_connection(&format!("{base}///"), "probe-key-1", crate::image::MODEL)
            .await
            .unwrap();
        assert_eq!(report.base_url, base);
    }

    #[tokio::test]
    async fn probe_maps_http_errors_with_body_summary() {
        let base = spawn_models_server(401, r#"{"error":"bad key"}"#);
        let err = test_connection(&base, "probe-key-1", crate::image::MODEL).await.unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 401);
                assert!(body_summary.contains("bad key"), "{body_summary}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
        assert_eq!(err.code(), "API_ERROR");
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn probe_requires_gpt_image_model() {
        let base = spawn_models_server(200, r#"{"data":[{"id":"gpt-4o"}]}"#);
        let err = test_connection(&base, "probe-key-1", crate::image::MODEL).await.unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
        assert!(err.to_string().contains(crate::image::MODEL));
    }

    #[tokio::test]
    async fn probe_flags_missing_model_list() {
        let base = spawn_models_server(200, r#"{"data":[]}"#);
        let err = test_connection(&base, "probe-key-1", crate::image::MODEL).await.unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
    }

    #[tokio::test]
    async fn probe_reports_unreachable_endpoints() {
        let err = test_connection("http://127.0.0.1:1", "probe-key-1", crate::image::MODEL)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "API_UNREACHABLE");
    }

    #[tokio::test]
    async fn probe_rejects_blank_inputs_before_network() {
        let err = test_connection("", "probe-key-1", crate::image::MODEL).await.unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
        let err = test_connection("http://127.0.0.1:1", "  ", crate::image::MODEL)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
        let err = test_connection("http://127.0.0.1:1", "probe-key-1", "  ").await.unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
    }
}
