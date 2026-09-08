//! CLI integration tests (docs/ARCHITECTURE.md §9): init/list/pick/export and
//! the dry-run paths. Every test runs against temp directories with an
//! isolated HOME, and NO test ever performs a real network call: `--yes` is
//! only used together with an explicitly empty `OPENAI_API_KEY`, which makes
//! the client fail fast with `CREDENTIAL_MISSING` before touching the network.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

struct Sandbox {
    root: PathBuf,
    home: PathBuf,
}

impl Sandbox {
    fn new(tag: &str) -> Sandbox {
        let unique = format!("rudder-cli-{tag}-{}", uuid::Uuid::new_v4());
        let root = std::env::temp_dir().join(&unique);
        fs::create_dir_all(&root).expect("create sandbox root");
        let home = root.join("home");
        fs::create_dir_all(&home).expect("create isolated home");
        Sandbox { root, home }
    }

    fn project_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn rudder(&self) -> Command {
        let mut cmd = Command::cargo_bin("rudder").expect("rudder binary");
        cmd.env("HOME", &self.home)
            // Belt & braces: even a stray --yes cannot reach a real endpoint.
            .env("OPENAI_BASE_URL", "http://127.0.0.1:1")
            .env("OPENAI_API_KEY", "");
        cmd
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn parse_envelope(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not a JSON envelope ({e}): {stdout}"))
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn make_fake_png(path: &Path, byte: u8) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("candidates dir");
    }
    fs::write(path, vec![byte; 20_480]).expect("fake png");
}

// ---------------------------------------------------------------------------
// init / list
// ---------------------------------------------------------------------------

#[test]
fn init_creates_project_and_list_reports_it() {
    let sb = Sandbox::new("init");
    let dir = sb.project_dir("proj");
    let out = sb
        .rudder()
        .args(["init", "压测项目", "--size", "web", "--dir"])
        .arg(&dir)
        .args(["--brief", "海军蓝 SaaS"])
        .output()
        .expect("init runs");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert!(dir.join("project.json").is_file());
    assert!(dir.join("board/candidates").is_dir());
    assert!(dir.join("refs").is_dir());

    let out = sb
        .rudder()
        .arg("list")
        .arg("--json")
        .arg("--project")
        .arg(&dir)
        .output()
        .expect("list runs");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["name"], "压测项目");
    assert_eq!(envelope["data"]["canvasSize"], "1536x1024");
    assert_eq!(envelope["data"]["anchor"], Value::Null);
}

#[test]
fn init_rejects_invalid_size_with_exit_1_and_size_invalid() {
    let sb = Sandbox::new("badsize");
    let dir = sb.project_dir("proj");
    let out = sb
        .rudder()
        .args(["init", "x", "--size", "1234x567", "--dir"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("init runs");
    assert_eq!(exit_code(&out), 1, "argument errors exit 1");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "SIZE_INVALID");
    assert!(envelope["error"]["hint"].as_str().is_some());
}

#[test]
fn unknown_command_and_parse_errors_exit_1_not_2() {
    let sb = Sandbox::new("parse");
    let out = sb.rudder().arg("frobnicate").output().expect("runs");
    assert_eq!(exit_code(&out), 1, "clap errors map to the 1 = bad arguments class");

    // --json still gets the envelope on a parse error.
    let out = sb.rudder().args(["page", "generate"]).arg("--json").output().expect("runs");
    assert_eq!(exit_code(&out), 1);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "INVALID_ARG");

    // --help exits 0.
    let out = sb.rudder().arg("--help").output().expect("runs");
    assert_eq!(exit_code(&out), 0);
}

// ---------------------------------------------------------------------------
// dry-run discipline (no --yes ⇒ plan only, zero network)
// ---------------------------------------------------------------------------

#[test]
fn board_generate_without_yes_prints_plan_and_writes_nothing() {
    let sb = Sandbox::new("dryboard");
    let dir = sb.project_dir("proj");
    sb.rudder()
        .args(["init", "dry", "--dir"])
        .arg(&dir)
        .output()
        .expect("init");

    // Human mode: plan goes to stderr, stdout stays empty (data only in --json).
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "2", "--quality", "low", "--project"])
        .arg(&dir)
        .output()
        .expect("board generate");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert!(out.stdout.is_empty(), "human mode keeps stdout clean");
    let err = stderr_text(&out);
    assert!(err.contains("[dry-run]"), "plan header on stderr: {err}");
    assert!(err.contains("/v1/images/generations"));
    assert!(err.contains("no network call"));

    // JSON mode: structured plan, no candidates.
    let out = sb
        .rudder()
        .args(["board", "generate", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate --json");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["dryRun"], true);
    assert_eq!(envelope["data"]["plan"]["endpoint"], "generations");
    assert_eq!(envelope["data"]["plan"]["params"]["model"], "gpt-image-2");
    assert_eq!(envelope["data"]["plan"]["params"]["thinking"], "medium");
    assert_eq!(
        envelope["data"]["candidates"].as_array().unwrap().len(),
        0,
        "dry-run writes no candidates"
    );
    // No artifacts on disk.
    let candidates = dir.join("board/candidates");
    let count = fs::read_dir(&candidates).unwrap().count();
    assert_eq!(count, 0, "no candidate files after dry-run");
    // promptLog untouched by dry-run (absent == empty: skipped when empty).
    let project: Value = serde_json::from_slice(&fs::read(dir.join("project.json")).unwrap()).unwrap();
    let log_len = project["promptLog"].as_array().map(|a| a.len()).unwrap_or(0);
    assert_eq!(log_len, 0, "dry-run must not append to promptLog");
}

#[test]
fn page_generate_without_anchor_exits_3_even_dry() {
    let sb = Sandbox::new("noanchor");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "p", "--dir"]).arg(&dir).output().unwrap();
    sb.rudder()
        .args(["page", "add", "dashboard", "--brief", "指标卡x4"])
        .arg("--project")
        .arg(&dir)
        .output()
        .expect("page add");

    let out = sb
        .rudder()
        .args(["page", "generate", "dashboard", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("page generate");
    assert_eq!(exit_code(&out), 3, "project-state errors exit 3");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "NO_ANCHOR");
}

