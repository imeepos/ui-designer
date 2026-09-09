//! Rudder CLI entry point (binary name: `rudder`).
//!
//! Implements docs/ARCHITECTURE.md §6. Global flags: `--project`, `--json`,
//! `--dry-run`, `--yes`. Real image-API spend requires `--yes`; without it
//! generate commands print their request plan (dry-run). Exit codes:
//! 0 ok · 1 bad argument · 2 API error · 3 project state.

mod output;

use clap::{error::ErrorKind as ClapErrorKind, Parser, Subcommand};
use output::{emit_err, emit_ok, format_plan, format_plan_summary, CmdResult};
use rudder_core::config::{credential, resolve_base_url, Config};
use rudder_core::image::ImageClient;
use rudder_core::ops::{self, GenerateOptions, Target};
use rudder_core::store;
use rudder_core::{export, RudderError};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rudder",
    version = rudder_core::VERSION,
    about = "Rudder - AI UI design studio CLI",
    long_about = None,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    /// Target project directory (default: cwd if it is a project, else last used).
    #[arg(long, global = true, value_name = "PATH")]
    project: Option<PathBuf>,

    /// Machine-readable envelope on stdout: {ok, data|error}.
    #[arg(long, global = true)]
    json: bool,

    /// Print the request plan only; never touch the network (free).
    #[arg(long, global = true)]
    dry_run: bool,

    /// REQUIRED for any real image API spend.
    #[arg(long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create a new project (directory layout + project.json).
    Init {
        /// Project display name.
        name: String,
        /// web | mobile | desktop | WxH (16-multiples, ≤3:1, 0.65-8.3 MP).
        #[arg(long, default_value = "web")]
        size: String,
        /// Project directory (default: current directory).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Seeds styleBrief.
        #[arg(long)]
        brief: Option<String>,
    },
    /// Design-system board (the style anchor).
    Board {
        #[command(subcommand)]
        command: BoardCommand,
    },
    /// Project metadata (name / briefs) — amend without regenerating.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Functional pages derived from the anchor.
    Page {
        #[command(subcommand)]
        command: PageCommand,
    },
    /// Component detail sheets derived from the anchor.
    Component {
        #[command(subcommand)]
        command: ComponentCommand,
    },
    /// Status overview: anchor present? pages/components with candidate counts.
    List {
        /// Optional filter: pages | components.
        arg: Option<String>,
    },
    /// Export the asset bundle (images + manifest.json + PROMPTS.md + DESIGN.template.md).
    Export {
        /// Output directory (default: ./export).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also copy board exploration candidates into the bundle
        /// (default: anchor + picked currents only).
        #[arg(long)]
        with_candidates: bool,
    },
    /// Self-test: sample project → board → 1 page → 1 component → export.
    E2e {
        /// Quality for the self-test generation steps (low keeps it cheap).
        #[arg(long, default_value = "low")]
        quality: String,
    },
    /// Generation defaults + credentials: non-secret keys live in
    /// ~/Rudder/config.json; `api-key` lives in the OS keychain only.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand, Debug)]
