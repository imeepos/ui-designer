//! Tests for [`crate::export`]: manifest lineage, PROMPTS.md and
//! DESIGN.template.md rendering, copy layout.

use super::*;
use crate::canvas::{CanvasSize, Preset};
use crate::image::{GenerateParams, MODEL};
use crate::store::{
    create_project, load_project, save_project, Anchor, GenRecord, GenParams, Page, Project,
    PromptLogEntry,
};

fn base_project() -> Project {
    Project::new(
        "e2e样例",
        CanvasSize::new(1536, 1024, Some(Preset::Web)),
        "远洋航运 SaaS",
        "海军蓝+黄铜点缀",
    )
}

fn gen_params(n: u32) -> GenParams {
    GenParams {
        model: MODEL.to_string(),
        size: "1536x1024".into(),
        quality: "low".into(),
        n,
        seed: Some(7),
        thinking: Some("medium".into()),
    }
}

fn record(prompt: &str, ids: &[&str], endpoint: &str) -> GenRecord {
    GenRecord {
        at: "2026-09-08T00:00:00.000Z".into(),
        endpoint: endpoint.into(),
        prompt: prompt.into(),
        params: gen_params(ids.len() as u32),
        candidate_ids: ids.iter().map(|s| s.to_string()).collect(),
    }
}

fn tmp(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rudder-export-{tag}-{}", uuid::Uuid::new_v4()))
}

/// Build a fully populated project on disk (anchor, one page, one component).
fn populated_project(tag: &str) -> PathBuf {
    let root = tmp(tag);
    let mut project = base_project();
    create_project(&root, &project).unwrap();

    // board candidates + anchor
    std::fs::write(root.join("board/candidates/0001.png"), vec![0xAA; 20480]).unwrap();
    std::fs::write(root.join("board/candidates/0002.png"), vec![0xBB; 20480]).unwrap();
    std::fs::copy(root.join("board/candidates/0001.png"), root.join("board/anchor.png")).unwrap();
    project.board_generations.push(record("BOARD PROMPT", &["0001", "0002"], "generations"));
    project.anchor = Some(Anchor {
        candidate_id: "0001".into(),
        prompt: "BOARD PROMPT".into(),
        seed: Some(7),
        created_at: store::now_rfc3339(),
    });

    // page with candidate + current
    let mut page = Page::new("dashboard".into(), "顶部指标卡x4".into());
    std::fs::create_dir_all(root.join("pages/dashboard/candidates")).unwrap();
    std::fs::write(root.join("pages/dashboard/candidates/0001.png"), vec![0xCC; 20480]).unwrap();
    std::fs::copy(root.join("pages/dashboard/candidates/0001.png"), root.join("pages/dashboard/current.png")).unwrap();
    page.prompt = Some("PAGE PROMPT".into());
    page.generations.push(record("PAGE PROMPT", &["0001"], "edits"));
    project.pages.push(page);

    // component with candidate + current
    let mut comp = crate::store::Component::new("button-set".into(), "buttons".into(), "主/次/幽灵".into());
    std::fs::create_dir_all(root.join("components/button-set/candidates")).unwrap();
    std::fs::write(root.join("components/button-set/candidates/0001.png"), vec![0xDD; 20480]).unwrap();
    std::fs::copy(
        root.join("components/button-set/candidates/0001.png"),
        root.join("components/button-set/current.png"),
    )
    .unwrap();
    comp.prompt = Some("COMP PROMPT".into());
    comp.generations.push(record("COMP PROMPT", &["0001"], "edits"));
    project.components.push(comp);

    project.prompt_log.push(PromptLogEntry {
        at: "2026-09-08T00:00:00.000Z".into(),
        kind: "board".into(),
        target: String::new(),
        endpoint: "generations".into(),
        prompt: "BOARD PROMPT".into(),
        params: gen_params(2),
        candidate_ids: vec!["0001".into(), "0002".into()],
        dry_run: false,
    });

    save_project(&root, &project).unwrap();
    root
}