// ---------------------------------------------------------------------------
// full pick/export lifecycle with fabricated candidates (still zero network)
// ---------------------------------------------------------------------------

#[test]
fn pick_promotes_candidate_and_export_bundles_everything() {
    let sb = Sandbox::new("export");
    let dir = sb.project_dir("proj");
    sb.rudder()
        .args(["init", "样例", "--size", "mobile", "--dir"])
        .arg(&dir)
        .output()
        .expect("init");
    sb.rudder()
        .args(["page", "add", "dashboard", "--brief", "KPI 卡x4"])
        .arg("--project")
        .arg(&dir)
        .output()
        .expect("page add");
    sb.rudder()
        .args(["component", "add", "button-set", "--type", "buttons", "--brief", "三态"])
        .arg("--project")
        .arg(&dir)
        .output()
        .expect("component add");

    // Fabricate generation artifacts as if a real API call had happened.
    make_fake_png(&dir.join("board/candidates/0001.png"), 0xAA);
    make_fake_png(&dir.join("board/candidates/0002.png"), 0xBB);
    make_fake_png(&dir.join("pages/dashboard/candidates/0001.png"), 0xCC);
    make_fake_png(&dir.join("components/button-set/candidates/0001.png"), 0xDD);

    // board pick via --json.
    let out = sb
        .rudder()
        .args(["board", "pick", "0001", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board pick");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["anchor"], "0001");
    assert_eq!(envelope["data"]["file"], "board/anchor.png");
    assert!(dir.join("board/anchor.png").is_file());

    // page pick (candidate → current).
    let out = sb
        .rudder()
        .args(["page", "pick", "dashboard", "0001", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("page pick");
    assert_eq!(exit_code(&out), 0);
    assert_eq!(parse_envelope(&out)["data"]["current"], "pages/dashboard/current.png");
    assert!(dir.join("pages/dashboard/current.png").is_file());

    // component pick.
    let out = sb
        .rudder()
        .args(["component", "pick", "button-set", "0001", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("component pick");
    assert_eq!(exit_code(&out), 0);

    // list shows everything in place.
    let out = sb
        .rudder()
        .args(["list", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("list");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["anchor"], "0001");
    assert_eq!(envelope["data"]["pages"][0]["hasCurrent"], true);
    assert_eq!(envelope["data"]["pages"][0]["candidates"], 1);
    assert_eq!(envelope["data"]["components"][0]["hasCurrent"], true);

    // export the bundle.
    let out_dir = sb.project_dir("export-out");
    let out = sb
        .rudder()
        .args(["export", "--out"])
        .arg(&out_dir)
        .arg("--project")
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("export");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    for rel in [
        "manifest.json",
        "PROMPTS.md",
        "DESIGN.template.md",
        "board/anchor.png",
        "pages/dashboard/current.png",
        "components/button-set/current.png",
    ] {
        assert!(out_dir.join(rel).is_file(), "{rel} must be exported");
    }
    let manifest: Value = serde_json::from_slice(&fs::read(out_dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["project"]["name"], "样例");
    assert_eq!(manifest["pages"][0]["slug"], "dashboard");
    assert_eq!(manifest["board"]["anchor"]["file"], "board/anchor.png");
}

#[test]
fn page_pick_rotates_previous_current_into_history() {
    let sb = Sandbox::new("history");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "h", "--dir"]).arg(&dir).output().unwrap();
    sb.rudder()
        .args(["page", "add", "dash", "--brief", "x"])
        .arg("--project")
        .arg(&dir)
        .output()
        .unwrap();
    make_fake_png(&dir.join("pages/dash/candidates/0001.png"), 1);
    make_fake_png(&dir.join("pages/dash/candidates/0002.png"), 2);

    sb.rudder()
        .args(["page", "pick", "dash", "0001", "--project"])
        .arg(&dir)
        .output()
        .unwrap();
    let out = sb
        .rudder()
        .args(["page", "pick", "dash", "0002", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("second pick");
    assert_eq!(exit_code(&out), 0);
    assert!(dir.join("pages/dash/current.png").is_file());
    let history: Vec<_> = fs::read_dir(dir.join("pages/dash/history"))
        .expect("history dir created on rotation")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(history.len(), 1, "previous current rotated into history");
}

// ---------------------------------------------------------------------------
// --yes without credentials fails fast, before any network I/O (exit 2)
// ---------------------------------------------------------------------------

#[test]
fn yes_without_api_key_is_credential_missing_exit_2() {
    let sb = Sandbox::new("nokey");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "k", "--dir"]).arg(&dir).output().unwrap();

    // OPENAI_API_KEY is explicitly empty in the sandbox → CredentialMissing
    // before a single byte reaches the wire.
    let out = sb
        .rudder()
        .args(["board", "generate", "--yes", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate --yes");
    assert_eq!(exit_code(&out), 2, "API-class errors exit 2");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "CREDENTIAL_MISSING");
}

// ---------------------------------------------------------------------------
// config
// ---------------------------------------------------------------------------

#[test]
fn config_set_then_get_roundtrips_and_validates() {
    let sb = Sandbox::new("config");
    let out = sb
        .rudder()
        .args(["config", "set", "quality", "low", "--json"])
        .output()
        .expect("config set");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert_eq!(parse_envelope(&out)["data"]["value"], "low");

    let out = sb
        .rudder()
        .args(["config", "get", "quality", "--json"])
        .output()
        .expect("config get");
    assert_eq!(parse_envelope(&out)["data"]["value"], "low");

    // invalid values are argument errors (exit 1)
    let out = sb
        .rudder()
        .args(["config", "set", "quality", "ultra", "--json"])
        .output()
        .expect("config set bad");
    assert_eq!(exit_code(&out), 1);
    assert_eq!(parse_envelope(&out)["error"]["code"], "INVALID_ARG");

    // secrets are rejected as unknown keys — never stored.
    let out = sb
        .rudder()
        .args(["config", "set", "api_key", "sk-test", "--json"])
        .output()
        .expect("config set secret");
    assert_eq!(exit_code(&out), 1);

    // stored in the isolated home only.
    let config_path = sb.home.join("Rudder/config.json");
    assert!(config_path.is_file());
    let config: Value = serde_json::from_slice(&fs::read(config_path).unwrap()).unwrap();
    assert!(config.get("api_key").is_none(), "no secret keys in config");
}

#[test]
fn dry_run_flag_overrides_yes_for_planning() {
    let sb = Sandbox::new("dryover");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "d", "--dir"]).arg(&dir).output().unwrap();
    // --yes + --dry-run ⇒ still a plan, still no credential requirement.
    let out = sb
        .rudder()
        .args(["board", "generate", "--yes", "--dry-run", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert_eq!(parse_envelope(&out)["data"]["dryRun"], true);
}

// ---------------------------------------------------------------------------
// e2e self-test (dry-run: no --yes)
// ---------------------------------------------------------------------------

#[test]
fn e2e_dry_run_walks_all_steps_without_network() {
    let sb = Sandbox::new("e2edry");
    let out = sb
        .rudder()
        .args(["e2e", "--json"])
        .current_dir(&sb.root)
        .output()
        .expect("e2e runs");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["dryRun"], true);
    let steps: Vec<&str> = envelope["data"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["step"].as_str().unwrap())
        .collect();
    assert_eq!(
        steps,
        vec!["init", "board generate", "page add", "page generate", "component add", "component generate", "export"],
        "all six workflow steps plus export"
    );
    assert!(Path::new(envelope["data"]["projectDir"].as_str().unwrap()).join("project.json").is_file());
    assert!(Path::new(envelope["data"]["exportDir"].as_str().unwrap()).join("manifest.json").is_file());
    // Cleanup the temp e2e workspace.
    fs::remove_dir_all(Path::new(envelope["data"]["projectDir"].as_str().unwrap()).parent().unwrap()).ok();
}

#[test]
fn project_resolution_prefers_cwd_then_last_project() {
    let sb = Sandbox::new("resolve");
    let dir = sb.project_dir("proj");
    sb.rudder()
        .args(["init", "cwd", "--dir"])
        .arg(&dir)
        .output()
        .expect("init");

    // cwd inside the project → no --project needed.
    let out = sb
        .rudder()
        .args(["list", "--json"])
        .current_dir(&dir)
        .output()
        .expect("list from cwd");
    assert_eq!(exit_code(&out), 0);
    assert_eq!(parse_envelope(&out)["data"]["name"], "cwd");

    // from anywhere else → last-project tracking kicks in.
    let elsewhere = sb.root.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let out = sb
        .rudder()
        .args(["list", "--json"])
        .current_dir(&elsewhere)
        .output()
        .expect("list from elsewhere");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert_eq!(parse_envelope(&out)["data"]["name"], "cwd");
}
