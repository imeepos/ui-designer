//! gpt-image-2 client (ARCHITECTURE §4).
//!
//! - Endpoints: `{base}/v1/images/generations` (JSON) and
//!   `{base}/v1/images/edits` (multipart, `image[]` accepts several
//!   reference images — anchor first).
//! - Auth: `Authorization: Bearer <api key>`; key resolution is
//!   `OPENAI_API_KEY` env → OS keychain → none (`config::credential`), base
//!   resolution is `OPENAI_BASE_URL` env → `config.json` → default. Secrets
//!   are never stored in files, logged, or printed.
//! - Response: `b64_json`, decoded to raw bytes.
//! - Retry: 429/5xx **and malformed 2xx bodies** (e.g. missing `b64_json`)
//!   with exponential backoff, at most 3 retries; errors carry the HTTP
//!   status and a body summary.
//! - DryRun: `run_*` returns the full request plan (URL, params, multipart
//!   shape) and performs no network I/O.
//! - Timeout: 300s per attempt by default, enforced as a hard ceiling
//!   (`tokio::time::timeout`) covering DNS, connect, response and body —
//!   hangs surface as `API_UNREACHABLE` instead of stalling the CLI.

use crate::error::{Result, RudderError};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default image model served by the configured endpoint. The effective
/// model is resolved at client-build time via `config::resolve_model`
/// (`OPENAI_MODEL` env → `config.json` `model` → this constant).
pub const MODEL: &str = "gpt-image-2";
/// Per-request timeout (docs/ARCHITECTURE.md §4).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
/// Base delay for the exponential backoff (retries: ×1, ×2, ×4).
pub const DEFAULT_BACKOFF: Duration = Duration::from_secs(2);
/// Maximum number of retries after the initial attempt.
pub const MAX_RETRIES: u32 = 3;
/// Default endpoint base when `OPENAI_BASE_URL` is unset: the self-hosted
/// Rudder backend (`https://veren.top/api`). The client appends
/// `/v1/images/generations` / `/v1/images/edits`, landing on the backend's
/// OpenAI-compatible `/api/v1/images/...` proxy routes. `OPENAI_BASE_URL` /
/// `config.json` `base_url` still override this for BYO-key setups.
pub const DEFAULT_BASE_URL: &str = "https://veren.top/api";

// ---------------------------------------------------------------------------
// Parameters & plan types
// ---------------------------------------------------------------------------

/// Parameters for one image generation batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateParams {
    pub prompt: String,
    /// `"1536x1024"` form.
    pub size: String,
    pub quality: String,
    pub n: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Optional passthrough (endpoint-verified field; default `medium`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

/// One reference image for the edits endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRef {
    /// `"anchor"` or `"ref"` — ordering/role only, never a credential.
    pub role: String,
    pub path: PathBuf,
}

/// One image entry inside a `RequestPlan`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanImage {
    pub role: String,
    pub path: String,
    /// Size in bytes when the file exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// True when the file does not exist yet (planning ahead of a pick).
    #[serde(default)]
    pub missing: bool,
    /// Sniffed MIME type (png/jpeg/octet-stream).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

/// The full description of a would-be HTTP request. Dry-run output; also
/// stored with real runs so the manifest can cite the exact endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestPlan {
    /// `generations` | `edits`.
    pub endpoint: String,
    pub method: String,
    pub url: String,
    /// Whether an Authorization header would be sent (never its value).
    pub auth_header_present: bool,
    /// JSON body (generations) or the multipart text fields (edits).
    pub params: Value,
    /// Multipart `image[]` parts (edits only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<PlanImage>,
}

