//! CMS account client (register / login / self-service API key / balance).
//!
//! The desktop shell talks to the existing cms service (base = the config
//! `server_url`, default [`DEFAULT_CMS_BASE_URL`]; cms routes live under
//! `/v1/...`). Contract highlights (cms `internal/platform/envelope.go` and
//! the domain `routes.go` files):
//!
//! - Every answer is the envelope `{code, message, data}`; **code = 0 is the
//!   only success signal**, the HTTP status carries transport semantics only.
//!   Non-zero codes map to [`RudderError::ApiError`] with a `"CODE: message"`
//!   summary (truncated to 200 chars, same as the legacy auth client);
//!   network/timeout failures map to [`RudderError::ApiUnreachable`]; a 2xx
//!   body that does not parse maps to [`RudderError::BadResponse`].
//! - Endpoints used here: `POST /v1/users/register`, `POST /v1/users/login`
//!   (answers `Set-Cookie: cms_session=…`; wrong password 1001, disabled
//!   account 1002, unauthenticated 1003, duplicate email 1000),
//!   `GET /v1/me`, `GET /v1/apikeys` (list, prefix only), `POST /v1/apikeys`
//!   (mint; the key plaintext appears exactly once), `POST
//!   /v1/apikeys/{id}/revoke` (idempotent) and `GET /v1/points/me`
//!   (`{balance, entries, total}`).
//! - Key management routes reject `Authorization: Bearer` outright (cms
//!   `RejectBearer`, key-minting anti-escalation) — every account call here
//!   rides the session cookie, never a bearer header.
//! - Discipline: the session cookie value and the minted key plaintext are
//!   persisted ONLY in the OS keychain (service `rudder`, accounts
//!   `cms-api-key` / `cms-session` via `config::credential`); they never
//!   reach files, logs, Debug output, or error text. [`login`] rotates a
//!   dedicated `rudder-desktop` key on every login (revoke-then-mint) so the
//!   keychain always holds exactly one valid plaintext.

use crate::config::credential::{
    keychain_disabled, load_cms_session_in, store_cms_api_key_in, store_cms_session_in,
    CMS_API_KEY_STORE, CMS_SESSION_STORE, SecretStore,
};
use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

/// Default cms base (config `server_url` default; cms routes hang `/v1/...`
/// off this base).
pub const DEFAULT_CMS_BASE_URL: &str = "https://veren.top/api";
/// Account calls are tiny JSON round-trips — short timeout, no retries
/// (register/login are user-interactive and safe to re-submit).
pub const CMS_TIMEOUT: Duration = Duration::from_secs(15);
/// Name of the self-service key minted (and rotated) for image generation.
pub const IMAGE_KEY_NAME: &str = "rudder-desktop";
/// Permission requested for the desktop key: cms gates the LLM protocol
/// gateway (`POST /v1/images/generations` included) behind `llm:invoke`
/// (cms `rbac.PermLLMInvoke` + `apikey.Middleware`; README §6 mints keys
/// with exactly this code).
pub const IMAGE_KEY_PERMISSION: &str = "llm:invoke";
/// cms session cookie name (cms `auth.CookieName`).
pub const SESSION_COOKIE_NAME: &str = "cms_session";
/// cms business code: unauthenticated / session invalid (cms
/// `user.CodeUnauthorized`, injected into the auth middleware).
pub const CODE_UNAUTHENTICATED: i64 = 1003;

const REGISTER_PATH: &str = "/v1/users/register";
const LOGIN_PATH: &str = "/v1/users/login";
const ME_PATH: &str = "/v1/me";
const LOGOUT_PATH: &str = "/v1/users/logout";
const APIKEYS_PATH: &str = "/v1/apikeys";
/// Page size 1: login/account status only need the balance.
const POINTS_ME_PATH: &str = "/v1/points/me?limit=1";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Public profile of a cms account (`userJSON` in cms `internal/user`).
/// Never a secret; timestamps are unix seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsUser {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub disabled: bool,
    pub created_at: i64,
}

/// One points ledger entry (`entryJSON` in cms `internal/points`). Part of
/// the cms contract surface; the account client itself only consumes the
/// balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsPointsEntry {
    pub id: i64,
    pub entry_type: String,
    pub ref_type: String,
    pub ref_id: i64,
    pub amount: i64,
    pub balance_after: i64,
    pub reason: String,
    pub created_at: i64,
}

/// Outcome of a fresh login: profile plus balance. Deliberately carries NO
/// key/cookie plaintext — those go straight into the OS keychain and are
/// never surfaced (a derived `Debug` can never leak them).
#[derive(Debug, Clone, Serialize)]
pub struct CmsSession {
    pub user: CmsUser,
    pub balance: i64,
}

