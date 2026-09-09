//! Tests for [`crate::templates`]: embedded assets parse, slot filling is
//! deterministic, unknown templates/slots fail with exit-1 argument errors.

use super::*;

fn sample_template() -> Template {
    parse(
        "sample",
        r#"{
            "id": "sample",
            "appliesTo": "page",
            "name": {"zh": "样例", "en": "Sample"},
            "summary": {"zh": "样例", "en": "Sample"},
            "slots": [
                {"name": "page.slug", "required": true},
                {"name": "page.brief", "required": true},
                {"name": "project.brandBrief", "required": false},
                {"name": "anchor.reference", "required": true},
                {"name": "project.invariants", "required": false}
            ],
            "skeleton": "{anchor.reference}。沿用设计语言。\nPage: {page.slug}\nBrand brief: {project.brandBrief}\nBrief: {page.brief}\n{project.invariants}",
            "fillGuide": {
                "howTo": "逐槽填写。",
                "slots": [
                    {"slot": "page.brief", "what": "布局结构", "goodExample": "顶部导航 3 项", "commonMistakes": ["只写形容词"]}
                ]
            }
        }"#,
    )
    .unwrap()
}

#[test]
fn all_embedded_templates_parse_and_validate() {
    let ids = builtin_ids();
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
    for id in &ids {
        let template = load_builtin(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(template.id, *id);
        assert!(!template.skeleton.trim().is_empty());
        assert!(!template.fill_guide.slots.is_empty(), "{id} needs fillGuide");
    }
}

#[test]
fn manifest_matches_embedded_templates() {
    let manifest = builtin_manifest().unwrap();
    assert_eq!(manifest.templates.len(), EMBEDDED.len());
    assert_eq!(manifest.attribution.license, "MIT");
    assert!(manifest.attribution.url.contains("awesome-gpt-image-2"));
    for entry in &manifest.templates {
        let template = load_builtin(&entry.id).unwrap_or_else(|e| panic!("{}: {e}", entry.id));
        assert_eq!(template.applies_to, entry.applies_to, "{} appliesTo mismatch", entry.id);
        // Manifest slot list mirrors the template's declared slots.
        let declared: Vec<String> = template.slot_names().iter().map(|s| s.to_string()).collect();
        assert_eq!(declared, entry.slots, "{} slot list mismatch", entry.id);
        assert!(!manifest.slot_vocabulary.is_empty());
    }
    // Every vocabulary key is a known engine variable for some kind.
    for key in manifest.slot_vocabulary.keys() {
        assert!(
            VOCABULARY.contains(&key.as_str()),
            "manifest vocabulary key `{key}` unknown to the engine"
        );
    }
}

#[test]
fn default_templates_exist_per_kind() {
    assert_eq!(default_template_id("board"), BOARD_TEMPLATE_ID);
    assert_eq!(default_template_id("page"), PAGE_TEMPLATE_ID);
    assert_eq!(default_template_id("component"), COMPONENT_TEMPLATE_ID);
    assert_eq!(
        load_builtin(BOARD_TEMPLATE_ID).unwrap().applies_to,
        "board"
    );
    assert_eq!(
        load_builtin(PAGE_TEMPLATE_ID).unwrap().applies_to,
        "page"
    );
    assert_eq!(
        load_builtin(COMPONENT_TEMPLATE_ID).unwrap().applies_to,
        "component"
    );
}

#[test]
fn unknown_template_is_exit1_with_templates_list_hint() {
    let err = load_builtin("nope").unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    assert_eq!(err.exit_code(), 1);
    let hint = err.hint();
    assert!(hint.contains("templates list"), "hint: {hint}");
    assert!(err.to_string().contains("nope"));
}

#[test]
fn fill_substitutes_slots_and_drops_all_empty_lines() {
    let template = sample_template();
    let mut vars = Vars::new();
    vars.insert("anchor.reference".into(), "Image 1 是本产品设计系统总板".into());
    vars.insert("page.slug".into(), "dashboard".into());
    vars.insert("page.brief".into(), "顶部导航 3 项".into());
    // Optional slots present in the skeleton are always provided (possibly
    // empty) by the engine; empty ones drop their whole line.
    vars.insert("project.brandBrief".into(), String::new());
    vars.insert("project.invariants".into(), String::new());
    let text = fill(&template, &vars).unwrap();
    assert!(text.starts_with("Image 1 是本产品设计系统总板。沿用设计语言。"), "{text}");
    assert!(text.contains("Page: dashboard"));
    assert!(text.contains("Brief: 顶部导航 3 项"));
    assert!(!text.contains("Brand brief"), "empty optional line must drop:\n{text}");
    assert!(!text.contains("{"), "no unfilled slots remain:\n{text}");
    // Deterministic.
    assert_eq!(fill(&template, &vars).unwrap(), text);
}

#[test]
fn fill_keeps_lines_with_mixed_empty_and_non_empty_slots() {
    let template = parse(
        "mixed",
        r#"{
            "id": "mixed",
            "appliesTo": "board",
            "name": {"zh": "m", "en": "m"},
            "summary": {"zh": "m", "en": "m"},
            "slots": [
                {"name": "canvas.w", "required": true},
                {"name": "project.brandBrief", "required": false}
            ],
            "skeleton": "Canvas {canvas.w} wide. Brand: {project.brandBrief}.",
            "fillGuide": {"howTo": "x", "slots": []}
        }"#,
    )
    .unwrap();
    let mut vars = Vars::new();
    vars.insert("canvas.w".into(), "1536".into());
    vars.insert("project.brandBrief".into(), String::new());
    let text = fill(&template, &vars).unwrap();
    assert_eq!(text, "Canvas 1536 wide. Brand: .", "line keeps a used slot's neighbors");
}

