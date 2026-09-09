//! Core error type (`thiserror`), per AGENTS.md engineering conventions.
//!
//! Every error carries a stable machine `code()` (surfaced in `--json`
//! envelopes and the export manifest) and an `exit_code()` mapping to the
//! CLI contract in docs/ARCHITECTURE.md §6:
//! `0` ok · `1` bad argument · `2` API error · `3` project state error.

/// Result alias used across the core library.
pub type Result<T> = std::result::Result<T, RudderError>;

/// Unified error type for rudder-core.
#[derive(Debug, thiserror::Error)]
pub enum RudderError {
    // ---- argument class (exit 1) --------------------------------------
    /// Canvas size violates gpt-image-2 constraints (16-multiples, ratio,
    /// megapixel window) or is not parseable.
    #[error("invalid canvas size: {detail}")]
    SizeInvalid { detail: String },

    /// Any other invalid CLI argument or enum value.
    #[error("{detail}")]
    InvalidArg { detail: String },

    // ---- API class (exit 2) -------------------------------------------
    /// No API key could be resolved from the environment or the OS keychain
    /// but a real (non dry-run) call was made.
    #[error("no API key configured (env OPENAI_API_KEY or OS keychain)")]
    CredentialMissing,

    /// Reading/writing the OS keychain failed (locked keychain, denied
    /// access, unsupported platform store).
    #[error("OS keychain access failed: {detail}")]
    KeychainAccess { detail: String },

    /// Endpoint returned a non-success status after retries.
    #[error("image API returned HTTP {status}")]
    ApiError { status: u16, body_summary: String },

    /// Rate limited (HTTP 429) after exhausting retries.
    #[error("image API rate limited (429) after retries")]
    RateLimited { body_summary: String },

    /// Could not reach the endpoint at all (DNS/TCP/TLS/timeout).
    #[error("image API unreachable: {detail}")]
    ApiUnreachable { detail: String },

    /// Endpoint answered 2xx but the payload could not be interpreted.
    #[error("unexpected image API response: {detail}")]
    BadResponse { detail: String },

    // ---- project state class (exit 3) ----------------------------------
    /// No project could be resolved (no --project, cwd is not a project,
    /// no last-used project).
    #[error("no rudder project found (run `rudder init` or pass --project)")]
    NoProject,

    /// The resolved directory exists but is not a rudder project.
    #[error("{path} is not a rudder project (missing project.json)")]
    NotAProject { path: String },

    /// Operation requires a picked board anchor.
    #[error("no board anchor picked yet (run `rudder board pick <candidate-id>`)")]
    NoAnchor,

    /// A referenced entity (page, component, candidate, config key) does not exist.
    #[error("{what} not found")]
    NotFound { what: String },

    /// The entity already exists (duplicate page slug / component name / project).
    #[error("{what} already exists")]
    AlreadyExists { what: String },

    /// Filesystem / serialization failure while touching project state.
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),
}

impl RudderError {
    /// Stable machine-readable error code (docs/ARCHITECTURE.md §6).
    pub fn code(&self) -> &'static str {
        match self {
            RudderError::SizeInvalid { .. } => "SIZE_INVALID",
            RudderError::InvalidArg { .. } => "INVALID_ARG",
            RudderError::CredentialMissing => "CREDENTIAL_MISSING",
            RudderError::KeychainAccess { .. } => "KEYCHAIN_ACCESS",
            RudderError::ApiError { .. } => "API_ERROR",
            RudderError::RateLimited { .. } => "RATE_LIMITED",
            RudderError::ApiUnreachable { .. } => "API_UNREACHABLE",
            RudderError::BadResponse { .. } => "BAD_RESPONSE",
            RudderError::NoProject => "NO_PROJECT",
            RudderError::NotAProject { .. } => "NOT_A_PROJECT",
            RudderError::NoAnchor => "NO_ANCHOR",
            RudderError::NotFound { .. } => "NOT_FOUND",
            RudderError::AlreadyExists { .. } => "ALREADY_EXISTS",
            RudderError::Io(_) => "IO_ERROR",
        }
    }

    /// CLI exit code contract: 1 argument / 2 API / 3 project state.
    pub fn exit_code(&self) -> i32 {
        match self {
            RudderError::SizeInvalid { .. } | RudderError::InvalidArg { .. } => 1,
            RudderError::CredentialMissing
            | RudderError::KeychainAccess { .. }
            | RudderError::ApiError { .. }
            | RudderError::RateLimited { .. }
            | RudderError::ApiUnreachable { .. }
            | RudderError::BadResponse { .. } => 2,
            RudderError::NoProject
            | RudderError::NotAProject { .. }
            | RudderError::NoAnchor
            | RudderError::NotFound { .. }
            | RudderError::AlreadyExists { .. }
            | RudderError::Io(_) => 3,
        }
    }

    /// Actionable recovery hint for the `error.hint` field.
    pub fn hint(&self) -> String {
        match self {
            RudderError::SizeInvalid { .. } => {
                "sides must be multiples of 16, aspect ratio <= 3:1, total pixels 0.65M-8.3M; \
                 or use presets web/mobile/desktop"
                    .to_string()
            }
            RudderError::InvalidArg { .. } => "check the flag value against references/cli.md".to_string(),
            RudderError::CredentialMissing => {
                "export OPENAI_API_KEY (env wins) or store a key: `echo <key> | rudder config set \
                 api-key`, or the desktop Settings dialog"
                    .to_string()
            }
            RudderError::KeychainAccess { .. } => {
                "unlock / authorize your OS keychain (Keychain Access on macOS), or disable it \
                 with RUDDER_KEYCHAIN=0 and use OPENAI_API_KEY instead"
                    .to_string()
            }
            RudderError::ApiError { .. } => "inspect the endpoint status; retry with lower --n or quality".to_string(),
            RudderError::RateLimited { .. } => "wait ~30s and retry once, then reduce --n or quality".to_string(),
            RudderError::ApiUnreachable { .. } => "check OPENAI_BASE_URL and network connectivity".to_string(),
            RudderError::BadResponse { .. } => "the endpoint replied 2xx with an unexpected body; verify it serves gpt-image-2".to_string(),
            RudderError::NoProject => "run `rudder init` first or pass --project <dir>".to_string(),
            RudderError::NotAProject { .. } => "point --project at a directory containing project.json".to_string(),
            RudderError::NoAnchor => "generate board candidates and run `rudder board pick` first".to_string(),
            RudderError::NotFound { .. } => "run `rudder list` to see existing pages/components/candidates".to_string(),
            RudderError::AlreadyExists { .. } => "pick another name, or run `rudder list` to inspect the project".to_string(),
            RudderError::Io(_) => "check directory permissions and free disk space".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_stable() {
        let err = RudderError::NoAnchor;
        assert_eq!(err.to_string(), "no board anchor picked yet (run `rudder board pick <candidate-id>`)");
        assert_eq!(err.code(), "NO_ANCHOR");
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn exit_code_classes() {
        assert_eq!(RudderError::SizeInvalid { detail: "x".into() }.exit_code(), 1);
        assert_eq!(RudderError::InvalidArg { detail: "x".into() }.exit_code(), 1);
        assert_eq!(RudderError::RateLimited { body_summary: "x".into() }.exit_code(), 2);
        assert_eq!(RudderError::CredentialMissing.exit_code(), 2);
        assert_eq!(RudderError::NoProject.exit_code(), 3);
        assert_eq!(RudderError::NotFound { what: "page".into() }.exit_code(), 3);
    }
}
