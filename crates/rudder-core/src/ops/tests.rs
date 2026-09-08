//! Tests for [`crate::ops`]: state-machine rules, defaults, dry-run plans
//! and the real persistence path — all against a dry client or a tiny
//! in-process mock server (see [`crate::image::tests`] for the server).

use super::*;
use crate::image::ImageClient;
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
    assert_eq!(params.quality, "high", "documented default");
    assert_eq!(params.thinking.as_deref(), Some("medium"));
    assert_eq!(params.n, 4);
}
