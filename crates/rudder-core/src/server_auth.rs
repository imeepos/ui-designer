//! Self-hosted Rudder backend auth client (0-配置 mode).
//!
//! The desktop shell calls these once at login/registration time and then
//! persists the session token via
//! [`crate::config::credential::store_session_token`] (OS keychain only).
//! Afterwards the image client reuses the token as its bearer credential —
//! the backend exposes an OpenAI-compatible image proxy under
//! `{server}/v1/images/...`, so no user-side baseUrl/apiKey setup remains.
//!
//! - Endpoints (docs/ARCHITECTURE.md §4, backend `/api/v1` routes):
//!   `POST {server_base}/v1/auth/register` (`{username,password,email?}`),
//!   `POST {server_base}/v1/auth/login` (`{username,password}`),
//!   `GET {server_base}/v1/auth/me` (`Authorization: Bearer <token>`).
//!   `register`/`login` answer `{"token":"…","user":{…}}`; `me` answers
//!   `{"user":{…}}`.
//! - Signatures: `register`/`login` return the shared concrete [`AuthFuture`]
//!   (see its docs); `me` is a plain `async fn` returning `Result<AuthUser>`.
//! - Errors: non-2xx bodies `{"error":{"code","message","hint"?}}` map to
//!   [`RudderError::ApiError`] with `body_summary = "CODE: message"` (plus
//!   `" — hint"` when present), truncated to 200 chars. Network/timeout
//!   failures map to [`RudderError::ApiUnreachable`]; a 2xx body that does
//!   not parse maps to [`RudderError::BadResponse`]. Validation problems
//!   (username/password rules, taken names, closed registration) surface the
//!   backend's own message.
//! - Discipline: the token never reaches files, logs, or stdout;
//!   [`AuthSession`]'s `Debug` output is redacted like
//!   [`crate::config::credential::ApiKeyResolution`].

use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Default self-hosted backend base (`config.json` `server_url` and the
/// `RUDDER_SERVER_URL` env override win over this; see
/// [`crate::config::resolve_server_base_url`]).
pub const DEFAULT_SERVER_URL: &str = "https://veren.top/api";
/// Auth calls are tiny JSON round-trips — short timeout, no retries
/// (login/register are user-interactive and idempotent to re-submit).
pub const AUTH_TIMEOUT: Duration = Duration::from_secs(15);

/// Public profile of the authenticated user (backend `userDTO`); never a
/// secret. Field names match the backend JSON exactly (`createdAt` camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub status: String,
    pub credits: i64,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// A successful register/login: the bearer token plus the fresh profile.
/// `Debug` is hand-written and redacted — an accidental `{:?}` log line can
/// never leak the session token.
#[derive(Clone)]
pub struct AuthSession {
    pub token: String,
    pub user: AuthUser,
}

impl std::fmt::Debug for AuthSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthSession")
            .field("token", &"<redacted; set>")
            .field("user", &self.user)
            .finish()
    }
}

#[derive(Deserialize)]
struct SessionPayload {
    token: String,
    user: AuthUser,
}

#[derive(Deserialize)]
struct MePayload {
    user: AuthUser,
}

/// Awaitable returned by [`register`] / [`login`]: one **concrete** future
/// type for both calls, so callers can branch without awaiting each arm —
/// `let session = if register { register(…) } else { login(…) }.await;`
/// compiles, while two `async fn`s would produce distinct opaque futures
/// (E0308). The Tauri command layer relies on exactly this shape.
pub struct AuthFuture(Pin<Box<dyn Future<Output = Result<AuthSession>> + Send>>);

impl Future for AuthFuture {
    type Output = Result<AuthSession>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(cx)
    }
}

impl std::fmt::Debug for AuthFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AuthFuture(<redacted>)")
    }
}

/// Register a new account and return the immediate session.
pub fn register(
    server_base: &str,
    username: &str,
    password: &str,
    email: Option<&str>,
) -> AuthFuture {
    let mut body = json!({
        "username": username.trim(),
        "password": password,
    });
    if let Some(email) = email.map(str::trim).filter(|email| !email.is_empty()) {
        body["email"] = json!(email);
    }
    let server_base = server_base.to_string();
    AuthFuture(Box::pin(async move {
        post_session(&server_base, "/v1/auth/register", body).await
    }))
}

/// Log in with username/password and return the session (30-day JWT).
pub fn login(server_base: &str, username: &str, password: &str) -> AuthFuture {
    let body = json!({
        "username": username.trim(),
        "password": password,
    });
    let server_base = server_base.to_string();
    AuthFuture(Box::pin(async move {
        post_session(&server_base, "/v1/auth/login", body).await
    }))
}

