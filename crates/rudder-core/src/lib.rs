//! Rudder core library.
//!
//! Modules (docs/ARCHITECTURE.md §3-6):
//! - [`canvas`] — canvas size model + gpt-image-2 constraint validation
//! - [`store`]  — project storage with atomic writes (tempfile + rename)
//! - [`config`] — `~/Rudder/config.json` defaults + last-used project;
//!   [`config::credential`] — OS-keychain API key (env → keychain → none)
//!   and the free `/v1/models` connectivity probe
//! - [`prompt`] — three-part prompt engine for board/page/component
//! - [`image`]  — gpt-image-2 client with dry-run, retry, b64 decode
//! - [`export`] — asset bundle export (images + manifest + PROMPTS.md)
//! - [`ops`]    — high-level flows shared by CLI and desktop shell

pub mod canvas;
pub mod config;
pub mod error;
pub mod store;
pub mod ops;
pub mod export;
pub mod image;
pub mod prompt;

pub use canvas::CanvasSize;
pub use config::Config;
pub use error::{Result, RudderError};

/// Crate version, surfaced to both CLI and desktop shell.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_exposed() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn modules_reexport_cleanly() {
        // The unified error type keeps its code/exit contract.
        let err = RudderError::NoProject;
        assert_eq!(err.code(), "NO_PROJECT");
        assert_eq!(err.exit_code(), 3);
    }
}