/// Snapshot of the stored session (keychain cookie → `GET /v1/me` +
/// `/v1/points/me`). Same shape as [`CmsSession`] but read back from the
/// keychain rather than minted by a login.
#[derive(Debug, Clone, Serialize)]
pub struct CmsAccount {
    pub user: CmsUser,
    pub balance: i64,
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Register a new cms account and return the fresh profile (no session is
/// created; duplicate email is cms code 1000).
pub async fn register(base: &str, email: &str, password: &str, name: &str) -> Result<CmsUser> {
    let http = http_client()?;
    let body = json!({ "email": email.trim(), "password": password, "name": name.trim() });
    let data = post_envelope(&http, base, REGISTER_PATH, body, None).await?;
    parse_user(REGISTER_PATH, data)
}

/// Log in, rotate the dedicated `rudder-desktop` image key (revoke every
/// still-active key of that name, then mint a fresh one), read the balance,
/// and persist key + session cookie to the OS keychain. Returns the profile
/// and balance only — the plaintexts live in the keychain from here on.
pub async fn login(base: &str, email: &str, password: &str) -> Result<CmsSession> {
    if keychain_disabled() {
        return Err(RudderError::InvalidArg {
            detail: "RUDDER_KEYCHAIN is disabled; cms login has nowhere to persist credentials"
                .into(),
        });
    }
    login_in(base, email, password, &CMS_API_KEY_STORE, &CMS_SESSION_STORE).await
}

/// [`login`] against explicit stores (hermetic tests / previews).
pub async fn login_in(
    base: &str,
    email: &str,
    password: &str,
    key_store: &dyn SecretStore,
    session_store: &dyn SecretStore,
) -> Result<CmsSession> {
    let base = normalize_base(base)?;
    let http = http_client()?;
    let body = json!({ "email": email.trim(), "password": password });
    let response = http
        .post(format!("{base}{LOGIN_PATH}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    // Capture the session cookie before the body is consumed.
    let cookie = extract_session_cookie(&response);
    let data = read_envelope(response, LOGIN_PATH).await?;
    let payload: LoginData = serde_json::from_value(data).map_err(|e| RudderError::BadResponse {
        detail: format!("`{LOGIN_PATH}` data is not the expected session shape: {e}"),
    })?;
    let cookie = cookie.ok_or_else(|| RudderError::BadResponse {
        detail: format!(
            "`{LOGIN_PATH}` answered code=0 without a `{SESSION_COOKIE_NAME}` Set-Cookie"
        ),
    })?;

    // Rotate the dedicated image key: revoke every still-active key named
    // `rudder-desktop`, then mint a fresh one (key plaintext only ever
    // appears in the create response).
    let plain_key = rotate_image_key(&http, &base, &cookie).await?;
    let balance = fetch_balance(&http, &base, &cookie).await?;

    // Persist last: any failure above must leave the keychain untouched.
    store_cms_api_key_in(key_store, &plain_key)?;
    store_cms_session_in(session_store, &cookie)?;

    Ok(CmsSession { user: payload.user, balance })
}

/// Snapshot of the stored cms session: keychain cookie → `GET /v1/me` +
/// `GET /v1/points/me`. A missing cookie reports
/// [`RudderError::CredentialMissing`]; an invalid/expired cookie reports
/// cms code 1003 — use [`is_session_expired`] to detect it and prompt a
/// re-login.
pub async fn account_status(base: &str) -> Result<CmsAccount> {
    if keychain_disabled() {
        return Err(RudderError::InvalidArg {
            detail: "RUDDER_KEYCHAIN is disabled; the cms session is unavailable".into(),
        });
    }
    account_status_in(base, &CMS_SESSION_STORE).await
}

/// [`account_status`] against an explicit store (hermetic tests / previews).
pub async fn account_status_in(
    base: &str,
    session_store: &dyn SecretStore,
) -> Result<CmsAccount> {
    let cookie = load_cms_session_in(session_store)?.ok_or(RudderError::CredentialMissing)?;
    let http = http_client()?;
    let data = get_envelope(&http, base, ME_PATH, Some(&cookie)).await?;
    let user = parse_user(ME_PATH, data)?;
    let balance = fetch_balance(&http, base, &cookie).await?;
    Ok(CmsAccount { user, balance })
}

/// Best-effort server-side sign-out: `POST /v1/users/logout` with the
/// stored session cookie. Deliberately infallible in spirit — the call
/// answers `Ok(())` even when the server is unreachable, the envelope is an
/// error, or no cookie is stored, because the local keychain cleanup is what
/// actually ends the session; the server call only shortens the cookie's
/// server-side life.
pub async fn logout(base: &str) {
    if keychain_disabled() {
        return;
    }
    let Ok(Some(cookie)) = load_cms_session_in(&CMS_SESSION_STORE) else {
        return;
    };
    logout_in(base, &cookie).await;
}

/// [`logout`] with an explicit cookie (hermetic tests; no keychain read).
pub async fn logout_in(base: &str, cookie: &str) {
    let Ok(http) = http_client() else { return };
    let _ = post_envelope(&http, base, LOGOUT_PATH, json!({}), Some(cookie)).await;
}

/// True when `err` is the cms "session invalid / not logged in" failure
/// (envelope code [`CODE_UNAUTHENTICATED`], surfaced as `ApiError` with a
/// `"1003: …"` summary). Upper layers branch on this to guide re-login.
pub fn is_session_expired(err: &RudderError) -> bool {
    matches!(err, RudderError::ApiError { body_summary, .. } if body_summary.starts_with("1003:"))
}

// ---------------------------------------------------------------------------
// Envelope plumbing
// ---------------------------------------------------------------------------

/// cms business envelope (`platform.Envelope`): code 0 is the only success.
#[derive(Deserialize)]
struct CmsEnvelope {
    code: i64,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

#[derive(Deserialize)]
struct LoginData {
    user: CmsUser,
}

#[derive(Deserialize)]
struct PointsMeData {
    balance: i64,
}

/// One row of `GET /v1/apikeys` (prefix only — the plaintext never returns).
#[derive(Deserialize)]
struct CmsApiKeyItem {
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    revoked_at: Option<i64>,
}

/// `POST /v1/apikeys` create response — the one-time plaintext.
#[derive(Deserialize)]
struct CreatedKeyData {
    key: String,
}

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(CMS_TIMEOUT)
        .build()
        .map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })
}

fn normalize_base(base: &str) -> Result<String> {
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err(RudderError::InvalidArg { detail: "cms base url is empty".into() });
    }
    Ok(base.to_string())
}

/// `Cookie: cms_session=<value>` — the stored keychain value is used
/// verbatim as the cookie pair (cms `auth.CookieName`).
fn session_cookie_header(cookie: &str) -> String {
    format!("{SESSION_COOKIE_NAME}={cookie}")
}

/// Pull the `cms_session` cookie value out of a response's Set-Cookie
/// headers (ignoring unrelated cookies; value = raw token, base64url).
fn extract_session_cookie(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| {
            let pair = value.split(';').next()?.trim();
            let (name, cookie_value) = pair.split_once('=')?;
            if name.trim().eq_ignore_ascii_case(SESSION_COOKIE_NAME) {
                Some(cookie_value.trim().to_string())
            } else {
                None
            }
        })
}