#[test]
fn export_copies_layout_and_manifest_has_lineage() {
    let root = populated_project("copy");
    let out = tmp("out");
    let report = export_project(&root, &out).unwrap();
    assert!(report.files >= 5, "anchor + 2 board cands + page + component");

    for rel in [
        "board/anchor.png",
        "board/candidates/0001.png",
        "board/candidates/0002.png",
        "pages/dashboard/current.png",
        "components/button-set/current.png",
        "manifest.json",
        "PROMPTS.md",
        "DESIGN.template.md",
    ] {
        assert!(out.join(rel).is_file(), "{rel} must exist");
    }

    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["project"]["name"], "e2e样例");
    assert_eq!(manifest["project"]["canvasSize"]["preset"], "web");
    // Board lineage: anchor keeps prompt/model/seed/size.
    let anchor = &manifest["board"]["anchor"];
    assert_eq!(anchor["file"], "board/anchor.png");
    assert_eq!(anchor["prompt"], "BOARD PROMPT");
    assert_eq!(anchor["model"], MODEL);
    assert_eq!(anchor["seed"], 7);
    assert_eq!(anchor["size"], "1536x1024");
    assert_eq!(manifest["board"]["candidates"].as_array().unwrap().len(), 2);
    // Page/component entries carry their edit-endpoint lineage.
    let page = &manifest["pages"][0];
    assert_eq!(page["slug"], "dashboard");
    assert_eq!(page["file"], "pages/dashboard/current.png");
    assert_eq!(page["prompt"], "PAGE PROMPT");
    assert_eq!(page["endpoint"], "edits");
    let comp = &manifest["components"][0];
    assert_eq!(comp["type"], "buttons");
    assert_eq!(comp["prompt"], "COMP PROMPT");
    // The smoke script's contract: manifest parses, entries reference files.
    assert!(!manifest["promptLog"].as_array().unwrap().is_empty());

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&out).ok();
}

#[test]
fn export_works_on_sparse_project() {
    // Fresh project with no generations at all: export must not fail.
    let root = tmp("sparse");
    create_project(&root, &base_project()).unwrap();
    let out = tmp("sparse-out");
    let report = export_project(&root, &out).unwrap();
    assert_eq!(report.files, 0);
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["board"]["anchor"], serde_json::Value::Null);
    assert_eq!(manifest["pages"].as_array().unwrap().len(), 0);
    let prompts = std::fs::read_to_string(out.join("PROMPTS.md")).unwrap();
    assert!(prompts.contains("No generations logged yet"));
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&out).ok();
}

#[test]
fn export_fails_without_project() {
    let root = tmp("noproj");
    std::fs::create_dir_all(&root).unwrap();
    let out = tmp("noproj-out");
    let err = export_project(&root, &out).unwrap_err();
    assert_eq!(err.code(), "NOT_A_PROJECT");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn out_dir_inside_project_is_rejected() {
    let root = tmp("inside");
    create_project(&root, &base_project()).unwrap();
    let err = validate_out_dir(&root, &root.join("export")).unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn prompts_md_renders_log_entries() {
    let root = populated_project("prompts");
    let project = load_project(&root).unwrap();
    let md = render_prompts_md(&project);
    assert!(md.starts_with("# PROMPTS.md — e2e样例"));
    assert!(md.contains("## 1. board — generations"));
    assert!(md.contains("model=`gpt-image-2`"));
    assert!(md.contains("seed=`7`"));
    assert!(md.contains("thinking=`medium`"));
    assert!(md.contains("candidates: 0001, 0002"));
    assert!(md.contains("BOARD PROMPT"));
    // Deterministic rendering.
    assert_eq!(md, render_prompts_md(&project));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn design_template_prefills_paths_and_keeps_tokens_blank() {
    let root = populated_project("design");
    let project = load_project(&root).unwrap();
    let md = render_design_template(&project, "manifest.json");
    assert!(md.contains("# DESIGN.md — e2e样例"));
    assert!(md.contains("Source: manifest.json"));
    assert!(md.contains("never invent"));
    // Token tables stay blank (agent fills from images).
    assert!(md.contains("| primary | #… |"));
    // Image paths are prefilled for the agent to VIEW.
    assert!(md.contains("components/button-set/current.png"));
    assert!(md.contains("pages/dashboard/current.png"));
    assert!(md.contains("### dashboard"));
    assert!(md.contains("## Known deviations"));
    assert_eq!(md, render_design_template(&project, "manifest.json"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn generate_params_roundtrip_shape() {
    // GenerateParams ↔ GenParams share the manifest lineage shape.
    let p = GenerateParams {
        prompt: "x".into(),
        size: "1024x1536".into(),
        quality: "high".into(),
        n: 1,
        seed: None,
        thinking: None,
    };
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.get("seed").is_none(), "absent seed must not serialize");
    assert!(v.get("thinking").is_none());
}