/// Outcome of a `run_*` call: the plan (always) plus decoded image bytes
/// (`None` in dry-run mode).
#[derive(Debug, Clone)]
pub struct RunOutput {
    pub plan: RequestPlan,
    /// `None` in dry-run; otherwise one raw image per `data[i].b64_json`.
    pub images: Option<Vec<Vec<u8>>>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// gpt-image-2 HTTP client with dry-run support and bounded retries.
pub struct ImageClient {
    base_url: String,
    /// Effective image model (see [`crate::config::resolve_model`]).
    model: String,
    api_key: Option<String>,
    http: reqwest::Client,
    dry_run: bool,
    backoff: Duration,
    /// Hard per-attempt ceiling applied via `tokio::time::timeout` on top of
    /// reqwest's own timeout, so DNS/connect hangs can never exceed budget.
    timeout: Duration,
}

impl ImageClient {
    /// Build a client pointed at `base_url` with explicit settings
    /// (used by tests against a mock server). Uses the default [`MODEL`];
    /// [`ImageClient::from_config`] swaps in the resolved model name.
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        dry_run: bool,
        backoff: Duration,
        timeout: Duration,
    ) -> Result<ImageClient> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
        Ok(ImageClient {
            base_url: normalize_base(base_url.into()),
            model: MODEL.to_string(),
            api_key,
            http,
            dry_run,
            backoff,
            timeout,
        })
    }

    /// Resolve credentials/base/model through the standard chain
    /// (env → keychain / config → defaults, see [`crate::config`]). Never
    /// fails in dry-run: a missing key only makes `auth_header_present=false`
    /// in the plan.
    pub fn from_config(dry_run: bool) -> Result<ImageClient> {
        let base = crate::config::resolve_base_url();
        let resolution = crate::config::credential::resolve_api_key();
        let mut client = Self::new(base, resolution.key, dry_run, DEFAULT_BACKOFF, DEFAULT_TIMEOUT)?;
        client.model = crate::config::resolve_model();
        Ok(client)
    }

    /// The effective image model name (for lineage records and probes).
    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn generations_url(&self) -> String {
        format!("{}/v1/images/generations", self.base_url)
    }

    fn edits_url(&self) -> String {
        format!("{}/v1/images/edits", self.base_url)
    }

    fn params_json(&self, params: &GenerateParams) -> Value {
        let mut v = json!({
            "model": self.model,
            "prompt": params.prompt,
            "size": params.size,
            "quality": params.quality,
            "n": params.n,
        });
        if let Some(seed) = params.seed {
            v["seed"] = json!(seed);
        }
        if let Some(thinking) = &params.thinking {
            v["thinking"] = json!(thinking);
        }
        v
    }

    /// Build (without sending) the generations request plan.
    pub fn plan_generations(&self, params: &GenerateParams) -> RequestPlan {
        RequestPlan {
            endpoint: "generations".into(),
            method: "POST".into(),
            url: self.generations_url(),
            auth_header_present: self.api_key.is_some(),
            params: self.params_json(params),
            images: Vec::new(),
        }
    }

    /// Build (without sending) the edits request plan. Reference images are
    /// stat'ed (never read into the plan) so dry-run stays cheap and honest.
    pub fn plan_edits(&self, params: &GenerateParams, images: &[ImageRef]) -> RequestPlan {
        let mut params_json = self.params_json(params);
        params_json["image[]"] = json!(images.iter().map(|img| img.path.display().to_string()).collect::<Vec<_>>());
        let plan_images = images
            .iter()
            .map(|img| {
                let meta = std::fs::metadata(&img.path).ok();
                let bytes = meta.as_ref().map(|m| m.len());
                PlanImage {
                    role: img.role.clone(),
                    path: img.path.display().to_string(),
                    bytes,
                    missing: meta.is_none(),
                    mime: bytes.and_then(|_| sniff_mime(&img.path)),
                }
            })
            .collect();
        RequestPlan {
            endpoint: "edits".into(),
            method: "POST".into(),
            url: self.edits_url(),
            auth_header_present: self.api_key.is_some(),
            params: params_json,
            images: plan_images,
        }
    }

    /// Generations: JSON body → `data[].b64_json`. Dry-run returns the plan
    /// only. Retries 429/5xx and malformed 2xx bodies with exponential
    /// backoff (≤ [`MAX_RETRIES`]).
    pub async fn run_generations(&self, params: &GenerateParams) -> Result<RunOutput> {
        let plan = self.plan_generations(params);
        if self.dry_run {
            return Ok(RunOutput { plan, images: None });
        }
        let key = self.require_key()?;
        let body = self.params_json(params);
        let url = self.generations_url();
        let send = |client: &ImageClient, key: &str| {
            client
                .http
                .post(&url)
                .bearer_auth(key)
                .json(&body)
        };
        let images = self.execute(send, key.as_str()).await?;
        Ok(RunOutput { plan, images: Some(images) })
    }

    /// Edits: multipart with one or more `image[]` parts (anchor first,
    /// then extra refs). Dry-run returns the plan only.
    pub async fn run_edits(&self, params: &GenerateParams, images: &[ImageRef]) -> Result<RunOutput> {
        let plan = self.plan_edits(params, images);
        if self.dry_run {
            return Ok(RunOutput { plan, images: None });
        }
        if images.is_empty() {
            return Err(RudderError::InvalidArg {
                detail: "edits calls need at least one reference image (the anchor)".into(),
            });
        }
        let key = self.require_key()?;

        // Read reference bytes up-front so a missing file fails before any
        // network I/O.
        let mut parts = Vec::with_capacity(images.len());
        for img in images {
            let bytes = std::fs::read(&img.path).map_err(|e| RudderError::InvalidArg {
                detail: format!("cannot read reference image {}: {e}", img.path.display()),
            })?;
            let mime = sniff_mime(&img.path).unwrap_or_else(|| "application/octet-stream".into());
            let file_name = img
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "reference.png".into());
            parts.push((img.role.clone(), bytes, mime, file_name));
        }

        let url = self.edits_url();
        let text_fields = self.params_json(params);
        let send = |client: &ImageClient, key: &str| {
            let mut form = reqwest::multipart::Form::new();
            for (field, value) in text_fields.as_object().expect("params_json is an object") {
                if field == "image[]" {
                    continue;
                }
                let value = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                form = form.text(field.clone(), value);
            }
            for (role, bytes, mime, file_name) in &parts {
                form = form.part(
                    "image[]",
                    reqwest::multipart::Part::bytes(bytes.clone())
                        .file_name(format!("{role}-{file_name}"))
                        .mime_str(mime)
                        .expect("mime is a valid header value"),
                );
            }
            client
                .http
                .post(&url)
                .bearer_auth(key)
                .multipart(form)
        };
        let decoded = self.execute(send, key.as_str()).await?;
        Ok(RunOutput { plan, images: Some(decoded) })
    }

    fn require_key(&self) -> Result<String> {
        self.api_key.clone().ok_or(RudderError::CredentialMissing)
    }

    /// Shared send/retry/decode loop. `send` rebuilds the request per attempt
    /// (bodies can be consumed by a failed send).
    async fn execute<F>(&self, send: F, key: &str) -> Result<Vec<Vec<u8>>>
    where
        F: Fn(&ImageClient, &str) -> reqwest::RequestBuilder,
    {
        let mut attempt = 0u32;
        loop {
            // Hard ceiling over the entire attempt — reqwest's own timeout
            // does not reliably cover resolver/connect stalls, and tonight's
            // incident (a proxy accepting the connection then never replying)
            // left the CLI hanging far past budget. Fail fast instead.
            let response = tokio::time::timeout(self.timeout, send(self, key).send())
                .await
                .map_err(|_| RudderError::ApiUnreachable {
                    detail: format!(
                        "no response within {:?} (hard ceiling covers DNS/connect/body)",
                        self.timeout
                    ),
                })?
                .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
            let status = response.status();
            if status.is_success() {
                let bytes = tokio::time::timeout(self.timeout, response.bytes())
                    .await
                    .map_err(|_| RudderError::ApiUnreachable {
                        detail: format!("response body exceeded {:?}", self.timeout),
                    })?
                    .map_err(|e| RudderError::BadResponse { detail: e.to_string() })?;
                match decode_b64_images(&bytes) {
                    Ok(images) => return Ok(images),
                    // A 2xx with an unusable body (not JSON, no `data` array,
                    // missing `b64_json`) is treated as a transient glitch —
                    // some proxies occasionally degrade the payload — and
                    // retried under the same backoff budget as 429/5xx.
                    Err(e) if e.code() == "BAD_RESPONSE" && attempt < MAX_RETRIES => {
                        let delay = self.backoff * (1u32 << attempt);
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            let body_summary = body_summary(response).await;
            let retryable = status.as_u16() == 429 || status.is_server_error();
            if retryable && attempt < MAX_RETRIES {
                let delay = self.backoff * (1u32 << attempt);
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }
            if status.as_u16() == 429 {
                return Err(RudderError::RateLimited { body_summary });
            }
            return Err(RudderError::ApiError {
                status: status.as_u16(),
                body_summary,
            });
        }
    }
}

fn normalize_base(mut base: String) -> String {
    while base.ends_with('/') {
        base.pop();
    }
    if base.is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        base
    }
}

/// Truncated, newline-free body summary for error messages.
async fn body_summary(response: reqwest::Response) -> String {
    let text = response.text().await.unwrap_or_default();
    let flat: String = text.chars().map(|c| if c.is_whitespace() { ' ' } else { c }).collect();
    let mut summary: String = flat.chars().take(200).collect();
    if flat.chars().count() > 200 {
        summary.push('…');
    }
    summary
}

/// Parse `{ "data": [ { "b64_json": "…" }, … ] }` into raw image bytes.
pub fn decode_b64_images(body: &[u8]) -> Result<Vec<Vec<u8>>> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|e| RudderError::BadResponse { detail: format!("body is not JSON: {e}") })?;
    let Some(data) = value.get("data").and_then(Value::as_array) else {
        return Err(RudderError::BadResponse {
            detail: format!("no `data` array in response: {}", summarize(value)),
        });
    };
    if data.is_empty() {
        return Err(RudderError::BadResponse {
            detail: "`data` array is empty".into(),
        });
    }
    let mut images = Vec::with_capacity(data.len());
    for (i, item) in data.iter().enumerate() {
        let b64 = item
            .get("b64_json")
            .and_then(Value::as_str)
            .ok_or_else(|| RudderError::BadResponse {
                detail: format!("data[{i}] has no b64_json (only b64 responses are supported)"),
            })?;
        // Tolerate data-URL prefixed payloads.
        let b64 = b64.strip_prefix("data:image/png;base64,").unwrap_or(b64);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| RudderError::BadResponse {
                detail: format!("data[{i}] b64_json is not valid base64: {e}"),
            })?;
        images.push(bytes);
    }
    Ok(images)
}

fn summarize(value: Value) -> String {
    let mut s = value.to_string();
    if s.len() > 200 {
        s.truncate(200);
        s.push('…');
    }
    s
}

/// Sniff a conservative MIME type from magic bytes.
pub fn sniff_mime(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut head = [0u8; 8];
    let n = file.read(&mut head).ok()?;
    let head = &head[..n];
    if head.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png".into())
    } else if head.starts_with(&[0xFF, 0xD8]) {
        Some("image/jpeg".into())
    } else if head.starts_with(b"GIF8") {
        Some("image/gif".into())
    } else if head.starts_with(b"RIFF") && head.len() >= 12 && &head[8..12] == b"WEBP" {
        Some("image/webp".into())
    } else {
        Some("application/octet-stream".into())
    }
}

#[cfg(test)]
mod tests;