/// Fetch the current profile for `token` (`GET /v1/auth/me`). Cheap and
/// side-effect free — suitable for session validation on app start.
pub async fn me(server_base: &str, token: &str) -> Result<AuthUser> {
    let base = normalize_base(server_base)?;
    let url = format!("{base}/v1/auth/me");
    let http = http_client()?;
    let response = http
        .get(&url)
        .bearer_auth(token.trim())
        .send()
        .await
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(RudderError::ApiError {
            status: status.as_u16(),
            body_summary: error_body_summary(&body),
        });
    }
    let payload: MePayload = serde_json::from_str(&body).map_err(|e| RudderError::BadResponse {
        detail: format!("`/v1/auth/me` body is not the expected shape: {e}"),
    })?;
    Ok(payload.user)
}

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(AUTH_TIMEOUT)
        .build()
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })
}

fn normalize_base(server_base: &str) -> Result<String> {
    let base = server_base.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(RudderError::InvalidArg { detail: "server base url is empty".into() });
    }
    Ok(base.to_string())
}

async fn post_session(server_base: &str, path: &str, body: Value) -> Result<AuthSession> {
    let base = normalize_base(server_base)?;
    let url = format!("{base}{path}");
    let http = http_client()?;
    let response = http
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(RudderError::ApiError {
            status: status.as_u16(),
            body_summary: error_body_summary(&body),
        });
    }
    let payload: SessionPayload =
        serde_json::from_str(&body).map_err(|e| RudderError::BadResponse {
            detail: format!("`{path}` body is not the expected shape: {e}"),
        })?;
    Ok(AuthSession { token: payload.token, user: payload.user })
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    #[serde(default)]
    error: Option<ErrorInner>,
}

#[derive(Deserialize)]
struct ErrorInner {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    hint: Option<String>,
}