/// Read one cms envelope: code=0 → `data`; code≠0 → `ApiError` with
/// `"CODE: message"` (transport status attached); unparseable 2xx →
/// `BadResponse`; unparseable non-2xx → `ApiError` with the raw summary.
async fn read_envelope(response: reqwest::Response, path: &str) -> Result<Value> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    match serde_json::from_str::<CmsEnvelope>(&body) {
        Ok(env) if env.code == 0 => Ok(env.data.unwrap_or(Value::Null)),
        Ok(env) => Err(RudderError::ApiError {
            status: status.as_u16(),
            body_summary: summarize_body(&format!("{}: {}", env.code, env.message)),
        }),
        Err(_) if status.is_success() => Err(RudderError::BadResponse {
            detail: format!("`{path}` body is not a cms envelope"),
        }),
        Err(_) => Err(RudderError::ApiError {
            status: status.as_u16(),
            body_summary: summarize_body(&body),
        }),
    }
}

async fn get_envelope(
    http: &reqwest::Client,
    base: &str,
    path: &str,
    cookie: Option<&str>,
) -> Result<Value> {
    let url = format!("{}{path}", normalize_base(base)?);
    let mut request = http.get(&url);
    if let Some(cookie) = cookie {
        request = request.header(reqwest::header::COOKIE, session_cookie_header(cookie));
    }
    let response =
        request.send().await.map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    read_envelope(response, path).await
}

