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
    init_project(Some(&root), "样例", "web", Some("海军蓝+黄铜")).unwrap();
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
fn init_without_dir_lands_in_scan_root_and_skips_registry() {
    ensure_test_home();
    let dir = init_project(None, "默认落点", "web", None).unwrap();
    let canonical = dir.canonicalize().unwrap();
    let scan_root = crate::registry::projects_root().unwrap();
    assert!(
        canonical.starts_with(scan_root.canonicalize().unwrap()),
        "default init must live under the shared scan root, got {dir:?}"
    );
    // Under the scan root the catalog already sees it: never registered.
    assert!(
        !crate::registry::list_valid().unwrap().contains(&canonical),
        "scan-root projects must not be registered"
    );
    // last_project pointer is canonical-absolute (whatever the latest
    // parallel init wrote — the invariant is "absolute", not the value).
    let last = Config::load().last_project.expect("last_project set");
    assert!(last.is_absolute(), "last_project must be absolute: {last:?}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn init_with_external_dir_registers_for_catalog() {
    ensure_test_home();
    let root = tmp("external");
    let dir = init_project(Some(&root), "外部目录", "web", None).unwrap();
    let canonical = dir.canonicalize().unwrap();
    // Outside the scan root → registered so the desktop catalog merges it.
    assert!(
        crate::registry::list_valid().unwrap().contains(&canonical),
        "external init must be registered"
    );
    let last = Config::load().last_project.expect("last_project set");
    assert!(last.is_absolute(), "last_project must be absolute: {last:?}");
    std::fs::remove_dir_all(&dir).ok();
    // Self-heal: after deletion the entry drops out of the valid list.
    assert!(!crate::registry::list_valid().unwrap().contains(&canonical));
}

#[test]
fn init_rejects_bad_size_and_empty_name() {
    ensure_test_home();
    let root = tmp("badinit");
    let err = init_project(Some(&root), "x", "1234x5678", None).unwrap_err();
    assert_eq!(err.code(), "SIZE_INVALID");
    assert!(!root.join("project.json").exists(), "no project.json on failed init");
    let err = init_project(Some(&root), "  ", "web", None).unwrap_err();
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

// ---------------------------------------------------------------------------
// record_generated (C2: 前端 SDK 生图 → Rust 只负责落盘)
// ---------------------------------------------------------------------------

/// The record-only stage must leave exactly the same on-disk state as the
/// full `generate` flow (candidates, lineage, prompt log) — the desktop
/// frontend calls upstream itself, Rust only persists the result.
#[tokio::test]
async fn record_generated_persists_the_same_state_as_full_generate() {
    use crate::test_support::{b64_response, MockServer};

    ensure_test_home();
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 7, 7, 7, 7];
    let server = MockServer::start(move |_req, _i| (200, b64_response(&[PNG])));
    let client = ImageClient::new(
        server.url(),
        Some("k".into()),
        false,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .unwrap();

    // Full flow (CLI path) on project A.
    let root_full = seeded_project("rec-parity-full");
    let full = generate(
        &root_full,
        &Target::Board,
        GenerateOptions { n: Some(1), quality: Some("low".into()), ..Default::default() },
        &client,
    )
    .await
    .unwrap();
    assert!(!full.dry_run);

    // Record-only path (desktop SDK path) on project B with the same
    // metadata, copied from the full-flow report.
    let root_rec = seeded_project("rec-parity-only");
    let batch = GeneratedBatch {
        target: Target::Board,
        endpoint: full.plan.endpoint.clone(),
        prompt: full.prompt.clone(),
        source: full.source.clone(),
        template_id: full.template_id.clone(),
        model: full.plan.params["model"].as_str().unwrap().to_string(),
        size: full.plan.params["size"].as_str().unwrap().to_string(),
        quality: full.plan.params["quality"].as_str().unwrap().to_string(),
        n: u32::try_from(full.plan.params["n"].as_u64().unwrap()).unwrap(),
        seed: Some(full.plan.params["seed"].as_u64().unwrap()),
        thinking: full.plan.params["thinking"].as_str().map(str::to_string),
        images: vec![PNG.to_vec()],
        plan: None,
    };
    let recorded = record_generated(&root_rec, batch).unwrap();

    // Same report shape and identical candidates (plan params carry the
    // same request description; only the URL provenance differs).
    assert!(!recorded.dry_run);
    assert_eq!(recorded.kind, full.kind);
    assert_eq!(recorded.target, full.target);
    assert_eq!(recorded.prompt, full.prompt);
    assert_eq!(recorded.source, full.source);
    assert_eq!(recorded.template_id, full.template_id);
    assert_eq!(recorded.candidates, full.candidates);
    assert_eq!(recorded.plan.endpoint, full.plan.endpoint);
    assert_eq!(recorded.plan.params, full.plan.params);

    // Identical project.json state (timestamps excluded).
    let mut a = load_project(&root_full).unwrap();
    let mut b = load_project(&root_rec).unwrap();
    for record in a.board_generations.iter_mut().chain(b.board_generations.iter_mut()) {
        record.at.clear();
    }
    assert_eq!(a.board_generations, b.board_generations);
    for entry in a.prompt_log.iter_mut().chain(b.prompt_log.iter_mut()) {
        entry.at.clear();
    }
    assert_eq!(a.prompt_log, b.prompt_log);

    // Identical bytes on disk.
    assert_eq!(
        std::fs::read(root_full.join("board/candidates/0001.png")).unwrap(),
        std::fs::read(root_rec.join("board/candidates/0001.png")).unwrap()
    );
    assert_eq!(store::count_candidates(&root_rec.join("board")).unwrap(), 1);

    std::fs::remove_dir_all(&root_full).ok();
    std::fs::remove_dir_all(&root_rec).ok();
}

/// Page targets: the record stage must update prompt/seed/updated_at and
/// append both the GenRecord and the prompt-log entry, like a real run.
#[test]
fn record_generated_updates_page_lineage_fields() {
    let root = seeded_project("rec-page");
    page_add(&root, "dashboard", "四张 KPI 卡").unwrap();
    let batch = GeneratedBatch {
        target: Target::Page("dashboard".into()),
        endpoint: "edits".into(),
        prompt: "代理写好的最终提示词".into(),
        source: prompt::SOURCE_AGENT_FILE.into(),
        template_id: Some("page-ui-standard".into()),
        model: "gpt-image-2".into(),
        size: "1536x1024".into(),
        quality: "low".into(),
        n: 1,
        seed: Some(7),
        thinking: Some("low".into()),
        images: vec![b"img".to_vec()],
        plan: None,
    };
    let report = record_generated(&root, batch).unwrap();
    assert!(!report.dry_run);
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].file, "pages/dashboard/candidates/0001.png");
    assert_eq!(report.candidates[0].seed, Some(7));
    assert!(root.join("pages/dashboard/candidates/0001.png").is_file());

    let project = load_project(&root).unwrap();
    let page = project.page("dashboard").unwrap();
    assert_eq!(page.prompt.as_deref(), Some("代理写好的最终提示词"));
    assert_eq!(page.seed, Some(7));
    assert_eq!(page.generations.len(), 1);
    let record = &page.generations[0];
    assert_eq!(record.endpoint, "edits");
    assert_eq!(record.prompt, "代理写好的最终提示词");
    assert_eq!(record.candidate_ids, vec!["0001".to_string()]);
    assert_eq!(record.source.as_deref(), Some("agent-file"));
    assert_eq!(record.template_id.as_deref(), Some("page-ui-standard"));
    assert_eq!(record.params.model, "gpt-image-2");
    assert_eq!(record.params.seed, Some(7));
    assert_eq!(record.params.thinking.as_deref(), Some("low"));

    let log = project.prompt_log.last().unwrap();
    assert_eq!(log.kind, "page");
    assert_eq!(log.target, "dashboard");
    assert_eq!(log.endpoint, "edits");
    assert_eq!(log.candidate_ids, vec!["0001".to_string()]);
    assert_eq!(log.source.as_deref(), Some("agent-file"));
    assert_eq!(log.template_id.as_deref(), Some("page-ui-standard"));
    std::fs::remove_dir_all(&root).ok();
}

/// Rejected batches (unknown endpoint, no images) must persist nothing.
#[test]
fn record_generated_rejects_unknown_endpoint_and_empty_images() {
    let root = seeded_project("rec-bad");
    let batch = |endpoint: &str, images: Vec<Vec<u8>>| GeneratedBatch {
        target: Target::Board,
        endpoint: endpoint.into(),
        prompt: "p".into(),
        source: "engine".into(),
        template_id: None,
        model: "gpt-image-2".into(),
        size: "1536x1024".into(),
        quality: "low".into(),
        n: 1,
        seed: Some(1),
        thinking: None,
        images,
        plan: None,
    };

    let err = record_generated(&root, batch("chat", vec![b"img".to_vec()])).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    let err = record_generated(&root, batch("generations", Vec::new())).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");

    // Nothing was persisted by the rejected calls.
    let project = load_project(&root).unwrap();
    assert!(project.board_generations.is_empty());
    assert!(project.prompt_log.is_empty());
    assert_eq!(store::count_candidates(&root.join("board")).unwrap(), 0);
    std::fs::remove_dir_all(&root).ok();
}
