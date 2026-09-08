//! Tauri commands: thin wrappers over rudder-core.
//!
//! Errors will be normalized to `{code, message}` in Phase 3
//! (docs/ARCHITECTURE.md §7).

use serde::Serialize;

/// Reply payload of the `ping` command.
#[derive(Serialize)]
pub struct PingReply {
    pub message: String,
    pub version: String,
}

/// Liveness probe used by the frontend to verify the Rust bridge.
#[tauri::command]
pub fn ping() -> PingReply {
    PingReply {
        message: "pong".to_string(),
        version: rudder_core::VERSION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_replies_pong_with_version() {
        let reply = ping();
        assert_eq!(reply.message, "pong");
        assert_eq!(reply.version, rudder_core::VERSION);
    }
}