async fn post_envelope(
    http: &reqwest::Client,
    base: &str,
    path: &str,
    body: Value,
    cookie: Option<&str>,
) -> Result<Value> {
    let url = format!("{}{path}", normalize_base(base)?);
    let mut request = http.post(&url).json(&body);
    if let Some(cookie) = cookie {
        request = request.header(reqwest::header::COOKIE, session_cookie_header(cookie));
    }
    let response =
        request.send().await.map_err(|e| RudderError::ApiUnreachable { detail: e.to_string() })?;
    read_envelope(response, path).await
}

fn parse_user(path: &str, data: Value) -> Result<CmsUser> {
    serde_json::from_value(data).map_err(|e| RudderError::BadResponse {
        detail: format!("`{path}` data is not the expected user shape: {e}"),
    })
}

/// Revoke every still-active key named [`IMAGE_KEY_NAME`], then mint a fresh
/// one carrying [`IMAGE_KEY_PERMISSION`]. Returns the one-time plaintext.
async fn rotate_image_key(http: &reqwest::Client, base: &str, cookie: &str) -> Result<String> {
    let data = get_envelope(http, base, APIKEYS_PATH, Some(cookie)).await?;
    let list: Vec<CmsApiKeyItem> =
        serde_json::from_value(data).map_err(|e| RudderError::BadResponse {
            detail: format!("`{APIKEYS_PATH}` data is not the expected key list: {e}"),
        })?;
    // Revoke is idempotent (cms apikey service), and already-revoked rows
    // are skipped — stale duplicates never block a fresh mint.
    for item in list.iter().filter(|k| k.name == IMAGE_KEY_NAME && k.revoked_at.is_none()) {
        let path = format!("/v1/apikeys/{}/revoke", item.id);
        post_envelope(http, base, &path, json!({}), Some(cookie)).await?;
    }
    let data = post_envelope(
        http,
        base,
        APIKEYS_PATH,
        json!({ "name": IMAGE_KEY_NAME, "permissions": [IMAGE_KEY_PERMISSION] }),
        Some(cookie),
    )
    .await?;
    let created: CreatedKeyData =
        serde_json::from_value(data).map_err(|e| RudderError::BadResponse {
            detail: format!("`{APIKEYS_PATH}` data carries no key plaintext: {e}"),
        })?;
    if created.key.trim().is_empty() {
        return Err(RudderError::BadResponse {
            detail: format!("`{APIKEYS_PATH}` returned an empty key plaintext"),
        });
    }
    Ok(created.key)
}

/// Balance from `GET /v1/points/me?limit=1`. Only `balance` is consumed —
/// ledger entry shape drift must never break a login.
async fn fetch_balance(http: &reqwest::Client, base: &str, cookie: &str) -> Result<i64> {
    let data = get_envelope(http, base, POINTS_ME_PATH, Some(cookie)).await?;
    let points: PointsMeData =
        serde_json::from_value(data).map_err(|e| RudderError::BadResponse {
            detail: format!("`/v1/points/me` data is not the expected shape: {e}"),
        })?;
    Ok(points.balance)
}

