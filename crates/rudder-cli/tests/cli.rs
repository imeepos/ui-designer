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
            .env("OPENAI_API_KEY", "")
            // Hermetic: never consult the developer machine's keychain.
            .env("RUDDER_KEYCHAIN", "0");
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

    // Human mode: a one-line dry-run summary on stdout (UI-REVIEW 缺陷 6),
    // the verbose plan on stderr.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "2", "--quality", "low", "--project"])
        .arg(&dir)
        .output()
        .expect("board generate");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[dry-run]"), "stdout one-line summary: {stdout}");
    assert_eq!(stdout.trim().lines().count(), 1, "exactly one summary line");
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
    // UI-REVIEW 缺陷 1: the plan records a seed even when --seed is omitted.
    assert!(envelope["data"]["plan"]["params"]["seed"].is_u64(), "seed recorded for reproducibility");
    // UI-REVIEW 缺陷 5: default quality is the exploration tier.
    assert_eq!(envelope["data"]["plan"]["params"]["quality"], "low");
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

    // export the bundle (default: anchor + picked currents, no candidates).
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
    assert!(
        !out_dir.join("board/candidates").exists(),
        "default export must not include exploration candidates (UI-REVIEW #7)"
    );
    let manifest: Value = serde_json::from_slice(&fs::read(out_dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["project"]["name"], "样例");
    assert_eq!(manifest["pages"][0]["slug"], "dashboard");
    assert_eq!(manifest["board"]["anchor"]["file"], "board/anchor.png");

    // --with-candidates brings the exploration drafts back (UI-REVIEW #7).
    let out_dir2 = sb.project_dir("export-out-cands");
    let out = sb
        .rudder()
        .args(["export", "--out"])
        .arg(&out_dir2)
        .arg("--with-candidates")
        .arg("--project")
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("export --with-candidates");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert!(out_dir2.join("board/candidates/0001.png").is_file());
    assert!(out_dir2.join("board/candidates/0002.png").is_file());
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

    // secrets are rejected as unknown keys — never stored. The `api_key`
    // spelling routes to the keychain path, which forbids argv values.
    let out = sb
        .rudder()
        .args(["config", "set", "api_key", "secret-value", "--json"])
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
fn config_base_url_set_get_roundtrips_and_validates() {
    let sb = Sandbox::new("cfgbase");
    let out = sb
        .rudder()
        .env_remove("OPENAI_BASE_URL")
        .args(["config", "set", "base_url", "https://proxy.example.com/", "--json"])
        .output()
        .expect("config set base_url");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    assert_eq!(parse_envelope(&out)["data"]["value"], "https://proxy.example.com");

    let out = sb
        .rudder()
        .env_remove("OPENAI_BASE_URL")
        .args(["config", "get", "base_url", "--json"])
        .output()
        .expect("config get base_url");
    assert_eq!(parse_envelope(&out)["data"]["value"], "https://proxy.example.com");

    let out = sb
        .rudder()
        .args(["config", "set", "base_url", "proxy.example.com", "--json"])
        .output()
        .expect("config set base_url bad");
    assert_eq!(exit_code(&out), 1);
    assert_eq!(parse_envelope(&out)["error"]["code"], "INVALID_ARG");
}

#[test]
fn config_set_api_key_never_accepts_the_secret_via_argv() {
    let sb = Sandbox::new("cfgargv");
    let out = sb
        .rudder()
        .args(["config", "set", "api-key", "secret-value", "--json"])
        .output()
        .expect("config set api-key with argv value");
    assert_eq!(exit_code(&out), 1, "argv values are a shell-history leak");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "INVALID_ARG");
    assert!(
        envelope["error"]["message"].as_str().unwrap_or_default().contains("stdin"),
        "hint must point at stdin: {envelope}"
    );
}