#[test]
fn fill_collapses_blank_runs_and_trims() {
    let template = parse(
        "blank",
        r#"{
            "id": "blank",
            "appliesTo": "board",
            "name": {"zh": "b", "en": "b"},
            "summary": {"zh": "b", "en": "b"},
            "slots": [
                {"name": "canvas.w", "required": true},
                {"name": "project.brandBrief", "required": false},
                {"name": "project.styleBrief", "required": false}
            ],
            "skeleton": "A: {canvas.w}\nBrand: {project.brandBrief}\nStyle: {project.styleBrief}\nTail",
            "fillGuide": {"howTo": "x", "slots": []}
        }"#,
    )
    .unwrap();
    let mut vars = Vars::new();
    vars.insert("canvas.w".into(), "1".into());
    vars.insert("project.brandBrief".into(), String::new());
    vars.insert("project.styleBrief".into(), String::new());
    let text = fill(&template, &vars).unwrap();
    assert_eq!(text, "A: 1\nTail", "dropped lines must not stack blanks:\n{text}");
}

#[test]
fn unknown_slot_in_skeleton_is_exit1() {
    let template = parse(
        "badslot",
        r#"{
            "id": "badslot",
            "appliesTo": "board",
            "name": {"zh": "x", "en": "x"},
            "summary": {"zh": "x", "en": "x"},
            "slots": [{"name": "canvas.w", "required": true}],
            "skeleton": "Hello {page.slug}",
            "fillGuide": {"howTo": "x", "slots": []}
        }"#,
    );
    // Validation catches the undeclared slot at parse time.
    let err = template.unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    assert!(err.to_string().contains("undeclared slot"), "{err}");
}

#[test]
fn validation_rejects_unknown_vocabulary_and_bad_kinds() {
    let bad_slot = parse(
        "vocab",
        r#"{
            "id": "vocab",
            "appliesTo": "board",
            "name": {"zh": "x", "en": "x"},
            "summary": {"zh": "x", "en": "x"},
            "slots": [{"name": "user.email", "required": true}],
            "skeleton": "Email {user.email}",
            "fillGuide": {"howTo": "x", "slots": []}
        }"#,
    )
    .unwrap_err();
    assert!(bad_slot.to_string().contains("unknown slot"), "{bad_slot}");

    let bad_kind = parse(
        "kind",
        r#"{
            "id": "kind",
            "appliesTo": "poster",
            "name": {"zh": "x", "en": "x"},
            "summary": {"zh": "x", "en": "x"},
            "slots": [],
            "skeleton": "plain",
            "fillGuide": {"howTo": "x", "slots": []}
        }"#,
    )
    .unwrap_err();
    assert!(bad_kind.to_string().contains("appliesTo"), "{bad_kind}");
}

#[test]
fn extract_slots_scans_braces_and_leaves_prose() {
    assert_eq!(
        extract_slots("A {canvas.w}x{canvas.h} sheet, cost {n/a} and literal }{"),
        vec!["canvas.w".to_string(), "canvas.h".to_string()],
        "only identifier-shaped braces count"
    );
    assert!(extract_slots("no slots here").is_empty());
}

#[test]
fn allowed_variables_follow_the_kind() {
    let board = allowed_variables("board");
    assert!(board.contains(&"canvas.w"));
    assert!(!board.contains(&"page.slug"));

    let page = allowed_variables("page");
    assert!(page.contains(&"page.brief") && page.contains(&"anchor.reference"));
    assert!(!page.contains(&"component.kind"));

    let component = allowed_variables("component");
    assert!(component.contains(&"component.kind") && component.contains(&"anchor.reference"));
    assert!(!component.contains(&"page.slug"));
}

#[test]
fn corrupt_json_is_invalid_arg() {
    let err = parse("corrupt", "{not json").unwrap_err();
    assert_eq!(err.code(), "INVALID_ARG");
    assert_eq!(err.exit_code(), 1);
}