/// Whitespace-flattened, 200-char-truncated summary (legacy semantics).
fn summarize_body(text: &str) -> String {
    let flat: String = text.chars().map(|c| if c.is_whitespace() { ' ' } else { c }).collect();
    let mut truncated: String = flat.chars().take(200).collect();
    if flat.chars().count() > 200 {
        truncated.push('…');
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::credential::{load_cms_api_key_in, load_cms_session_in, MemoryStore};
    use crate::test_support::MockServer;
    use std::sync::Mutex;

    // Fake-but-realistic fixtures; no real credentials in tests.
    const KEY_PLAINTEXT: &str = "cms-minted-plain-key-4242";
    const COOKIE_PLAINTEXT: &str = "captured-cookie-value-9999";

    fn user_json() -> Value {
        json!({
            "id": 11,
            "email": "fan@example.com",
            "name": "舵手",
            "disabled": false,
            "created_at": 1_700_000_000i64
        })
    }

    fn env_ok(data: Value) -> Vec<u8> {
        json!({"code": 0, "message": "ok", "data": data}).to_string().into_bytes()
    }

    fn env_err(code: i64, message: &str) -> Vec<u8> {
        json!({"code": code, "message": message, "data": Value::Null}).to_string().into_bytes()
    }

    fn set_cookie_header() -> (String, String) {
        (
            "Set-Cookie".into(),
            format!(
                "{SESSION_COOKIE_NAME}={COOKIE_PLAINTEXT}; Path=/; \
                 Expires=Fri, 18 Sep 2026 04:00:00 GMT; HttpOnly; Secure; SameSite=Lax"
            ),
        )
    }

    fn login_ok_body() -> Vec<u8> {
        env_ok(json!({"user": user_json(), "expires_at": 1_700_086_400i64}))
    }

    fn created_key_body() -> Vec<u8> {
        env_ok(json!({
            "id": 30,
            "key": KEY_PLAINTEXT,
            "prefix": "cms_",
            "name": IMAGE_KEY_NAME,
            "created_at": 1_700_000_000i64,
            "expires_at": Value::Null,
            "permission_codes": [IMAGE_KEY_PERMISSION]
        }))
    }

    fn points_body(balance: i64) -> Vec<u8> {
        env_ok(json!({
            "balance": balance,
            "entries": [{
                "id": 900,
                "entry_type": "grant",
                "ref_type": "manual",
                "ref_id": 1,
                "amount": balance,
                "balance_after": balance,
                "reason": "初始赠送",
                "created_at": 1_700_000_000i64
            }],
            "total": 1
        }))
    }

    /// Fresh stores for a hermetic login.
    fn fresh_stores() -> (MemoryStore, MemoryStore) {
        (MemoryStore::new(), MemoryStore::new())
    }

    #[tokio::test]
    async fn register_success_parses_user_and_posts_fields() {
        let server = MockServer::start(move |_req, _i| (200, env_ok(user_json())));
        let user = register(&server.url(), " fan@example.com ", "pw-1", " 舵手 ")
            .await
            .expect("register succeeds");
        assert_eq!(user.id, 11);
        assert_eq!(user.email, "fan@example.com");
        assert_eq!(user.name, "舵手");
        assert!(!user.disabled);
        assert_eq!(user.created_at, 1_700_000_000);

        let req = &server.recorded()[0];
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, REGISTER_PATH);
        assert!(req.header("cookie").is_none(), "register needs no session");
        assert!(req.header("authorization").is_none(), "register needs no bearer");
        let sent: Value = serde_json::from_str(&req.body_str()).unwrap();
        assert_eq!(sent["email"], "fan@example.com", "email is trimmed");
        assert_eq!(sent["name"], "舵手", "name is trimmed");
        assert_eq!(sent["password"], "pw-1");
    }

    #[tokio::test]
    async fn register_duplicate_email_maps_envelope_1000() {
        let server = MockServer::start(move |_req, _i| (409, env_err(1000, "邮箱已被注册")));
        let err = register(&server.url(), "fan@example.com", "pw-1", "舵手")
            .await
            .unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 409);
                assert!(body_summary.starts_with("1000:"), "{body_summary}");
                assert!(body_summary.contains("邮箱已被注册"), "{body_summary}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
        assert_eq!(err.code(), "API_ERROR");
        assert_eq!(err.exit_code(), 2);
    }

    #[tokio::test]
    async fn non_zero_envelope_on_http_200_is_still_an_api_error() {
        // code=0 is the only success signal; the HTTP status is transport
        // noise, so a business failure on 200 must not be read as success.
        let server = MockServer::start(move |_req, _i| (200, env_err(1000, "邮箱已被注册")));
        let err = register(&server.url(), "fan@example.com", "pw-1", "舵手")
            .await
            .unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 200);
                assert!(body_summary.starts_with("1000:"), "{body_summary}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn login_success_captures_cookie_mints_key_reads_balance_and_stores() {
        let server = MockServer::start_with_headers(|req, _i| {
            match (req.method.as_str(), req.path.as_str()) {
                ("POST", LOGIN_PATH) => (200, vec![set_cookie_header()], login_ok_body()),
                ("GET", APIKEYS_PATH) => (200, vec![], env_ok(json!([]))),
                ("POST", APIKEYS_PATH) => (200, vec![], created_key_body()),
                ("GET", POINTS_ME_PATH) => (200, vec![], points_body(4200)),
                _ => (404, vec![], env_err(9999, "unexpected route")),
            }
        });
        let (key_store, session_store) = fresh_stores();
        let session = login_in(&server.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .expect("login flow succeeds");
        assert_eq!(session.user.id, 11);
        assert_eq!(session.user.email, "fan@example.com");
        assert_eq!(session.balance, 4200);

        // Keychain holds the minted plaintext key and the captured cookie.
        assert_eq!(load_cms_api_key_in(&key_store).unwrap().as_deref(), Some(KEY_PLAINTEXT));
        assert_eq!(load_cms_session_in(&session_store).unwrap().as_deref(), Some(COOKIE_PLAINTEXT));

        // Request sequence: login → list keys → create key → points.
        let recorded = server.recorded();
        let paths: Vec<&str> = recorded.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            [LOGIN_PATH, APIKEYS_PATH, APIKEYS_PATH, POINTS_ME_PATH],
            "login → list → create → points"
        );

        // Login posts credentials and nothing else.
        let sent: Value = serde_json::from_str(&recorded[0].body_str()).unwrap();
        assert_eq!(sent["email"], "fan@example.com");
        assert_eq!(sent["password"], "pw-1");
        assert!(recorded[0].header("cookie").is_none(), "login has no cookie yet");

        // Key-management calls ride the session cookie, never a bearer.
        for req in &recorded[1..] {
            assert_eq!(
                req.header("cookie"),
                Some(session_cookie_header(COOKIE_PLAINTEXT).as_str()),
                "cookie must ride {}",
                req.path
            );
            assert!(req.header("authorization").is_none(), "Bearer is rejected on cms account routes");
        }

        // The mint request carries the dedicated name + minimal permission.
        let mint: Value = serde_json::from_str(&recorded[2].body_str()).unwrap();
        assert_eq!(mint["name"], IMAGE_KEY_NAME);
        assert_eq!(mint["permissions"], json!([IMAGE_KEY_PERMISSION]));
    }

    #[tokio::test]
    async fn login_rotates_existing_named_key_before_minting() {
        let server = MockServer::start_with_headers(|req, _i| {
            match (req.method.as_str(), req.path.as_str()) {
                ("POST", LOGIN_PATH) => (200, vec![set_cookie_header()], login_ok_body()),
                ("GET", APIKEYS_PATH) => (
                    200,
                    vec![],
                    env_ok(json!([
                        {"id": 7, "prefix": "cms_a", "name": IMAGE_KEY_NAME, "created_at": 1,
                         "expires_at": Value::Null, "revoked_at": Value::Null,
                         "permission_codes": [IMAGE_KEY_PERMISSION]},
                        {"id": 8, "prefix": "cms_b", "name": "other-key", "created_at": 1,
                         "expires_at": Value::Null, "revoked_at": Value::Null,
                         "permission_codes": []},
                        {"id": 9, "prefix": "cms_c", "name": IMAGE_KEY_NAME, "created_at": 1,
                         "expires_at": Value::Null, "revoked_at": 123,
                         "permission_codes": []}
                    ])),
                ),
                ("POST", "/v1/apikeys/7/revoke") => (200, vec![], env_ok(Value::Null)),
                ("POST", APIKEYS_PATH) => (200, vec![], created_key_body()),
                ("GET", POINTS_ME_PATH) => (200, vec![], points_body(1)),
                _ => (404, vec![], env_err(9999, "unexpected route")),
            }
        });
        let (key_store, session_store) = fresh_stores();
        let session = login_in(&server.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .expect("login flow succeeds");
        assert_eq!(session.balance, 1);

        let recorded = server.recorded();
        let paths: Vec<&str> = recorded.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                LOGIN_PATH,
                APIKEYS_PATH,
                "/v1/apikeys/7/revoke",
                APIKEYS_PATH,
                POINTS_ME_PATH
            ],
            "active same-name key is revoked first; unrelated or already-revoked keys are left alone"
        );
    }

    #[tokio::test]
    async fn login_wrong_password_maps_1001_and_stores_nothing() {
        let server = MockServer::start(move |_req, _i| (401, env_err(1001, "邮箱或密码错误")));
        let (key_store, session_store) = fresh_stores();
        let err = login_in(&server.url(), "fan@example.com", "wrong", &key_store, &session_store)
            .await
            .unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 401);
                assert!(body_summary.starts_with("1001:"), "{body_summary}");
                assert!(body_summary.contains("邮箱或密码错误"), "{body_summary}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
        assert_eq!(
            server.recorded().len(),
            1,
            "a failed login must not trigger follow-up key/points calls"
        );
        assert_eq!(load_cms_api_key_in(&key_store).unwrap(), None, "nothing persisted");
        assert_eq!(load_cms_session_in(&session_store).unwrap(), None, "nothing persisted");
    }

    #[tokio::test]
    async fn login_disabled_account_maps_1002() {
        let server = MockServer::start(move |_req, _i| (403, env_err(1002, "账号已禁用，拒绝登录")));
        let (key_store, session_store) = fresh_stores();
        let err = login_in(&server.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .unwrap_err();
        match &err {
            RudderError::ApiError { status, body_summary } => {
                assert_eq!(*status, 403);
                assert!(body_summary.starts_with("1002:"), "{body_summary}");
                assert!(body_summary.contains("账号已禁用"), "{body_summary}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
        assert_eq!(load_cms_session_in(&session_store).unwrap(), None);
    }

    #[tokio::test]
    async fn login_without_session_cookie_maps_bad_response() {
        // code=0 but no cms_session Set-Cookie (only an unrelated cookie):
        // the flow must refuse rather than store a bogus session.
        let server = MockServer::start_with_headers(move |_req, _i| {
            (
                200,
                vec![("Set-Cookie".into(), "other=xyz; Path=/".into())],
                login_ok_body(),
            )
        });
        let (key_store, session_store) = fresh_stores();
        let err = login_in(&server.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
        assert_eq!(load_cms_api_key_in(&key_store).unwrap(), None);
        assert_eq!(load_cms_session_in(&session_store).unwrap(), None);
    }

    #[tokio::test]
    async fn account_status_reads_me_and_balance_with_stored_cookie() {
        let server = MockServer::start(move |req, _i| {
            match (req.method.as_str(), req.path.as_str()) {
                ("GET", ME_PATH) => (200, env_ok(user_json())),
                ("GET", POINTS_ME_PATH) => (200, points_body(4321)),
                _ => (404, env_err(9999, "unexpected route")),
            }
        });
        let store = MemoryStore::with_key(COOKIE_PLAINTEXT);
        let account = account_status_in(&server.url(), &store).await.expect("status succeeds");
        assert_eq!(account.user.id, 11);
        assert_eq!(account.user.name, "舵手");
        assert_eq!(account.balance, 4321);

        for req in server.recorded() {
            assert_eq!(
                req.header("cookie"),
                Some(session_cookie_header(COOKIE_PLAINTEXT).as_str()),
                "stored cookie value rides as the cms_session cookie"
            );
            assert!(req.header("authorization").is_none());
        }
    }

    #[tokio::test]
    async fn account_status_parses_points_me_payload() {
        // The payload carries the full `{balance, entries, total}` envelope
        // data; only `balance` is consumed, extra fields are ignored.
        let server = MockServer::start(move |req, _i| {
            match (req.method.as_str(), req.path.as_str()) {
                ("GET", ME_PATH) => (200, env_ok(user_json())),
                ("GET", POINTS_ME_PATH) => (
                    200,
                    env_ok(json!({
                        "balance": 50,
                        "entries": [{
                            "id": 3, "entry_type": "consume", "ref_type": "image",
                            "ref_id": 77, "amount": -20, "balance_after": 50,
                            "reason": "生图扣费", "created_at": 1_700_000_500i64
                        }],
                        "total": 9
                    })),
                ),
                _ => (404, env_err(9999, "unexpected route")),
            }
        });
        let store = MemoryStore::with_key(COOKIE_PLAINTEXT);
        let account = account_status_in(&server.url(), &store).await.expect("status succeeds");
        assert_eq!(account.balance, 50);
    }

    #[tokio::test]
    async fn account_status_without_stored_session_reports_credential_missing() {
        let err = account_status_in("http://127.0.0.1:1", &MemoryStore::new()).await.unwrap_err();
        assert_eq!(err.code(), "CREDENTIAL_MISSING");
    }

    #[tokio::test]
    async fn expired_cookie_is_a_distinguishable_session_error() {
        let server = MockServer::start(move |_req, _i| (401, env_err(1003, "authentication required")));
        let store = MemoryStore::with_key(COOKIE_PLAINTEXT);
        let err = account_status_in(&server.url(), &store).await.unwrap_err();
        assert!(is_session_expired(&err), "expected session-expired, got {err:?}");
        assert_eq!(err.code(), "API_ERROR");
        // Other failures must not be classified as session expiry.
        assert!(!is_session_expired(&RudderError::CredentialMissing));
    }

    #[tokio::test]
    async fn unreachable_server_maps_api_unreachable() {
        let (key_store, session_store) = fresh_stores();
        let err = login_in(
            "http://127.0.0.1:1",
            "fan@example.com",
            "pw-1",
            &key_store,
            &session_store,
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "API_UNREACHABLE");
    }

    #[tokio::test]
    async fn malformed_success_body_maps_bad_response() {
        let server = MockServer::start(move |_req, _i| (200, b"not json".to_vec()));
        let (key_store, session_store) = fresh_stores();
        let err = login_in(&server.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
        assert_eq!(load_cms_session_in(&session_store).unwrap(), None);
    }

    #[tokio::test]
    async fn blank_base_rejected_before_network() {
        let err = register("   ", "fan@example.com", "pw-1", "舵手").await.unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
        assert_eq!(normalize_base("   ").unwrap_err().code(), "INVALID_ARG");
        assert_eq!(
            normalize_base("https://veren.top/api/").unwrap(),
            "https://veren.top/api",
            "trailing slashes are normalized"
        );
    }

    #[tokio::test]
    async fn debug_and_display_outputs_never_leak_secrets() {
        let server = MockServer::start_with_headers(|req, _i| {
            match (req.method.as_str(), req.path.as_str()) {
                ("POST", LOGIN_PATH) => (200, vec![set_cookie_header()], login_ok_body()),
                ("GET", APIKEYS_PATH) => (200, vec![], env_ok(json!([]))),
                ("POST", APIKEYS_PATH) => (200, vec![], created_key_body()),
                ("GET", POINTS_ME_PATH) => (200, vec![], points_body(7)),
                _ => (404, vec![], env_err(9999, "unexpected route")),
            }
        });
        let key_store = MemoryStore::with_key(KEY_PLAINTEXT);
        let session_store = MemoryStore::with_key(COOKIE_PLAINTEXT);
        let session = login_in(&server.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .expect("login flow succeeds");

        let rendered = format!("{session:?}");
        assert!(!rendered.contains(KEY_PLAINTEXT), "Debug leaked the key: {rendered}");
        assert!(!rendered.contains(COOKIE_PLAINTEXT), "Debug leaked the cookie: {rendered}");
        assert!(rendered.contains("fan@example.com"), "profile stays visible for support");

        // Error surfaces (Debug + Display): server message may show, secrets never.
        let fail = MockServer::start(move |_req, _i| (401, env_err(1001, "邮箱或密码错误")));
        let err = login_in(&fail.url(), "fan@example.com", "pw-1", &key_store, &session_store)
            .await
            .unwrap_err();
        let err_text = format!("{err:?} — {err}");
        assert!(err_text.contains("1001:"), "{err_text}");
        assert!(!err_text.contains(KEY_PLAINTEXT), "error leaked the key: {err_text}");
        assert!(!err_text.contains(COOKIE_PLAINTEXT), "error leaked the cookie: {err_text}");

        // Store Debug output is redacted too.
        let stores = format!("{key_store:?} {session_store:?}");
        assert!(!stores.contains(KEY_PLAINTEXT) && !stores.contains(COOKIE_PLAINTEXT), "{stores}");
    }

    #[tokio::test]
    async fn public_entrypoints_refuse_to_persist_when_keychain_disabled() {
        let _guard = KillSwitch::enable();
        let err = login("http://127.0.0.1:1", "fan@example.com", "pw-1").await.unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG", "kill switch must stop login before network");
        let err = account_status("http://127.0.0.1:1").await.unwrap_err();
        assert_eq!(err.code(), "INVALID_ARG");
    }

    #[tokio::test]
    async fn logout_posts_the_session_cookie_and_is_infallible() {
        let server = MockServer::start(move |_req, _i| (200, env_ok(Value::Null)));
        logout_in(&server.url(), COOKIE_PLAINTEXT).await;
        let recorded = server.recorded();
        assert_eq!(recorded.len(), 1, "exactly one logout POST");
        assert_eq!(recorded[0].method, "POST");
        assert_eq!(recorded[0].path, LOGOUT_PATH);
        assert_eq!(
            recorded[0].header("cookie"),
            Some(session_cookie_header(COOKIE_PLAINTEXT).as_str()),
            "the stored cookie rides the logout call"
        );
        assert!(recorded[0].header("authorization").is_none());

        // An unreachable server must not turn sign-out into an error path.
        logout_in("http://127.0.0.1:1", COOKIE_PLAINTEXT).await;
    }

    #[test]
    fn session_expired_matches_the_1003_prefix_only() {
        let expired = RudderError::ApiError {
            status: 401,
            body_summary: format!("{CODE_UNAUTHENTICATED}: authentication required"),
        };
        assert!(is_session_expired(&expired));
        let other = RudderError::ApiError { status: 409, body_summary: "1000: 邮箱已被注册".into() };
        assert!(!is_session_expired(&other));
    }

    // -- env hygiene ---------------------------------------------------------

    /// Serializes the env-mutating test; restores the prior value on drop.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct KillSwitch {
        saved: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl KillSwitch {
        fn enable() -> KillSwitch {
            let lock = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let saved = std::env::var("RUDDER_KEYCHAIN").ok();
            std::env::set_var("RUDDER_KEYCHAIN", "0");
            KillSwitch { saved, _lock: lock }
        }
    }

    impl Drop for KillSwitch {
        fn drop(&mut self) {
            match self.saved.take() {
                Some(value) => std::env::set_var("RUDDER_KEYCHAIN", value),
                None => std::env::remove_var("RUDDER_KEYCHAIN"),
            }
        }
    }
}