#[test]
fn config_set_api_key_requires_a_piped_value() {
    let sb = Sandbox::new("cfgstdin");
    let out = sb
        .rudder()
        .args(["config", "set", "api-key", "--json"])
        .write_stdin("")
        .output()
        .expect("config set api-key with empty stdin");
    assert_eq!(exit_code(&out), 1, "empty stdin must fail before any keychain write");
    assert_eq!(parse_envelope(&out)["error"]["code"], "INVALID_ARG");
}

#[test]
fn config_clear_rejects_non_credential_keys() {
    let sb = Sandbox::new("cfgclear");
    let out = sb
        .rudder()
        .args(["config", "clear", "quality", "--json"])
        .output()
        .expect("config clear quality");
    assert_eq!(exit_code(&out), 1);
    assert_eq!(parse_envelope(&out)["error"]["code"], "INVALID_ARG");
}

#[test]
fn config_test_reports_none_without_credentials() {
    let sb = Sandbox::new("cfgtestnone");
    let out = sb
        .rudder()
        .args(["config", "test", "--json"])
        .output()
        .expect("config test without credentials");
    assert_eq!(exit_code(&out), 0, "reporting `none` is not an error");
    let data = parse_envelope(&out)["data"].clone();
    assert_eq!(data["keySource"], "none");
    assert_eq!(data["tested"], false);
    assert_eq!(data["modelCount"], Value::Null);
    assert_eq!(data["baseUrl"], "http://127.0.0.1:1");
}

/// One-shot mock `/v1/models` server for the free connectivity probe.
fn spawn_models_server(body: &'static str) -> String {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    format!("http://{addr}")
}

#[test]
fn config_test_probes_models_and_reports_source_and_count() {
    let sb = Sandbox::new("cfgtestmock");
    let base = spawn_models_server(
        r#"{"data":[{"id":"gpt-image-2"},{"id":"gpt-4o"}]}"#,
    );
    let out = sb
        .rudder()
        .env("OPENAI_BASE_URL", &base)
        .env("OPENAI_API_KEY", "sandbox-key")
        .args(["config", "test", "--json"])
        .output()
        .expect("config test against mock server");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let data = parse_envelope(&out)["data"].clone();
    assert_eq!(data["keySource"], "env");
    assert_eq!(data["tested"], true);
    assert_eq!(data["modelCount"], 2);
    assert_eq!(data["imageModelAvailable"], true);
    assert_eq!(data["baseUrl"], base);
    // The piped key never appears in stdout (json envelope or human summary).
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(!stdout.contains("sandbox-key"), "key leaked to stdout: {stdout}");
}

#[test]
fn config_test_maps_unreachable_endpoint_to_exit_2() {
    let sb = Sandbox::new("cfgtestdown");
    let out = sb
        .rudder()
        .env("OPENAI_API_KEY", "sandbox-key")
        .args(["config", "test", "--json"])
        .output()
        .expect("config test against closed port");
    assert_eq!(exit_code(&out), 2, "API-class errors exit 2");
    assert_eq!(parse_envelope(&out)["error"]["code"], "API_UNREACHABLE");
}

#[test]
fn config_model_defaults_then_set_then_get_roundtrip() {
    let sb = Sandbox::new("cfgmodel");
    let default_out = sb
        .rudder()
        .env("OPENAI_MODEL", "")
        .args(["config", "get", "model", "--json"])
        .output()
        .expect("config get model on a fresh config");
    assert_eq!(exit_code(&default_out), 0, "stderr: {}", stderr_text(&default_out));
    assert_eq!(
        parse_envelope(&default_out)["data"]["value"],
        "gpt-image-2",
        "old configs without a model field resolve to the documented default"
    );

    let set_out = sb
        .rudder()
        .env("OPENAI_MODEL", "")
        .args(["config", "set", "model", "my-image-model", "--json"])
        .output()
        .expect("config set model");
    assert_eq!(exit_code(&set_out), 0, "stderr: {}", stderr_text(&set_out));
    assert_eq!(parse_envelope(&set_out)["data"]["value"], "my-image-model");

    let get_out = sb
        .rudder()
        .env("OPENAI_MODEL", "")
        .args(["config", "get", "model", "--json"])
        .output()
        .expect("config get model after set");
    assert_eq!(parse_envelope(&get_out)["data"]["value"], "my-image-model");

    // env overrides the stored config value (same chain as base_url).
    let env_out = sb
        .rudder()
        .env("OPENAI_MODEL", "env-model-7")
        .args(["config", "get", "model", "--json"])
        .output()
        .expect("config get model with env override");
    assert_eq!(parse_envelope(&env_out)["data"]["value"], "env-model-7");
}

