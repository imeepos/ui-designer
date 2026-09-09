//! Unified command error envelope: `{code, message, hint}`.
//!
//! `code` uses the frontend `ApiErrorCode` vocabulary from
//! `src/lib/api/types.ts` (i18n keys `errors.<code>.*`); core
//! `RudderError::code()` values are translated per command context.

use rudder_core::RudderError;
use serde::Serialize;

/// Frontend-facing error codes (`src/lib/api/types.ts` `ApiErrorCode`).
pub mod codes {
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";
    pub const ANCHOR_REQUIRED: &str = "ANCHOR_REQUIRED";
    pub const ANCHOR_LOCKED: &str = "ANCHOR_LOCKED";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const DUPLICATE_SLUG: &str = "DUPLICATE_SLUG";
    pub const DUPLICATE_NAME: &str = "DUPLICATE_NAME";
    pub const NO_CREDENTIALS: &str = "NO_CREDENTIALS";
    pub const KEYCHAIN_ACCESS: &str = "KEYCHAIN_ACCESS";
    pub const EXPORT_FAILED: &str = "EXPORT_FAILED";
    pub const API_ERROR: &str = "API_ERROR";
    pub const UNKNOWN: &str = "UNKNOWN";
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub hint: String,
}

impl CommandError {
    pub fn new(code: &str, message: impl Into<String>, hint: impl Into<String>) -> Self {
        CommandError {
            code: code.to_string(),
            message: message.into(),
            hint: hint.into(),
        }
    }

    /// Plain `ANCHOR_LOCKED` (no core counterpart).
    pub fn anchor_locked(message: impl Into<String>) -> Self {
        Self::new(
            codes::ANCHOR_LOCKED,
            message,
            "pick another candidate as the anchor first",
        )
    }

    /// Translate a core error with the default code mapping.
    pub fn from_core(err: &RudderError) -> Self {
        Self::map_core(err, codes::VALIDATION_ERROR, codes::UNKNOWN)
    }

    /// Translate a core error, overriding the code used for
    /// `ALREADY_EXISTS` and `IO_ERROR` (e.g. `DUPLICATE_SLUG`, `EXPORT_FAILED`).
    pub fn with_overrides(
        err: &RudderError,
        exists_code: &str,
        io_code: &str,
    ) -> Self {
        Self::map_core(err, exists_code, io_code)
    }

    fn map_core(err: &RudderError, exists_code: &str, io_code: &str) -> Self {
        let code = match err {
            RudderError::SizeInvalid { .. }
            | RudderError::InvalidArg { .. }
            | RudderError::TemplateNotFound { .. } => {
                codes::VALIDATION_ERROR
            }
            RudderError::CredentialMissing => codes::NO_CREDENTIALS,
            RudderError::KeychainAccess { .. } => codes::KEYCHAIN_ACCESS,
            RudderError::ApiError { .. }
            | RudderError::RateLimited { .. }
            | RudderError::ApiUnreachable { .. }
            | RudderError::BadResponse { .. } => codes::API_ERROR,
            RudderError::NoAnchor => codes::ANCHOR_REQUIRED,
            RudderError::NotFound { .. }
            | RudderError::NoProject
            | RudderError::NotAProject { .. } => codes::NOT_FOUND,
            RudderError::AlreadyExists { .. } => exists_code,
            RudderError::Io(_) => io_code,
        };
        CommandError {
            code: code.to_string(),
            message: err.to_string(),
            hint: err.hint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_map_to_frontend_codes() {
        let cases: [(RudderError, &str); 8] = [
            (RudderError::SizeInvalid { detail: "x".into() }, "VALIDATION_ERROR"),
            (RudderError::NoAnchor, "ANCHOR_REQUIRED"),
            (RudderError::NotFound { what: "page".into() }, "NOT_FOUND"),
            (RudderError::RateLimited { body_summary: "x".into() }, "API_ERROR"),
            (RudderError::CredentialMissing, "NO_CREDENTIALS"),
            (RudderError::KeychainAccess { detail: "x".into() }, "KEYCHAIN_ACCESS"),
            (RudderError::NotAProject { path: "/x".into() }, "NOT_FOUND"),
            (RudderError::BadResponse { detail: "x".into() }, "API_ERROR"),
        ];
        for (err, code) in cases {
            assert_eq!(CommandError::from_core(&err).code, code);
        }
    }

    #[test]
    fn overrides_apply_to_exists_and_io() {
        let exists = RudderError::AlreadyExists { what: "page `dash`".into() };
        assert_eq!(
            CommandError::with_overrides(&exists, "DUPLICATE_SLUG", "UNKNOWN").code,
            "DUPLICATE_SLUG"
        );
        let io = RudderError::Io(std::io::Error::other("disk"));
        assert_eq!(
            CommandError::with_overrides(&io, "VALIDATION_ERROR", "EXPORT_FAILED").code,
            "EXPORT_FAILED"
        );
        assert_eq!(CommandError::from_core(&io).code, "UNKNOWN");
    }

    #[test]
    fn envelope_serializes_camel_case() {
        let err = CommandError::new("ANCHOR_REQUIRED", "no anchor", "pick one first");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "ANCHOR_REQUIRED");
        assert_eq!(json["message"], "no anchor");
        assert_eq!(json["hint"], "pick one first");
    }
}