enum BoardCommand {
    /// Generate board candidates (board/candidates/NNNN.png).
    Generate {
        /// Candidate count, 1-4 (default 4).
        #[arg(long)]
        n: Option<u32>,
        /// low | medium | high (default from config, else low).
        #[arg(long)]
        quality: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        /// thinking effort passthrough (default from config, else medium).
        #[arg(long)]
        thinking: Option<String>,
        /// Extra inspiration reference images (switches to the edits endpoint).
        #[arg(long = "ref")]
        refs: Vec<PathBuf>,
    },
    /// Pick the anchor: candidate → board/anchor.png.
    Pick {
        candidate_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCommand {
    /// Amend project metadata (`--name`, `--brand-brief`, `--style-brief`;
    /// at least one required) — the supported way to iterate on briefs.
    Update {
        /// New project display name.
        #[arg(long)]
        name: Option<String>,
        /// Replace the brand brief.
        #[arg(long = "brand-brief")]
        brand_brief: Option<String>,
        /// Replace the style brief (board mood + generation invariants).
        #[arg(long = "style-brief")]
        style_brief: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum PageCommand {
    /// Register a page (slug: a-z0-9-).
    Add {
        slug: String,
        #[arg(long)]
        brief: String,
    },
    /// List registered pages.
    List,
    /// Generate candidates for <slug> or --all.
    Generate {
        /// Page slug, or `--all` for every page.
        #[arg(allow_hyphen_values = true)]
        target: String,
        /// Candidate count, 1-4 (default 1).
        #[arg(long)]
        n: Option<u32>,
        #[arg(long)]
        quality: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        thinking: Option<String>,
        /// Extra layout reference images (passed after the anchor).
        #[arg(long = "ref")]
        refs: Vec<PathBuf>,
    },
    /// Promote a candidate to pages/<slug>/current.png (old current → history/).
    Pick {
        slug: String,
        candidate_id: String,
    },
    /// Amend a page's layout brief (UI-REVIEW 缺陷 2: no hand-editing
    /// project.json).
    Update {
        slug: String,
        /// New layout brief (replaces the stored one).
        #[arg(long)]
        brief: String,
    },
}

#[derive(Subcommand, Debug)]
enum ComponentCommand {
    /// Register a component sheet (name: a-z0-9-).
    Add {
        name: String,
        /// e.g. buttons | forms | cards | navigation | icons | tables | modals.
        #[arg(long)]
        r#type: String,
        #[arg(long)]
        brief: String,
    },
    /// List registered components.
    List,
    /// Generate candidates for <name> or --all.
    Generate {
        /// Component name, or `--all` for every component.
        #[arg(allow_hyphen_values = true)]
        target: String,
        /// Candidate count, 1-4 (default 1).
        #[arg(long)]
        n: Option<u32>,
        #[arg(long)]
        quality: Option<String>,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        thinking: Option<String>,
        #[arg(long = "ref")]
        refs: Vec<PathBuf>,
    },
    /// Promote a candidate to components/<name>/current.png (old current → history/).
    Pick {
        name: String,
        candidate_id: String,
    },
    /// Amend a component sheet's type and/or brief.
    Update {
        name: String,
        /// New component type label (e.g. buttons | forms | cards | ...).
        #[arg(long)]
        r#type: Option<String>,
        /// New component brief (replaces the stored one).
        #[arg(long)]
        brief: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Print a key's effective value (quality | thinking | n | base_url).
    Get { key: String },
    /// Set a key: quality low|medium|high · thinking low|medium|high ·
    /// n 1-4 · base_url http(s). `api-key` reads the secret from STDIN
    /// (never argv — arguments leak into shell history) into the OS keychain.
    Set {
        key: String,
        /// Value for non-secret keys; must be omitted for `api-key`.
        value: Option<String>,
    },
    /// Remove a stored credential (only `api-key`; idempotent).
    Clear { key: String },
    /// Report the resolved base URL and key source (env/keychain/none),
    /// then probe GET {base}/v1/models (free) for the available model count.
    Test,
}

fn main() {
    // Exit-code contract: clap's own parse errors count as bad arguments (1),
    // not clap's default 2.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let is_help_or_version =
                matches!(err.kind(), ClapErrorKind::DisplayHelp | ClapErrorKind::DisplayVersion);
            let _ = err.print();
            if is_help_or_version {
                std::process::exit(0);
            }
            emit_parse_error();
            std::process::exit(1);
        }
    };
    let code = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(dispatch(cli));
    std::process::exit(code);
}

/// clap parse failures in --json mode still honor the envelope contract.
fn emit_parse_error() {
    if !std::env::args().any(|a| a == "--json") {
        return;
    }
    let envelope = json!({
        "ok": false,
        "error": {
            "code": "INVALID_ARG",
            "message": "invalid arguments (run `rudder --help`)",
            "hint": "check flags against skill/rudder-design/references/cli.md"
        }
    });
    println!("{envelope}");
}

fn resolve_root(project: Option<&PathBuf>) -> Result<PathBuf, RudderError> {
    store::resolve_project_root(project.map(|p| p.as_path()))
}

/// Final dry-run decision: explicit --dry-run always wins; a missing --yes
/// means dry-run for anything that could spend money.
fn spend_allowed(cli: &Cli) -> bool {
    cli.yes && !cli.dry_run
}

fn generate_opts(
    n: Option<u32>,
    quality: Option<String>,
    seed: Option<u64>,
    thinking: Option<String>,
    refs: Vec<PathBuf>,
    cli: &Cli,
) -> GenerateOptions {
    GenerateOptions {
        n,
        quality,
        seed,
        thinking,
        refs,
        dry_run: !spend_allowed(cli),
        assume_anchor: false,
    }
}

fn build_client(cli: &Cli) -> Result<ImageClient, RudderError> {
    ImageClient::from_config(!spend_allowed(cli))
}

async fn dispatch(cli: Cli) -> i32 {
    let json_mode = cli.json;
    let result = run(&cli).await;
    match result {
        Ok(ok) => emit_ok(json_mode, &ok),
        Err(err) => emit_err(json_mode, &err),
    }
}

async fn run(cli: &Cli) -> Result<CmdResult, RudderError> {
    match &cli.command {
        Command::Init { name, size, dir, brief } => {
            let dir = dir.clone().unwrap_or_else(|| PathBuf::from("."));
            ops::init_project(&dir, name, size, brief.as_deref())?;
            let project = store::load_project(&dir)?;
            let dir_abs = std::fs::canonicalize(&dir).unwrap_or(dir);
            Ok(CmdResult::new(
                format!(
                    "project `{}` created at {} (canvas {})",
                    project.name,
                    dir_abs.display(),
                    project.canvas_size.to_api_string()
                ),
                json!({
                    "projectId": project.id,
                    "dir": dir_abs.display().to_string(),
                    "canvasSize": {
                        "w": project.canvas_size.w,
                        "h": project.canvas_size.h,
                        "preset": ops::preset_name(&project.canvas_size),
                    }
                }),
            ))
        }

        Command::Board { command } => match command {
            BoardCommand::Generate { n, quality, seed, thinking, refs } => {
                let root = resolve_root(cli.project.as_ref())?;
                let client = build_client(cli)?;
                let report = ops::generate(
                    &root,
                    &Target::Board,
                    generate_opts(*n, quality.clone(), *seed, thinking.clone(), refs.clone(), cli),
                    &client,
                )
                .await?;
                Ok(generate_result(&report))
            }
            BoardCommand::Pick { candidate_id } => {
                let root = resolve_root(cli.project.as_ref())?;
                let file = ops::board_pick(&root, candidate_id)?;
                Ok(CmdResult::new(
                    format!("anchor set to candidate {candidate_id} → {file}"),
                    json!({ "anchor": candidate_id, "file": file }),
                ))
            }
        },

        Command::Project { command } => match command {
            ProjectCommand::Update { name, brand_brief, style_brief } => {
                let root = resolve_root(cli.project.as_ref())?;
                let project = ops::project_update(
                    &root,
                    ops::ProjectUpdate {
                        name: name.clone(),
                        brand_brief: brand_brief.clone(),
                        style_brief: style_brief.clone(),
                    },
                )?;
                Ok(CmdResult::new(
                    format!(
                        "project updated: name `{}` · brand brief {} char(s) · style brief {} char(s)",
                        project.name,
                        project.brand_brief.chars().count(),
                        project.style_brief.chars().count()
                    ),
                    json!({
                        "name": project.name,
                        "brandBrief": project.brand_brief,
                        "styleBrief": project.style_brief,
                    }),
                ))
            }
        },

        Command::Page { command } => match command {
            PageCommand::Add { slug, brief } => {
                let root = resolve_root(cli.project.as_ref())?;
                let page = ops::page_add(&root, slug, brief)?;
                Ok(CmdResult::new(
                    format!("page `{}` registered (brief {} chars)", page.slug, page.brief.chars().count()),
                    json!({ "slug": page.slug, "brief": page.brief }),
                ))
            }
            PageCommand::List => {
                let root = resolve_root(cli.project.as_ref())?;
                let status = ops::status(&root)?;
                Ok(CmdResult::new(
                    format!("{} page(s)", status.pages.len()),
                    json!({ "pages": status.pages }),
                ))
            }
            PageCommand::Generate { target, n, quality, seed, thinking, refs } => {
                let root = resolve_root(cli.project.as_ref())?;
                let client = build_client(cli)?;
                let slugs = expand_targets(&root, target, true)?;
                let mut reports = Vec::new();
                for slug in slugs {
                    let report = ops::generate(
                        &root,
                        &Target::Page(slug.clone()),
                        generate_opts(*n, quality.clone(), *seed, thinking.clone(), refs.clone(), cli),
                        &client,
                    )
                    .await?;
                    reports.push(report);
                }
                // UI-REVIEW 缺陷 3: a single target answers with the exact
                // board shape (top-level `candidates`); only `--all` wraps
                // results in an array.
                if reports.len() == 1 {
                    return Ok(generate_result(&reports[0]));
                }
                Ok(generate_all_result("page", reports))
            }
            PageCommand::Pick { slug, candidate_id } => {
                let root = resolve_root(cli.project.as_ref())?;
                let current = ops::page_pick(&root, slug, candidate_id)?;
                Ok(CmdResult::new(
                    format!("page `{slug}` current ← candidate {candidate_id} ({current})"),
                    json!({ "slug": slug, "picked": candidate_id, "current": current }),
                ))
            }
            PageCommand::Update { slug, brief } => {
                let root = resolve_root(cli.project.as_ref())?;
                let page = ops::page_update(&root, slug, brief)?;
                Ok(CmdResult::new(
                    format!("page `{}` brief updated ({} char(s))", page.slug, page.brief.chars().count()),
                    json!({ "slug": page.slug, "brief": page.brief }),
                ))
            }
        },

        Command::Component { command } => match command {
            ComponentCommand::Add { name, r#type, brief } => {
                let root = resolve_root(cli.project.as_ref())?;
                let component = ops::component_add(&root, name, r#type, brief)?;
                Ok(CmdResult::new(
                    format!("component `{}` ({}) registered", component.name, component.kind),
                    json!({ "name": component.name, "type": component.kind, "brief": component.brief }),
                ))
            }
            ComponentCommand::List => {
                let root = resolve_root(cli.project.as_ref())?;
                let status = ops::status(&root)?;
                Ok(CmdResult::new(
                    format!("{} component(s)", status.components.len()),
                    json!({ "components": status.components }),
                ))
            }
            ComponentCommand::Generate { target, n, quality, seed, thinking, refs } => {
                let root = resolve_root(cli.project.as_ref())?;
                let client = build_client(cli)?;
                let names = expand_targets(&root, target, false)?;
                let mut reports = Vec::new();
                for name in names {
                    let report = ops::generate(
                        &root,
                        &Target::Component(name.clone()),
                        generate_opts(*n, quality.clone(), *seed, thinking.clone(), refs.clone(), cli),
                        &client,
                    )
                    .await?;
                    reports.push(report);
                }
                if reports.len() == 1 {
                    return Ok(generate_result(&reports[0]));
                }
                Ok(generate_all_result("component", reports))
            }
            ComponentCommand::Pick { name, candidate_id } => {
                let root = resolve_root(cli.project.as_ref())?;
                let current = ops::component_pick(&root, name, candidate_id)?;
                Ok(CmdResult::new(
                    format!("component `{name}` current ← candidate {candidate_id} ({current})"),
                    json!({ "name": name, "picked": candidate_id, "current": current }),
                ))
            }
            ComponentCommand::Update { name, r#type, brief } => {
                let root = resolve_root(cli.project.as_ref())?;
                let component = ops::component_update(&root, name, r#type.as_deref(), brief.as_deref())?;
                Ok(CmdResult::new(
                    format!(
                        "component `{}` updated (type `{}`, brief {} char(s))",
                        component.name,
                        component.kind,
                        component.brief.chars().count()
                    ),
                    json!({ "name": component.name, "type": component.kind, "brief": component.brief }),
                ))
            }
        },

        Command::List { arg } => {
            let root = resolve_root(cli.project.as_ref())?;
            let status = ops::status(&root)?;
            let human = format!(
                "project `{}` ({}) · anchor: {} · pages: {} · components: {}",
                status.name,
                status.canvas_size,
                status.anchor.as_deref().unwrap_or("none"),
                status.pages.len(),
                status.components.len()
            );
            let data = match arg.as_deref() {
                Some("pages") => json!({ "pages": status.pages }),
                Some("components") => json!({ "components": status.components }),
                _ => json!(status),
            };
            Ok(CmdResult::new(human, data))
        }

        Command::Export { out, with_candidates } => {
            let root = resolve_root(cli.project.as_ref())?;
            let out = out.clone().unwrap_or_else(|| PathBuf::from("export"));
            export::validate_out_dir(&root, &out)?;
            let report = export::export_project_with(
                &root,
                &out,
                &export::ExportOptions { with_candidates: *with_candidates },
            )?;
            // UI-REVIEW 缺陷 10: never silently drop generated-but-unpicked
            // targets — warn on stderr (logs channel) in every mode.
            for warning in &report.warnings {
                eprintln!("warning: {warning}");
            }
            Ok(CmdResult::new(
                format!(
                    "exported {} image file(s) → {} (manifest.json, PROMPTS.md, DESIGN.template.md)",
                    report.files,
                    report.out.display()
                ),
                json!({
                    "out": report.out.display().to_string(),
                    "manifest": report.manifest.display().to_string(),
                    "files": report.files,
                    "withCandidates": with_candidates,
                    "warnings": report.warnings,
                }),
            ))
        }

        Command::E2e { quality } => {
            if !["low", "medium", "high"].contains(&quality.as_str()) {
                return Err(RudderError::InvalidArg {
                    detail: "quality must be one of low|medium|high".into(),
                });
            }
            let client = build_client(cli)?;
            let report = ops::run_e2e(&client, quality).await?;
            let steps: Vec<String> = report
                .steps
                .iter()
                .map(|s| {
                    if s.dry_run {
                        format!("{} (plan)", s.step)
                    } else {
                        s.step.clone()
                    }
                })
                .collect();
            Ok(CmdResult::new(
                format!(
                    "e2e {}: {} → {}",
                    if report.dry_run { "dry-run complete" } else { "complete" },
                    steps.join(" → "),
                    report.project_dir
                ),
                serde_json::to_value(&report).expect("e2e report serializes"),
            ))
        }

        Command::Config { command } => match command {
            ConfigCommand::Get { key } => {
                let config = Config::load();
                let value = config.get(key)?;
                Ok(CmdResult::new(
                    format!("{key} = {value}"),
                    json!({ "key": key, "value": value }),
                ))
            }
            ConfigCommand::Set { key, value } => {
                if key == "api-key" || key == "api_key" {
                    return config_set_api_key(value.as_deref());
                }
                let Some(value) = value else {
                    return Err(RudderError::InvalidArg {
                        detail: format!("config set `{key}` requires a <value>"),
                    });
                };
                let mut config = Config::load();
                config.set(key, value)?;
                config.save()?;
                // Echo the effective (normalized) value, matching `config get`.
                let effective = config.get(key)?;
                Ok(CmdResult::new(
                    format!("{key} = {effective}"),
                    json!({ "key": key, "value": effective }),
                ))
            }
            ConfigCommand::Clear { key } => {
                if key != "api-key" && key != "api_key" {
                    return Err(RudderError::InvalidArg {
                        detail: format!("only `api-key` can be cleared (got `{key}`)"),
                    });
                }
                credential::clear_api_key()?;
                Ok(CmdResult::new(
                    "api-key removed from OS keychain",
                    json!({ "key": "api-key", "stored": Value::Null }),
                ))
            }
            ConfigCommand::Test => config_test().await,
        },
    }
}

/// `config set api-key`: read the secret from stdin (never argv) and store
/// it in the OS keychain. Output only ever shows the masked tail.
fn config_set_api_key(argv_value: Option<&str>) -> Result<CmdResult, RudderError> {
    if argv_value.is_some() {
        return Err(RudderError::InvalidArg {
            detail: "api-key must be piped via stdin (`echo <key> | rudder config set api-key`); \
                     passing it as an argument would leak into shell history"
                .into(),
        });
    }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let piped = line.trim();
    if piped.is_empty() {
        return Err(RudderError::InvalidArg {
            detail: "no api key found on stdin; use `echo <key> | rudder config set api-key`"
                .into(),
        });
    }
    credential::set_api_key(piped)?;
    let tail = credential::tail4(piped);
    Ok(CmdResult::new(
        format!("api-key stored in OS keychain (tail …{tail})"),
        json!({ "key": "api-key", "stored": "keychain", "tail": tail }),
    ))
}

/// `config test`: report base URL + key source; when a key resolves, probe
/// `{base}/v1/models` (free call) for the visible model count. Without a
/// key the command still succeeds and reports `keySource: "none"`.
async fn config_test() -> Result<CmdResult, RudderError> {
    let base = resolve_base_url();
    let resolution = credential::resolve_api_key();
    let source = resolution.source.as_str();
    let Some(key) = resolution.key else {
        return Ok(CmdResult::new(
            format!(
                "base: {base} · key: none — not configured \
                 (export OPENAI_API_KEY or `echo <key> | rudder config set api-key`)"
            ),
            json!({
                "baseUrl": base,
                "keySource": "none",
                "tested": false,
                "modelCount": Value::Null,
            }),
        ));
    };
    let report = credential::test_connection(&base, &key).await?;
    Ok(CmdResult::new(
        format!(
            "base: {base} · key: {source} · models: {} ({} ✓)",
            report.model_count,
            rudder_core::image::MODEL
        ),
        json!({
            "baseUrl": base,
            "keySource": source,
            "tested": true,
            "modelCount": report.model_count,
            "imageModelAvailable": true,
        }),
    ))
}

/// Resolve `<slug|--all>` positional (allows the leading-hyphen form).
fn expand_targets(root: &std::path::Path, target: &str, pages: bool) -> Result<Vec<String>, RudderError> {
    if target == "--all" || target == "all" {
        let project = store::load_project(root)?;
        return Ok(if pages {
            project.pages.into_iter().map(|p| p.slug).collect()
        } else {
            project.components.into_iter().map(|c| c.name).collect()
        });
    }
    Ok(vec![target.to_string()])
}

/// Trimmed candidate row for `--json` output. The prompt lives ONCE in
/// `plan.params.prompt` — candidates reference it by id/file
/// (UI-REVIEW 缺陷 4), while seed/size/quality stay for reproducibility.
fn candidate_json(candidate: &ops::CandidateReport) -> Value {
    json!({
        "id": candidate.id,
        "file": candidate.file,
        "seed": candidate.seed,
        "size": candidate.size,
        "quality": candidate.quality,
    })
}

/// One generate report in the envelope shape shared by board and
/// single-target page/component (UI-REVIEW 缺陷 3): always
/// `{dryRun, kind, target, plan, candidates[]}` at the TOP LEVEL.
fn report_json(report: &ops::GenerateReport) -> Value {
    json!({
        "dryRun": report.dry_run,
        "kind": report.kind,
        "target": report.target,
        "plan": serde_json::to_value(&report.plan).expect("plan serializes"),
        "candidates": report.candidates.iter().map(candidate_json).collect::<Vec<_>>(),
    })
}

fn generate_result(report: &ops::GenerateReport) -> CmdResult {
    let data = report_json(report);
    if report.dry_run {
        CmdResult::with_detail(
            format_plan_summary(&report.plan, &report.kind, &report.target),
            format_plan(&report.plan),
            data,
        )
    } else {
        let ids: Vec<&str> = report.candidates.iter().map(|c| c.id.as_str()).collect();
        CmdResult::new(
            format!(
                "{} {} → {} candidate(s): {}",
                report.kind,
                if report.target.is_empty() { String::new() } else { format!("`{}`", report.target) },
                report.candidates.len(),
                ids.join(", ")
            ),
            data,
        )
    }
}

/// `--all` fan-out: one envelope with a `results` array; every entry keeps
/// the same `{kind, target, plan, candidates}` shape as single-target runs.
fn generate_all_result(kind: &str, reports: Vec<ops::GenerateReport>) -> CmdResult {
    let dry_run = reports.iter().all(|r| r.dry_run);
    let mut summaries = Vec::new();
    let mut details = Vec::new();
    let mut entries = Vec::new();
    for report in &reports {
        if report.dry_run {
            summaries.push(format_plan_summary(&report.plan, &report.kind, &report.target));
            details.push(format_plan(&report.plan));
        } else {
            let ids: Vec<&str> = report.candidates.iter().map(|c| c.id.as_str()).collect();
            summaries.push(format!(
                "{kind} `{}` → candidate(s): {}",
                report.target,
                ids.join(", ")
            ));
        }
        entries.push(json!({
            "kind": report.kind,
            "target": report.target,
            "plan": serde_json::to_value(&report.plan).expect("plan serializes"),
            "candidates": report.candidates.iter().map(candidate_json).collect::<Vec<_>>(),
        }));
    }
    let data = json!({
        "dryRun": dry_run,
        "kind": kind,
        "results": entries,
    });
    let summary = if summaries.is_empty() {
        format!("{kind} --all: no registered targets")
    } else {
        summaries.join("; ")
    };
    if details.is_empty() {
        CmdResult::new(summary, data)
    } else {
        CmdResult::with_detail(summary, details.join("\n"), data)
    }
}
