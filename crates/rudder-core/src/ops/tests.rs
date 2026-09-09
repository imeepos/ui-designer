//! Tests for [`crate::ops`]: state-machine rules, defaults, dry-run plans
//! and the real persistence path — all against a dry client or a tiny
//! in-process mock server (see [`crate::image::tests`] for the server).

use super::*;
use crate::image::{ImageClient, MODEL};
use std::sync::Once;
use std::time::Duration;

/// Redirect `~/Rudder/config.json` writes into a per-run temp directory so
/// tests never touch the developer's real config.
fn ensure_test_home() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!("rudder-test-home-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create test home");
        // Safety: single-threaded init via Once; other tests only read the
        // override after it is set.
        std::env::set_var("RUDDER_HOME", &dir);
    });
}

fn tmp(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rudder-ops-{tag}-{}", uuid::Uuid::new_v4()))
}

fn dry_client() -> ImageClient {
    ImageClient::new(
        "http://127.0.0.1:9",
        Some("k".into()),
        true,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .unwrap()
}

fn seeded_project(tag: &str) -> PathBuf {
    ensure_test_home();
    let root = tmp(tag);
    init_project(&root, "样例", "web", Some("海军蓝+黄铜")).unwrap();
    root
}

#[test]
fn init_creates_project_and_defaults() {
    ensure_test_home();
    let root = seeded_project("init");
    let project = store::load_project(&root).unwrap();
    assert_eq!(project.name, "样例");
    assert_eq!(project.canvas_size.to_api_string(), "1536x1024");
    assert_eq!(project.canvas_size.preset, Some(Preset::Web));
    assert_eq!(project.style_brief, "海军蓝+黄铜");
    assert!(project.anchor.is_none());
    // Layout ready for candidates.
    assert!(root.join("board/candidates").is_dir());
    assert!(root.join("refs").is_dir());
    // last-project tracking landed in the isolated test home.
    assert!(Config::path().expect("config path").is_file());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn init_rejects_bad_size_and_empty_name() {
    ensure_test_home();
    let root = tmp("badinit");
    let err = init_project(&root, "x", "1234x5678", None).unwrap_err();
    assert_eq!(err.code(), "SIZE_INVALID");
    assert!(!root.join("project.json").exists(), "no project.json on failed init");
    let err = init_project(&root, "  ", "web", None).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn board_generate_dry_run_makes_plan_only() {
    let root = seeded_project("boarddry");
    let client = dry_client();
    let report = generate(
        &root,
        &Target::Board,
        GenerateOptions { n: Some(2), quality: Some("low".into()), dry_run: true, ..Default::default() },
        &client,
    )
    .await
    .unwrap();
    assert!(report.dry_run);
    assert!(report.candidates.is_empty());
    assert_eq!(report.plan.endpoint, "generations", "board without refs uses generations");
    assert_eq!(report.plan.params["n"], 2);
    assert_eq!(report.plan.params["quality"], "low");
    assert_eq!(report.plan.params["thinking"], "medium");
    // Nothing written: no candidate files, project untouched.
    assert_eq!(store::count_candidates(&root.join("board")).unwrap(), 0);
    let project = store::load_project(&root).unwrap();
    assert!(project.board_generations.is_empty());
    assert!(project.prompt_log.is_empty());
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn page_generate_without_anchor_is_state_error() {
    let root = seeded_project("noanchor");
    page_add(&root, "dashboard", "四张指标卡").unwrap();
    let client = dry_client();
    // Even a dry-run keeps the contract: exit 3 without an anchor.
    let err = generate(
        &root,
        &Target::Page("dashboard".into()),
        GenerateOptions { dry_run: true, ..Default::default() },
        &client,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "NO_ANCHOR");
    assert_eq!(err.exit_code(), 3);

    // e2e-style planning may assume the anchor (dry-run only).
    let plan = generate(
        &root,
        &Target::Page("dashboard".into()),
        GenerateOptions { dry_run: true, assume_anchor: true, ..Default::default() },
        &client,
    )
    .await
    .unwrap();
    assert!(plan.dry_run);
    assert_eq!(plan.plan.endpoint, "edits");
    assert_eq!(plan.plan.images[0].role, "anchor");
    assert!(plan.plan.images[0].missing, "anchor file does not exist yet");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn page_add_validates_slug_and_duplicates() {
    let root = seeded_project("pageadd");
    page_add(&root, "dashboard", "主面板").unwrap();
    let err = page_add(&root, "dashboard", "again").unwrap_err();
    assert_eq!(err.code(), "ALREADY_EXISTS");
    assert_eq!(page_add(&root, "Bad Slug", "x").unwrap_err().code(), "INVALID_ARG");
    assert_eq!(page_add(&root, "ok-slug", "  ").unwrap_err().code(), "INVALID_ARG");
    let project = store::load_project(&root).unwrap();
    assert_eq!(project.pages.len(), 1);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn component_add_validates_type_and_duplicates() {
    let root = seeded_project("compadd");
    component_add(&root, "button-set", "buttons", "三态").unwrap();
    assert_eq!(
        component_add(&root, "button-set", "buttons", "x").unwrap_err().code(),
        "ALREADY_EXISTS"
    );
    assert_eq!(
        component_add(&root, "c2", "", "x").unwrap_err().code(),
        "INVALID_ARG"
    );
    assert_eq!(
        component_add(&root, "c2", "buttons", "").unwrap_err().code(),
        "INVALID_ARG"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn board_pick_requires_candidate_then_sets_anchor() {
    let root = seeded_project("pick");
    let err = board_pick(&root, "0001").unwrap_err();
    assert_eq!(err.code(), "NOT_FOUND");

    // Simulate a real generation's disk artifacts.
    std::fs::write(root.join("board/candidates/0001.png"), b"img1").unwrap();
    // Lineage must exist for the anchor prompt to be recorded.
    let mut project = store::load_project(&root).unwrap();
    project.board_generations.push(GenRecord {
        at: store::now_rfc3339(),
        endpoint: "generations".into(),
        prompt: "BOARD".into(),
        params: GenParams {
            model: MODEL.into(),
            size: "1536x1024".into(),
            quality: "low".into(),
            n: 1,
            seed: Some(3),
            thinking: Some("medium".into()),
        },
        candidate_ids: vec!["0001".into()],
        source: None,
        template_id: None,
    });
    save_project(&root, &project).unwrap();

    let anchor_file = board_pick(&root, "0001").unwrap();
    assert_eq!(anchor_file, "board/anchor.png");
    assert_eq!(std::fs::read(root.join("board/anchor.png")).unwrap(), b"img1");
    let project = store::load_project(&root).unwrap();
    let anchor = project.anchor.expect("anchor set");
    assert_eq!(anchor.candidate_id, "0001");
    assert_eq!(anchor.prompt, "BOARD");
    assert_eq!(anchor.seed, Some(3));

    // Now page generate dry-run includes the existing anchor.
    page_add(&root, "dash", "x").unwrap();
    let client = dry_client();
    let plan = generate(
        &root,
        &Target::Page("dash".into()),
        GenerateOptions { dry_run: true, ..Default::default() },
        &client,
    )
    .await
    .unwrap();
    assert!(!plan.plan.images.is_empty());
    assert!(!plan.plan.images[0].missing, "anchor.png exists on disk");
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn status_reports_candidates_and_anchor() {
    let root = seeded_project("status");
    page_add(&root, "dashboard", "x").unwrap();
    component_add(&root, "button-set", "buttons", "y").unwrap();
    let report = status(&root).unwrap();
    assert!(report.anchor.is_none());
    assert_eq!(report.pages.len(), 1);
    assert_eq!(report.pages[0].name, "dashboard");
    assert_eq!(report.pages[0].candidates, 0);
    assert!(!report.pages[0].has_current);
    assert_eq!(report.components.len(), 1);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn resolve_generation_params_validates_enums() {
    let config = Config::default();
    let canvas = CanvasSize::new(1536, 1024, Some(Preset::Web));
    let opts = GenerateOptions {
        quality: Some("ultra".into()),
        ..Default::default()
    };
    let err = resolve_generation_params(&opts, &config, "p".into(), &canvas, 4).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");

    let opts = GenerateOptions { n: Some(5), ..Default::default() };
    let err = resolve_generation_params(&opts, &config, "p".into(), &canvas, 1).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");

    let opts = GenerateOptions::default();
    let params = resolve_generation_params(&opts, &config, "p".into(), &canvas, 4).unwrap();
    assert_eq!(params.quality, "low", "documented default (UI-REVIEW #5)");
    assert_eq!(params.thinking.as_deref(), Some("medium"));
    assert_eq!(params.n, 4);
}

#[test]
fn generation_params_always_carry_a_seed() {
    // UI-REVIEW 缺陷 1: an omitted --seed auto-fills with a recorded random
    // seed, so every batch stays reproducible.
    let config = Config::default();
    let canvas = CanvasSize::new(1536, 1024, Some(Preset::Web));

    let params = resolve_generation_params(&GenerateOptions::default(), &config, "p".into(), &canvas, 1).unwrap();
    let auto_seed = params.seed.expect("auto seed must be recorded");
    assert_ne!(auto_seed, 0, "vanishingly unlikely; guards against all-zero fallback");

    let other = resolve_generation_params(&GenerateOptions::default(), &config, "p".into(), &canvas, 1).unwrap();
    assert_ne!(other.seed, Some(auto_seed), "two auto seeds must differ (probabilistic)");

    let explicit = resolve_generation_params(
        &GenerateOptions { seed: Some(42), ..Default::default() },
        &config,
        "p".into(),
        &canvas,
        1,
    )
    .unwrap();
    assert_eq!(explicit.seed, Some(42), "explicit --seed wins");
}

#[test]
fn project_page_component_update_amend_metadata() {
    let root = seeded_project("update");
    page_add(&root, "dashboard", "旧简报").unwrap();
    component_add(&root, "button-set", "buttons", "旧简报").unwrap();

    // project: rename + style brief swap.
    let project = project_update(
        &root,
        ProjectUpdate {
            name: Some("新名字".into()),
            style_brief: Some("黄铜+深蓝，圆角 4px".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(project.name, "新名字");
    assert_eq!(project.style_brief, "黄铜+深蓝，圆角 4px");

    // page brief swap.
    let page = page_update(&root, "dashboard", "新布局简报").unwrap();
    assert_eq!(page.brief, "新布局简报");
    let stored = load_project(&root).unwrap();
    assert_eq!(stored.page("dashboard").unwrap().brief, "新布局简报");

    // component type + brief swap.
    let comp = component_update(&root, "button-set", Some("cards"), Some("新简报")).unwrap();
    assert_eq!(comp.kind, "cards");
    assert_eq!(comp.brief, "新简报");

    // update events land in the prompt log (PROMPTS.md markers).
    let stored = load_project(&root).unwrap();
    let kinds: Vec<_> = stored.prompt_log.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"project-update"));
    assert!(kinds.contains(&"page-update"));
    assert!(kinds.contains(&"component-update"));

    // error paths.
    assert_eq!(
        project_update(&root, ProjectUpdate::default()).unwrap_err().code(),
        "INVALID_ARG"
    );
    assert_eq!(page_update(&root, "dashboard", "  ").unwrap_err().code(), "INVALID_ARG");
    assert_eq!(page_update(&root, "missing", "x").unwrap_err().code(), "NOT_FOUND");
    assert_eq!(
        component_update(&root, "button-set", None, None).unwrap_err().code(),
        "INVALID_ARG"
    );
    assert_eq!(
        component_update(&root, "missing", Some("cards"), None).unwrap_err().code(),
        "NOT_FOUND"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn board_generate_report_candidates_carry_seed_and_shape() {
    let root = seeded_project("seedreport");
    let client = dry_client();
    // Dry-run keeps candidates empty but the plan must show the seed.
    let plan = generate(
        &root,
        &Target::Board,
        GenerateOptions { n: Some(1), quality: Some("low".into()), dry_run: true, ..Default::default() },
        &client,
    )
    .await
    .unwrap();
    assert!(plan.plan.params["seed"].is_u64(), "plan params record the seed: {}", plan.plan.params);
    std::fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// template resolution + --prompt-file (模板协议，PRD §0)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn template_resolution_explicit_then_project_then_default() {
    let root = seeded_project("templateres");
    page_add(&root, "landing", "导航 4 项 + hero 双按钮 + 3 特性段").unwrap();
    let client = dry_client();
    let base = GenerateOptions { dry_run: true, assume_anchor: true, ..Default::default() };

    // 1. builtin default skeleton.
    let report = generate(&root, &Target::Page("landing".into()), base.clone(), &client)
        .await
        .unwrap();
    assert_eq!(report.source, "engine");
    assert_eq!(report.template_id.as_deref(), Some("page-ui-standard"));
    assert!(report.prompt.contains("five passes"), "{}", report.prompt);
    assert!(report.prompt.contains("Constraints:"), "engine path injects constraints");

    // 2. project.templateId wins over the builtin default.
    let mut project = store::load_project(&root).unwrap();
    project.template_id = Some("page-landing-sections".into());
    store::save_project(&root, &project).unwrap();
    let report = generate(&root, &Target::Page("landing".into()), base.clone(), &client)
        .await
        .unwrap();
    assert_eq!(report.template_id.as_deref(), Some("page-landing-sections"));
    assert!(report.prompt.contains("Section contract:"), "{}", report.prompt);

    // 3. explicit --template wins over project.templateId.
    let explicit = GenerateOptions {
        template: Some("page-ui-standard".into()),
        ..base.clone()
    };
    let report = generate(&root, &Target::Page("landing".into()), explicit, &client)
        .await
        .unwrap();
    assert_eq!(report.template_id.as_deref(), Some("page-ui-standard"));
    assert!(report.prompt.contains("five passes"));
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn unknown_or_mismatched_template_is_exit1_with_hint() {
    let root = seeded_project("badtemplate");
    page_add(&root, "dash", "简报").unwrap();
    let client = dry_client();

    // Unknown id → INVALID_ARG (exit 1) with a templates-list hint.
    let err = generate(
        &root,
        &Target::Page("dash".into()),
        GenerateOptions {
            template: Some("nope".into()),
            dry_run: true,
            assume_anchor: true,
            ..Default::default()
        },
        &client,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    assert_eq!(err.exit_code(), 1);
    assert!(err.hint().contains("templates list"), "hint: {}", err.hint());

    // Board template on a page target → kind mismatch error.
    let err = generate(
        &root,
        &Target::Page("dash".into()),
        GenerateOptions {
            template: Some("brand-identity-lite".into()),
            dry_run: true,
            assume_anchor: true,
            ..Default::default()
        },
        &client,
    )
    .await
    .unwrap_err();
    assert_eq!(err.exit_code(), 1);
    assert!(err.to_string().contains("applies to `board`"), "{err}");
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn prompt_file_full_chain_dry_run() {
    let root = seeded_project("promptfile");
    page_add(&root, "dashboard", "四张 KPI 卡").unwrap();
    let client = dry_client();
    let dir = tmp("promptfile-io");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("page.prompt");
    std::fs::write(&path, "代理写好的最终提示词\n第二行 Labels: 不会被引擎改写\n").unwrap();

    // Full chain: file read → source recorded → plan carries the text as-is.
    let report = generate(
        &root,
        &Target::Page("dashboard".into()),
        GenerateOptions {
            prompt_file: Some(path.clone()),
            dry_run: true,
            assume_anchor: true,
            ..Default::default()
        },
        &client,
    )
    .await
    .unwrap();
    assert_eq!(report.source, "agent-file", "recorded prompt source");
    assert_eq!(report.template_id, None, "no declared skeleton without --template");
    assert_eq!(report.prompt, "代理写好的最终提示词\n第二行 Labels: 不会被引擎改写", "trimmed copy of the file");
    assert_eq!(report.plan.params["prompt"], report.prompt, "the file content IS the request prompt");
    assert!(!report.prompt.contains("Constraints:"), "nothing is injected on the agent path");
    // The anchor still rides as Image 1.
    assert_eq!(report.plan.endpoint, "edits");
    assert_eq!(report.plan.images[0].role, "anchor");

    // --template alongside --prompt-file is recorded, never assembled.
    let report = generate(
        &root,
        &Target::Page("dashboard".into()),
        GenerateOptions {
            prompt_file: Some(path.clone()),
            template: Some("page-ui-standard".into()),
            dry_run: true,
            assume_anchor: true,
            ..Default::default()
        },
        &client,
    )
    .await
    .unwrap();
    assert_eq!(report.template_id.as_deref(), Some("page-ui-standard"));
    assert_eq!(report.source, "agent-file");
    assert!(!report.prompt.contains("five passes"), "skeleton must NOT be assembled");
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn prompt_file_validation_errors_exit1() {
    let dir = tmp("promptfile-bad");
    std::fs::create_dir_all(&dir).unwrap();

    // Missing file.
    let err = read_prompt_file(&dir.join("missing.prompt")).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    assert_eq!(err.exit_code(), 1);
    assert!(err.to_string().contains("does not exist"), "{err}");

    // Empty file.
    let empty = dir.join("empty.prompt");
    std::fs::write(&empty, "   \n  ").unwrap();
    let err = read_prompt_file(&empty).unwrap_err();
    assert!(err.to_string().contains("empty"), "{err}");

    // Not UTF-8.
    let binary = dir.join("binary.prompt");
    std::fs::write(&binary, [0xFF, 0xFE, 0x00, b'b']).unwrap();
    let err = read_prompt_file(&binary).unwrap_err();
    assert!(err.to_string().contains("UTF-8"), "{err}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn project_update_template_and_negative_hints() {
    let root = seeded_project("projtemplate");

    // Set the project-default template (validated to exist).
    let project = project_update(
        &root,
        ProjectUpdate {
            template: Some("brand-identity-lite".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(project.template_id.as_deref(), Some("brand-identity-lite"));

    // Unknown template id → INVALID_ARG.
    let err = project_update(
        &root,
        ProjectUpdate { template: Some("ghost".into()), ..Default::default() },
    )
    .unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");

    // Append negative hints (deduplicated), then clear them.
    let project = project_update(
        &root,
        ProjectUpdate {
            add_negative_hints: vec!["不要通用放大镜图标".into(), " no dark mode ".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(project.negative_hints, vec!["不要通用放大镜图标", "no dark mode"]);
    let project = project_update(
        &root,
        ProjectUpdate {
            add_negative_hints: vec!["不要通用放大镜图标".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(project.negative_hints.len(), 2, "duplicate hint not appended");
    let project = project_update(
        &root,
        ProjectUpdate { clear_negative_hints: true, ..Default::default() },
    )
    .unwrap();
    assert!(project.negative_hints.is_empty());

    // Clear the project-default template.
    let project = project_update(
        &root,
        ProjectUpdate { clear_template: true, ..Default::default() },
    )
    .unwrap();
    assert_eq!(project.template_id, None);

    // Conflicts and empty hints are argument errors.
    assert_eq!(
        project_update(
            &root,
            ProjectUpdate {
                template: Some("page-ui-standard".into()),
                clear_template: true,
                ..Default::default()
            },
        )
        .unwrap_err()
        .code(),
        "INVALID_ARG"
    );
    assert_eq!(
        project_update(
            &root,
            ProjectUpdate {
                add_negative_hints: vec!["x".into()],
                clear_negative_hints: true,
                ..Default::default()
            },
        )
        .unwrap_err()
        .code(),
        "INVALID_ARG"
    );
    assert_eq!(
        project_update(
            &root,
            ProjectUpdate { add_negative_hints: vec!["  ".into()], ..Default::default() },
        )
        .unwrap_err()
        .code(),
        "INVALID_ARG"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn old_project_json_without_new_fields_loads_unchanged() {
    // Backward compatibility: pre-template project.json files (no
    // templateId / negativeHints / source keys) load with serde defaults.
    let root = tmp("backward");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join(store::PROJECT_FILE),
        r#"{
            "id": "legacy-id",
            "name": "旧项目",
            "canvasSize": { "w": 1536, "h": 1024, "preset": "web" },
            "created_at": "2026-01-01T00:00:00.000Z"
        }"#,
    )
    .unwrap();
    let project = load_project(&root).unwrap();
    assert_eq!(project.name, "旧项目");
    assert_eq!(project.template_id, None);
    assert!(project.negative_hints.is_empty());
    std::fs::remove_dir_all(&root).ok();
}

/// Full-chain REAL run (mock server, no network): agent prompt file →
/// anchor edits call → candidates + GenRecord/promptLog recorded with
/// `source: "agent-file"` and the declared template id.
#[tokio::test]
async fn prompt_file_real_run_persists_agent_file_lineage() {
    use crate::test_support::{b64_response, MockServer};

    ensure_test_home();

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 1, 2, 3, 4];
    let server = MockServer::start(move |_req, _i| (200, b64_response(&[PNG])));
    let client = ImageClient::new(
        server.url(),
        Some("k".into()),
        false,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .unwrap();

    let root = seeded_project("agentfile-real");
    page_add(&root, "dashboard", "四张 KPI 卡").unwrap();

    // Anchor exists (as after a real board pick).
    std::fs::create_dir_all(root.join("board")).unwrap();
    std::fs::write(root.join("board/anchor.png"), PNG).unwrap();
    let mut project = load_project(&root).unwrap();
    project.anchor = Some(store::Anchor {
        candidate_id: "0001".into(),
        prompt: String::new(),
        seed: Some(1),
        created_at: store::now_rfc3339(),
    });
    save_project(&root, &project).unwrap();

    let dir = tmp("agentfile-real-io");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("page.prompt");
    std::fs::write(&path, "代理写好的最终提示词（真实链路）").unwrap();

    let report = generate(
        &root,
        &Target::Page("dashboard".into()),
        GenerateOptions {
            prompt_file: Some(path.clone()),
            template: Some("page-ui-standard".into()),
            n: Some(1),
            quality: Some("low".into()),
            ..Default::default()
        },
        &client,
    )
    .await
    .unwrap();
    assert!(!report.dry_run);
    assert_eq!(report.source, "agent-file");
    assert_eq!(report.candidates.len(), 1);
    assert!(root.join("pages/dashboard/candidates/0001.png").is_file());

    // One multipart edits call carried the file text verbatim.
    let reqs = server.recorded();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].path, "/v1/images/edits");
    let body = reqs[0].body_str();
    assert!(body.contains("代理写好的最终提示词（真实链路）"), "{body}");

    // Lineage: GenRecord on the page + promptLog both record the source
    // and the declared template id.
    let project = load_project(&root).unwrap();
    let record = &project.page("dashboard").unwrap().generations[0];
    assert_eq!(record.source.as_deref(), Some("agent-file"));
    assert_eq!(record.template_id.as_deref(), Some("page-ui-standard"));
    assert_eq!(record.prompt, "代理写好的最终提示词（真实链路）");
    let log = project.prompt_log.last().unwrap();
    assert_eq!(log.source.as_deref(), Some("agent-file"));
    assert_eq!(log.template_id.as_deref(), Some("page-ui-standard"));

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&dir).ok();
}

/// Engine-path real run records `source: "engine"` + the resolved template.
#[tokio::test]
async fn engine_real_run_persists_engine_lineage() {
    use crate::test_support::{b64_response, MockServer};

    ensure_test_home();
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 9, 8, 7, 6];
    let server = MockServer::start(move |_req, _i| (200, b64_response(&[PNG])));
    let client = ImageClient::new(
        server.url(),
        Some("k".into()),
        false,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .unwrap();

    let root = seeded_project("engine-real");
    let report = generate(
        &root,
        &Target::Board,
        GenerateOptions { n: Some(1), quality: Some("low".into()), ..Default::default() },
        &client,
    )
    .await
    .unwrap();
    assert_eq!(report.source, "engine");
    assert_eq!(report.template_id.as_deref(), Some("board-design-system"));

    let project = load_project(&root).unwrap();
    let record = &project.board_generations[0];
    assert_eq!(record.source.as_deref(), Some("engine"));
    assert_eq!(record.template_id.as_deref(), Some("board-design-system"));
    assert!(record.prompt.contains("Purpose: a UI design system board"));
    assert!(record.prompt.contains("Constraints:"));
    std::fs::remove_dir_all(&root).ok();
}
