//! Rudder core library.
//!
//! Phase 1 scaffold: compilable shell only.
//! Planned modules (docs/ARCHITECTURE.md §3-5):
//! - `store`  — project storage with atomic writes (tempfile + rename)
//! - `prompt` — three-part prompt engine for board/page/component
//! - `image`  — gpt-image-2 client with dry-run, retry, b64 decode
//! - `export` — asset bundle export (images + manifest.json + PROMPTS.md)

pub mod error;

pub use error::{Result, RudderError};

/// Crate version, surfaced to both CLI and desktop shell.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_stable() {
        let err = RudderError::NotImplemented("store");
        assert_eq!(err.to_string(), "not implemented yet: store");
    }

    #[test]
    fn version_is_exposed() {
        assert!(!VERSION.is_empty());
    }
}