#[test]
fn config_test_reports_the_effective_model() {
    let sb = Sandbox::new("cfgtestmodel");
    let base = spawn_models_server(
        r#"{"data":[{"id":"custom-image"},{"id":"gpt-4o"}]}"#,
    );
    let out = sb
        .rudder()
        .env("OPENAI_BASE_URL", &base)
        .env("OPENAI_API_KEY", "sandbox-key")
        .env("OPENAI_MODEL", "custom-image")
        .args(["config", "test", "--json"])
        .output()
        .expect("config test with a custom model");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let data = parse_envelope(&out)["data"].clone();
    assert_eq!(data["model"], "custom-image");
    assert_eq!(data["imageModelAvailable"], true);
    // The human summary names the model too.
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(stdout.contains("custom-image"), "summary must show the model: {stdout}");
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

// ---------------------------------------------------------------------------
// update subcommands (UI-REVIEW 缺陷 2)
// ---------------------------------------------------------------------------

#[test]
fn update_commands_amend_briefs_without_hand_editing() {
    let sb = Sandbox::new("update");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "旧名", "--dir"]).arg(&dir).output().unwrap();
    sb.rudder()
        .args(["page", "add", "dashboard", "--brief", "旧简报"])
        .arg("--project")
        .arg(&dir)
        .output()
        .unwrap();
    sb.rudder()
        .args(["component", "add", "button-set", "--type", "buttons", "--brief", "旧简报"])
        .arg("--project")
        .arg(&dir)
        .output()
        .unwrap();

    // project update: rename + style brief swap (human mode → stdout summary).
    let out = sb
        .rudder()
        .args(["project", "update", "--name", "新名字", "--style-brief", "黄铜+深蓝"])
        .arg("--project")
        .arg(&dir)
        .output()
        .expect("project update");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("project updated"), "one-line summary on stdout: {stdout}");

    // page update.
    let out = sb
        .rudder()
        .args(["page", "update", "dashboard", "--brief", "新布局简报", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("page update");
    assert_eq!(exit_code(&out), 0);
    assert_eq!(parse_envelope(&out)["data"]["brief"], "新布局简报");

    // component update (type + brief).
    let out = sb
        .rudder()
        .args(["component", "update", "button-set", "--type", "cards", "--brief", "新简报", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("component update");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["type"], "cards");
    assert_eq!(envelope["data"]["brief"], "新简报");

    // everything persisted.
    let project: Value = serde_json::from_slice(&fs::read(dir.join("project.json")).unwrap()).unwrap();
    assert_eq!(project["name"], "新名字");
    assert_eq!(project["style_brief"], "黄铜+深蓝", "style brief persisted (snake_case storage key)");
    let raw = fs::read_to_string(dir.join("project.json")).unwrap();
    assert!(raw.contains("新布局简报"), "page brief persisted");
    assert!(raw.contains("\"type\": \"cards\"") || raw.contains("\"type\":\"cards\""), "component type persisted");

    // error paths: unknown target exits 3, empty update exits 1.
    let out = sb
        .rudder()
        .args(["page", "update", "missing", "--brief", "x", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("page update missing");
    assert_eq!(exit_code(&out), 3);
    let out = sb
        .rudder()
        .args(["project", "update", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("project update no fields");
    assert_eq!(exit_code(&out), 1);
    assert_eq!(parse_envelope(&out)["error"]["code"], "INVALID_ARG");
}

// ---------------------------------------------------------------------------
// generate envelope shape + prompt dedupe against a local mock endpoint
// (UI-REVIEW 缺陷 1/3/4; zero external network)
// ---------------------------------------------------------------------------

/// Minimal HTTP/1.1 mock server (same proven pattern as
/// rudder-core::image::tests): per-connection thread, bounded reads via
/// read timeout + Content-Length, respond, close.
mod mock {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct RecordedRequest {
        /// Kept for debugging; the envelope assertions read project state.
        #[allow(dead_code)]
        body: Vec<u8>,
    }

    /// Always answers `200` with two b64 images; records request bodies.
    pub struct MockServer {
        addr: std::net::SocketAddr,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
    }

    impl MockServer {
        pub fn start() -> MockServer {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
            let addr = listener.local_addr().expect("local addr");
            let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::new(Mutex::new(Vec::new()));
            {
                let requests = requests.clone();
                std::thread::spawn(move || {
                    for stream in listener.incoming() {
                        let Ok(mut stream) = stream else { break };
                        let requests = requests.clone();
                        std::thread::spawn(move || {
                            let Ok(Some(req)) = read_request(&mut stream) else { return };
                            requests.lock().expect("request log lock").push(req);
                            let body =
                                br#"{"created":1,"data":[{"b64_json":"aGVsbG8="},{"b64_json":"aGVsbG8="}]}"#;
                            let head = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(head.as_bytes());
                            let _ = stream.write_all(body);
                            let _ = stream.flush();
                        });
                    }
                });
            }
            MockServer { addr, requests }
        }

        pub fn url(&self) -> String {
            format!("http://{}", self.addr)
        }

        #[allow(dead_code)]
        pub fn request_count(&self) -> usize {
            self.requests.lock().expect("request log lock").len()
        }
    }

    fn find_double_crlf(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|w| w == b"\r\n\r\n")
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<RecordedRequest>> {
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let mut buf: Vec<u8> = Vec::new();
        let mut tmp = [0u8; 8192];
        let head_end = loop {
            if let Some(pos) = find_double_crlf(&buf) {
                break pos;
            }
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                return Ok(None);
            }
            buf.extend_from_slice(&tmp[..n]);
        };
        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
        let content_length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.trim()
                    .eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        let mut body = buf[head_end + 4..].to_vec();
        while body.len() < content_length {
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&tmp[..n]);
        }
        body.truncate(content_length);
        Ok(Some(RecordedRequest { body }))
    }
}

#[test]
fn real_generate_is_reproducible_symmetric_and_prompt_deduped() {
    let sb = Sandbox::new("mockgen");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "mock", "--dir"]).arg(&dir).output().unwrap();
    sb.rudder()
        .args(["page", "add", "dashboard", "--brief", "KPI 卡x4"])
        .arg("--project")
        .arg(&dir)
        .output()
        .unwrap();

    let server = mock::MockServer::start();
    let base = server.url();
    let mut cmd = sb.rudder();
    cmd.env("OPENAI_BASE_URL", &base).env("OPENAI_API_KEY", "test-key");

    // board generate --n 2 --yes: real run against the local mock.
    let out = cmd
        .args(["board", "generate", "--n", "2", "--yes", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate real");
    assert_eq!(
        exit_code(&out),
        0,
        "stderr: {} · stdout: {}",
        stderr_text(&out),
        String::from_utf8_lossy(&out.stdout)
    );
    let envelope = parse_envelope(&out);
    let candidates = envelope["data"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    for row in candidates {
        // UI-REVIEW 缺陷 4: rows reference the prompt, never repeat it.
        assert!(row.get("prompt").is_none(), "candidate rows must not carry prompt: {row}");
        assert!(row["id"].is_string() && row["file"].is_string());
        // UI-REVIEW 缺陷 1: seed recorded per candidate row.
        assert!(row["seed"].is_u64(), "candidate row carries the seed: {row}");
    }
    // The prompt appears exactly once (inside plan.params.prompt).
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.matches("Purpose: a UI design system board").count(),
        1,
        "prompt must appear once per response, not per candidate"
    );
    assert!(dir.join("board/candidates/0001.png").is_file());

    // Pick the anchor, then single-target page generate: symmetric shape.
    sb.rudder()
        .args(["board", "pick", "0001", "--project"])
        .arg(&dir)
        .output()
        .unwrap();
    let out = sb
        .rudder()
        .env("OPENAI_BASE_URL", &base)
        .env("OPENAI_API_KEY", "test-key")
        .args(["page", "generate", "dashboard", "--yes", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("page generate real");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    // UI-REVIEW 缺陷 3: page answers with the exact board shape.
    assert_eq!(envelope["data"]["kind"], "page");
    assert_eq!(envelope["data"]["target"], "dashboard");
    assert!(envelope["data"]["candidates"].is_array(), "top-level candidates (no results wrapper)");
    assert!(envelope["data"]["candidates"].as_array().unwrap()[0]["seed"].is_u64());
    assert!(envelope["data"]["results"].is_null(), "no results wrapper for single target");

    // project.json lineage records the seed for both batches (缺陷 1).
    let raw = fs::read_to_string(dir.join("project.json")).unwrap();
    let project: Value = serde_json::from_str(&raw).unwrap();
    assert!(project["board_generations"][0]["params"]["seed"].is_u64());
    assert!(project["pages"][0]["generations"][0]["params"]["seed"].is_u64());
    // Both requests went through the mock (board + page edits).
    assert_eq!(server.request_count(), 2);
}

// ---------------------------------------------------------------------------
// export warnings (UI-REVIEW 缺陷 10)
// ---------------------------------------------------------------------------

#[test]
fn export_warns_on_generated_but_unpicked_targets() {
    let sb = Sandbox::new("exportwarn");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "warn", "--dir"]).arg(&dir).output().unwrap();
    sb.rudder()
        .args(["page", "add", "dashboard", "--brief", "x"])
        .arg("--project")
        .arg(&dir)
        .output()
        .unwrap();
    // Generated but never picked (page + board).
    make_fake_png(&dir.join("pages/dashboard/candidates/0001.png"), 1);
    make_fake_png(&dir.join("board/candidates/0001.png"), 2);

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
    assert_eq!(exit_code(&out), 0, "warnings are non-fatal; stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    let warnings = envelope["data"]["warnings"].as_array().expect("data.warnings array");
    assert_eq!(warnings.len(), 2, "board + page warnings: {warnings:?}");
    let stderr = stderr_text(&out);
    assert!(stderr.contains("warning:"), "warnings echoed on stderr: {stderr}");
    assert!(!out_dir.join("pages/dashboard/current.png").exists());
    assert_eq!(envelope["data"]["files"], 0);
}

// ---------------------------------------------------------------------------
// template protocol: templates list/show + --template + --prompt-file (PRD §0)
// ---------------------------------------------------------------------------

#[test]
fn templates_list_and_show_expose_fill_guides() {
    let sb = Sandbox::new("tpl-list");

    // `templates list --json`: the manifest with attribution + 5 templates.
    let out = sb.rudder().args(["templates", "list", "--json"]).output().expect("templates list");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["ok"], true);
    let templates = envelope["data"]["templates"].as_array().expect("templates array");
    assert_eq!(templates.len(), 5);
    let ids: Vec<&str> = templates.iter().filter_map(|t| t["id"].as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "board-design-system",
            "page-ui-standard",
            "page-landing-sections",
            "component-sheet-grid",
            "brand-identity-lite",
        ]
    );
    assert_eq!(envelope["data"]["attribution"]["license"], "MIT");
    assert!(
        envelope["data"]["attribution"]["url"]
            .as_str()
            .unwrap_or_default()
            .contains("awesome-gpt-image-2"),
        "attribution must cite the source"
    );
    assert!(envelope["data"]["slotVocabulary"].is_object(), "slot vocabulary for agents");
    assert!(!envelope["data"]["fillProtocol"].as_str().unwrap_or_default().is_empty());

    // `templates show <id> --json`: skeleton + fillGuide, no project needed.
    let out = sb
        .rudder()
        .args(["templates", "show", "page-landing-sections", "--json"])
        .output()
        .expect("templates show");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["id"], "page-landing-sections");
    assert_eq!(envelope["data"]["appliesTo"], "page");
    let skeleton = envelope["data"]["skeleton"].as_str().expect("skeleton string");
    assert!(skeleton.contains("{anchor.reference}"), "skeleton keeps slot placeholders");
    assert!(skeleton.contains("{page.brief}"));
    let guide = envelope["data"]["fillGuide"].as_object().expect("fillGuide object");
    assert!(guide.contains_key("howTo"));
    let guide_slots = guide["slots"].as_array().expect("guide slots");
    assert!(!guide_slots.is_empty());
    let first = &guide_slots[0];
    assert!(first["what"].is_string(), "per-slot 'what'");
    assert!(first["commonMistakes"].is_array(), "per-slot commonMistakes");

    // Human mode: skeleton + guide on stderr, summary on stdout.
    let out = sb.rudder().args(["templates", "show", "page-ui-standard"]).output().expect("show human");
    assert_eq!(exit_code(&out), 0);
    let stderr = stderr_text(&out);
    assert!(stderr.contains("skeleton"), "{stderr}");
    assert!(stderr.contains("fillGuide"), "{stderr}");
    assert!(stderr.contains("好例子"), "{stderr}");

    // Unknown id → exit 1 with a templates-list hint.
    let out = sb
        .rudder()
        .args(["templates", "show", "ghost", "--json"])
        .output()
        .expect("show unknown");
    assert_eq!(exit_code(&out), 1);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "INVALID_ARG");
    assert!(
        envelope["error"]["hint"].as_str().unwrap_or_default().contains("templates list"),
        "hint points at templates list"
    );
}

#[test]
fn prompt_file_full_chain_dry_run_and_validation() {
    let sb = Sandbox::new("promptfile");
    let dir = sb.project_dir("proj");
    sb.rudder().args(["init", "pf", "--dir"]).arg(&dir).output().expect("init");

    let prompt_path = sb.root.join("final-page.prompt");
    fs::write(&prompt_path, "总板风格沿用：海军蓝+黄铜。\n页面：dashboard，四张 KPI 卡 + 折线图。\n").expect("prompt file");

    // Full chain dry-run: the file content IS the prompt; source=agent-file;
    // the engine injected nothing.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "1", "--prompt-file"])
        .arg(&prompt_path)
        .arg("--project")
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate --prompt-file");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["source"], "agent-file");
    assert!(envelope["data"]["templateId"].is_null(), "no declared template");
    let expected = fs::read_to_string(&prompt_path).unwrap();
    assert_eq!(
        envelope["data"]["plan"]["params"]["prompt"].as_str().unwrap_or_default().trim(),
        expected.trim(),
        "file content passes through verbatim"
    );
    assert!(
        !envelope["data"]["plan"]["params"]["prompt"].as_str().unwrap().contains("Constraints:"),
        "agent path injects nothing"
    );

    // --template alongside --prompt-file: recorded only, prompt untouched.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "1", "--prompt-file"])
        .arg(&prompt_path)
        .args(["--template", "board-design-system", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate --prompt-file --template");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["source"], "agent-file");
    assert_eq!(envelope["data"]["templateId"], "board-design-system");
    assert!(
        !envelope["data"]["plan"]["params"]["prompt"].as_str().unwrap().contains("COLOR PALETTE"),
        "skeleton must not be assembled on the agent path"
    );

    // Missing file → exit 1 with hint.
    let out = sb
        .rudder()
        .args(["board", "generate", "--prompt-file", "/nonexistent/prompt.txt", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("missing prompt file");
    assert_eq!(exit_code(&out), 1, "argument errors exit 1");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "INVALID_ARG");
    assert!(envelope["error"]["message"].as_str().unwrap_or_default().contains("prompt file"));

    // Empty file → exit 1.
    let empty_path = sb.root.join("empty.prompt");
    fs::write(&empty_path, "  \n").unwrap();
    let out = sb
        .rudder()
        .args(["board", "generate", "--prompt-file"])
        .arg(&empty_path)
        .arg("--project")
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("empty prompt file");
    assert_eq!(exit_code(&out), 1);
    let envelope = parse_envelope(&out);
    assert!(envelope["error"]["message"].as_str().unwrap_or_default().contains("empty"));

    // Unknown template id → exit 1 + hint (independent of prompt-file).
    let out = sb
        .rudder()
        .args(["board", "generate", "--template", "ghost", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("unknown template");
    assert_eq!(exit_code(&out), 1);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["error"]["code"], "INVALID_ARG");
    assert!(
        envelope["error"]["hint"].as_str().unwrap_or_default().contains("templates list"),
        "hint: {envelope}"
    );
}

#[test]
fn template_selection_and_negative_hints_land_in_the_prompt() {
    let sb = Sandbox::new("tpl-prompt");
    let dir = sb.project_dir("proj");
    sb.rudder()
        .args(["init", "模板项目", "--dir"])
        .arg(&dir)
        .args(["--brief", "海军蓝 SaaS"])
        .output()
        .expect("init");

    // Default board skeleton via the engine path, with constraints injected.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "1", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate default");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["source"], "engine");
    assert_eq!(envelope["data"]["templateId"], "board-design-system");
    let prompt = envelope["data"]["plan"]["params"]["prompt"].as_str().unwrap();
    assert!(prompt.contains("COLOR PALETTE"));
    assert!(prompt.contains("Constraints:"));
    assert!(prompt.contains("- board-cohesion:"));
    assert!(!prompt.contains("- consistency-first:"));

    // project update: set the default template + append negative hints.
    let out = sb
        .rudder()
        .args(["project", "update", "--template", "brand-identity-lite", "--negative-hint", "不要通用放大镜图标", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("project update");
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_text(&out));
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["templateId"], "brand-identity-lite");
    assert_eq!(envelope["data"]["negativeHints"], serde_json::json!(["不要通用放大镜图标"]));

    // The project default template now drives board prompts; negative hints
    // ride the explicit-negatives constraint row.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "1", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate with project default");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["templateId"], "brand-identity-lite", "project.templateId wins over builtin");
    let prompt = envelope["data"]["plan"]["params"]["prompt"].as_str().unwrap();
    assert!(prompt.contains("pure white background"), "{prompt}");
    assert!(prompt.contains("NEVER DO"));
    assert!(
        prompt.contains("additionally forbidden per project: 不要通用放大镜图标"),
        "{prompt}"
    );

    // Explicit --template beats the project default.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "1", "--template", "board-design-system", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("board generate explicit template");
    let envelope = parse_envelope(&out);
    assert_eq!(envelope["data"]["templateId"], "board-design-system");
    let prompt = envelope["data"]["plan"]["params"]["prompt"].as_str().unwrap();
    assert!(prompt.contains("COLOR PALETTE"));

    // Clear the project default; clear negative hints.
    let out = sb
        .rudder()
        .args(["project", "update", "--clear-template", "--clear-negative-hints", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("project update clear");
    assert_eq!(exit_code(&out), 0);
    let envelope = parse_envelope(&out);
    assert!(envelope["data"]["templateId"].is_null());
    assert_eq!(envelope["data"]["negativeHints"], serde_json::json!([]));

    // Template/kind mismatch exits 1.
    let out = sb
        .rudder()
        .args(["board", "generate", "--n", "1", "--template", "page-ui-standard", "--project"])
        .arg(&dir)
        .arg("--json")
        .output()
        .expect("kind mismatch");
    assert_eq!(exit_code(&out), 1);
    let envelope = parse_envelope(&out);
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("applies to `page`"),
        "{envelope}"
    );
}
