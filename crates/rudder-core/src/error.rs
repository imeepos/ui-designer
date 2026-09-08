//! Core error type (`thiserror`), per AGENTS.md engineering conventions.

/// Result alias used across the core library.
pub type Result<T> = std::result::Result<T, RudderError>;

/// Unified error type for rudder-core.
///
/// Phase 2 will add storage/prompt/image/export variants with stable
/// error codes (docs/ARCHITECTURE.md §6 exit-code contract).
#[derive(Debug, thiserror::Error)]
pub enum RudderError {
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}
