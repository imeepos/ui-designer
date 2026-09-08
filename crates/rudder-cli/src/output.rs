//! CLI output discipline (docs/ARCHITECTURE.md §6, UI-REVIEW 缺陷 6):
//! - `--json` mode: data envelope on stdout (`{ok, data|error{code,message,hint}}`);
//! - human mode: a one-line summary on **stdout** (so scripts can judge
//!   success from stdout alone); verbose logs — dry-run plans, warnings —
//!   and all errors stay on stderr;
//! - exit codes: 0 ok · 1 bad argument · 2 API error · 3 project state.

use rudder_core::RudderError;
use serde_json::{json, Value};
use std::io::Write;

/// A command result: a one-line human summary (stdout) plus optional verbose
/// detail (stderr) and the machine data for `--json`.
pub struct CmdResult {
    /// Single-line success summary (human mode writes this to stdout).
    pub summary: String,
    /// Multi-line detail (dry-run plans); human mode writes this to stderr.
    pub detail: Option<String>,
    pub data: Value,
}

impl CmdResult {
    pub fn new(summary: impl Into<String>, data: Value) -> CmdResult {
        CmdResult { summary: summary.into(), detail: None, data }
    }

    pub fn with_detail(
        summary: impl Into<String>,
        detail: impl Into<String>,
        data: Value,
    ) -> CmdResult {
        CmdResult { summary: summary.into(), detail: Some(detail.into()), data }
    }
}

/// Print a successful result and return the process exit code (always 0).
pub fn emit_ok(json_mode: bool, result: &CmdResult) -> i32 {
    if json_mode {
        let envelope = json!({ "ok": true, "data": result.data });
        println!("{envelope}");
    } else {
        println!("{}", result.summary);
        if let Some(detail) = &result.detail {
            eprintln!("{detail}");
        }
    }
    let _ = std::io::stdout().flush();
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

/// One-line dry-run summary for stdout (UI-REVIEW 缺陷 6): humans and
/// scripts see the outcome at a glance; the full plan stays on stderr.
pub fn format_plan_summary(plan: &rudder_core::image::RequestPlan, kind: &str, target: &str) -> String {
    let target = if target.is_empty() {
        String::new()
    } else {
        format!(" `{target}`")
    };
    format!(
        "[dry-run] {kind}{target} → {} {} planned (no network call; pass --yes to run for real)",
        plan.method,
        plan.endpoint
    )
}