/// `"CODE: message"` (+" — hint"), whitespace-flattened, truncated to 200
/// chars. Non-JSON bodies fall back to the raw truncated text.
fn error_body_summary(body: &str) -> String {
    let summary = match serde_json::from_str::<ErrorEnvelope>(body).ok().and_then(|e| e.error) {
        Some(inner) => {
            let code = inner.code.unwrap_or_default();
            let message = inner.message.unwrap_or_default();
            let mut text = match (code.is_empty(), message.is_empty()) {
                (true, true) => String::new(),
                (true, false) => message,
                (false, true) => code,
                (false, false) => format!("{code}: {message}"),
            };
            if let Some(hint) = inner.hint.map(|h| h.trim().to_string()).filter(|h| !h.is_empty()) {
                if !text.is_empty() {
                    text.push_str(" — ");
                }
                text.push_str(&hint);
            }
            text
        }
        None => body.to_string(),
    };
    let flat: String = summary.chars().map(|c| if c.is_whitespace() { ' ' } else { c }).collect();
    let mut truncated: String = flat.chars().take(200).collect();
    if flat.chars().count() > 200 {
        truncated.push('…');
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockServer;

    // Fake-but-realistic fixtures; no real credentials in tests.
    const FAKE_TOKEN: &str = "test-jwt-header.fake-payload-4242.sig";
    const USER_JSON: &str = r#"{
        "id": "u-1",
        "username": "rudder-fan",
        "email": null,
        "role": "user",
        "status": "active",
        "credits": 100,
        "createdAt": "2026-09-10T08:00:00Z"
    }"#;

    fn session_body() -> Vec<u8> {
        format!(r#"{{"token":"{FAKE_TOKEN}","user":{USER_JSON}}}"#).into_bytes()
    }

    #[tokio::test]
    async fn login_success_parses_token_and_user() {
        let server = MockServer::start(move |_req, _i| (200, session_body()));
        let session = login(&server.url(), "rudder-fan", "secret-pass-1").await.unwrap();
        assert_eq!(session.token, FAKE_TOKEN);
        assert_eq!(session.user.id, "u-1");
        assert_eq!(session.user.username, "rudder-fan");
        assert_eq!(session.user.email, None, "null email parses as None");
        assert_eq!(session.user.role, "user");
        assert_eq!(session.user.status, "active");
        assert_eq!(session.user.credits, 100);
        assert_eq!(session.user.created_at, "2026-09-10T08:00:00Z");

        let req = &server.recorded()[0];
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/v1/auth/login");
        assert!(req.header("authorization").is_none(), "login needs no bearer");
        let sent: Value = serde_json::from_str(&req.body_str()).unwrap();
        assert_eq!(sent["username"], "rudder-fan");
        assert_eq!(sent["password"], "secret-pass-1");
    }

    #[tokio::test]
    async fn register_with_email_posts_email_field() {
        let server = MockServer::start(move |_req, _i| (200, session_body()));
        let session = register(&server.url(), "rudder-fan", "secret-pass-1", Some("fan@example.com"))
            .await
            .unwrap();
        assert_eq!(session.token, FAKE_TOKEN);

        let req = &server.recorded()[0];
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/v1/auth/register");
        let sent: Value = serde_json::from_str(&req.body_str()).unwrap();
        assert_eq!(sent["username"], "rudder-fan");
        assert_eq!(sent["email"], "fan@example.com");

        // Empty/blank email is omitted rather than sent as an empty string.
        let server2 = MockServer::start(move |_req, _i| (200, session_body()));
        register(&server2.url(), "rudder-fan", "secret-pass-1", Some("   "))
            .await
            .unwrap();
        let sent: Value = serde_json::from_str(&server2.recorded()[0].body_str()).unwrap();
        assert!(sent.get("email").is_none(), "blank email must be omitted");
    }

    #[tokio::test]
    async fn login_failure_surfaces_backend_message() {
        let body = r#"{"error":{"code":"INVALID_CREDENTIALS","message":"用户名或密码错误","hint":""}}"#;
        let server = MockServer::start(move |_req, _i| (401, body.as_bytes().to_vec()));
        let err = login(&server.url(), "rudder-fan", "wrong-pass").await.unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 401);
                assert!(body_summary.contains("INVALID_CREDENTIALS: 用户名或密码错误"), "{body_summary}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
        assert_eq!(err.code(), "API_ERROR");
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn registration_closed_error_carries_hint() {
        let body =
            r#"{"error":{"code":"REGISTRATION_CLOSED","message":"注册已关闭","hint":"请联系管理员开通账号"}}"#;
        let server = MockServer::start(move |_req, _i| (403, body.as_bytes().to_vec()));
        let err = register(&server.url(), "rudder-fan", "secret-pass-1", None)
            .await
            .unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 403);
                assert!(
                    body_summary.contains("REGISTRATION_CLOSED: 注册已关闭 — 请联系管理员开通账号"),
                    "{body_summary}"
                );
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn me_sends_bearer_token_and_parses_user() {
        let server = MockServer::start(move |_req, _i| {
            (200, format!(r#"{{"user":{USER_JSON}}}"#).into_bytes())
        });
        // Trailing slashes in the base must be normalized away.
        let user = me(&format!("{}/", server.url()), FAKE_TOKEN).await.unwrap();
        assert_eq!(user.username, "rudder-fan");
        assert_eq!(user.credits, 100);

        let req = &server.recorded()[0];
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/v1/auth/me");
        let expected_auth = format!("Bearer {FAKE_TOKEN}");
        assert_eq!(req.header("authorization"), Some(expected_auth.as_str()));
    }

    #[tokio::test]
    async fn non_json_error_body_falls_back_to_raw_summary() {
        let server = MockServer::start(move |_req, _i| (500, b"internal boom".to_vec()));
        let err = login(&server.url(), "rudder-fan", "secret-pass-1").await.unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 500);
                assert_eq!(body_summary, "internal boom");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn malformed_success_body_maps_bad_response() {
        let server = MockServer::start(move |_req, _i| (200, b"not json".to_vec()));
        let err = login(&server.url(), "rudder-fan", "secret-pass-1").await.unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
    }

    #[tokio::test]
    async fn unreachable_server_maps_api_unreachable() {
        let err = login("http://127.0.0.1:1", "rudder-fan", "secret-pass-1")
            .await
            .unwrap_err();
        assert_eq!(err.code(), "API_UNREACHABLE");
        let err = me("http://127.0.0.1:1", FAKE_TOKEN).await.unwrap_err();
        assert_eq!(err.code(), "API_UNREACHABLE");
    }

    #[test]
    fn blank_base_rejected_before_network() {
        let err = normalize_base("   ").unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
    }

    #[test]
    fn auth_session_debug_is_redacted() {
        let session = AuthSession {
            token: FAKE_TOKEN.to_string(),
            user: AuthUser {
                id: "u-1".into(),
                username: "rudder-fan".into(),
                email: None,
                role: "user".into(),
                status: "active".into(),
                credits: 100,
                created_at: "2026-09-10T08:00:00Z".into(),
            },
        };
        let rendered = format!("{session:?}");
        assert!(!rendered.contains(FAKE_TOKEN), "Debug leaked the token: {rendered}");
        assert!(rendered.contains("<redacted"));
        assert!(rendered.contains("rudder-fan"), "profile stays visible for support");
    }

    #[test]
    fn error_summary_is_flattened_and_truncated() {
        let long_message = "x".repeat(500);
        let body = format!(r#"{{"error":{{"code":"E","message":"{long_message}"}}}}"#);
        let summary = error_body_summary(&body);
        assert_eq!(summary.chars().count(), 201, "200 chars + ellipsis");
        assert!(summary.ends_with('…'));
    }
}
