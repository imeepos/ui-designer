//! CLI output discipline (docs/ARCHITECTURE.md §6):
//! - data → stdout, but only in `--json` mode (`{ok, data|error{code,message,hint}}`);
//! - human-readable one-line summaries and logs → stderr (references/cli.md);
//! - exit codes: 0 ok · 1 bad argument · 2 API error · 3 project state.

use rudder_core::RudderError;
use serde_json::{json, Value};
use std::io::Write;

/// A command result: machine data + a one-line human summary.
pub struct CmdResult {
    pub human: String,
    pub data: Value,
}

impl CmdResult {
    pub fn new(human: impl Into<String>, data: Value) -> CmdResult {
        CmdResult { human: human.into(), data }
    }
}

/// Print a successful result and return the process exit code (always 0).
pub fn emit_ok(json_mode: bool, result: &CmdResult) -> i32 {
    if json_mode {
        let envelope = json!({ "ok": true, "data": result.data });
        println!("{envelope}");
        let _ = std::io::stdout().flush();
    } else {
        eprintln!("{}", result.human);
    }
    0
}

/// Print an error and return the mapped exit code (1 / 2 / 3).
pub fn emit_err(json_mode: bool, err: &RudderError) -> i32 {
    if json_mode {
        let envelope = json!({
            "ok": false,
            "error": {
                "code": err.code(),
                "message": err.to_string(),
                "hint": err.hint(),
            }
        });
        println!("{envelope}");
        let _ = std::io::stdout().flush();
    } else {
        eprintln!("error [{}]: {}", err.code(), err);
        eprintln!("hint: {}", err.hint());
    }
    err.exit_code()
}

/// Format a dry-run plan as a short multi-line human summary (stderr).
pub fn format_plan(plan: &rudder_core::image::RequestPlan) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "[dry-run] {} {} (auth header: {})",
        plan.method,
        plan.url,
        if plan.auth_header_present { "present" } else { "MISSING — set OPENAI_API_KEY" }
    ));
    lines.push(format!("  params: {}", plan.params));
    for img in &plan.images {
        let state = if img.missing {
            "missing".to_string()
        } else {
            format!("{} bytes", img.bytes.unwrap_or(0))
        };
        lines.push(format!("  image[{}] {}: {}", img.role, img.path, state));
    }
    lines.push("  no network call — pass --yes to run for real".to_string());
    lines.join("\n")
}
