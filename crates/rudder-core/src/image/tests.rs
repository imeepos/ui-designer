//! Tests for [`crate::image`]: dry-run plan structure, b64 decoding, and the
//! full generate/edit flow against a hand-rolled in-process HTTP mock server
//! (see [`crate::test_support`]; no real network, no new dev-dependencies).

use super::*;
use crate::test_support::{b64_response, MockServer};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const PNG_BYTES_A: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
const PNG_BYTES_B: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 9, 8, 7, 6];

fn params(prompt: &str, n: u32) -> GenerateParams {
    GenerateParams {
        prompt: prompt.to_string(),
        size: "1536x1024".into(),
        quality: "low".into(),
        n,
        seed: Some(42),
        thinking: Some("medium".into()),
    }
}

fn temp_image(tag: &str, bytes: &[u8], ext: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rudder-image-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(format!("{tag}.{ext}"));
    std::fs::write(&path, bytes).expect("write temp image");
    path
}

fn dry_client(url: String) -> ImageClient {
    ImageClient::new(url, Some("test-key".into()), true, Duration::from_millis(1), Duration::from_secs(5))
        .expect("client builds")
}

fn live_client(url: String) -> ImageClient {
    ImageClient::new(url, Some("test-key".into()), false, Duration::from_millis(1), Duration::from_secs(5))
        .expect("client builds")
}

// ---------------------------------------------------------------------------
// Dry-run plan structure
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dry_run_plan_generations_structure() {
    // No server at all: dry-run must never touch the network.
    let client = dry_client("https://api.example.com/".into());
    let out = client
        .run_generations(&params("board prompt", 4))
        .await
        .expect("dry-run never fails on network");
    assert!(out.images.is_none(), "dry-run returns no images");
    let plan = out.plan;
    assert_eq!(plan.endpoint, "generations");
    assert_eq!(plan.method, "POST");
    assert_eq!(plan.url, "https://api.example.com/v1/images/generations");
    assert!(plan.auth_header_present);
    assert!(plan.images.is_empty());
    assert_eq!(plan.params["model"], MODEL);
    assert_eq!(plan.params["prompt"], "board prompt");
    assert_eq!(plan.params["size"], "1536x1024");
    assert_eq!(plan.params["quality"], "low");
    assert_eq!(plan.params["n"], 4);
    assert_eq!(plan.params["seed"], 42);
    assert_eq!(plan.params["thinking"], "medium");
    // Plan is JSON-serializable (the CLI --json payload).
    serde_json::to_value(&plan).expect("plan serializes");
}

#[tokio::test]
async fn dry_run_plan_reports_missing_credentials_without_failing() {
    let client = ImageClient::new(
        DEFAULT_BASE_URL,
        None,
        true,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .unwrap();
    let plan = client.plan_generations(&params("p", 1));
    assert_eq!(plan.url, "https://veren.top/api/v1/images/generations");
    assert!(!plan.auth_header_present);
}

#[tokio::test]
async fn dry_run_plan_edits_structure() {
    let anchor = temp_image("anchor", PNG_BYTES_A, "png");
    let r1 = temp_image("ref1", PNG_BYTES_B, "jpg");
    let client = dry_client("http://127.0.0.1:9".into()); // closed port; must not be touched
    let refs = vec![
        ImageRef { role: "anchor".into(), path: anchor.clone() },
        ImageRef { role: "ref".into(), path: r1.clone() },
    ];
    let out = client
        .run_edits(&params("page prompt", 2), &refs)
        .await
        .expect("dry-run never fails on network");
    assert!(out.images.is_none());
    let plan = out.plan;
    assert_eq!(plan.endpoint, "edits");
    assert_eq!(plan.url, "http://127.0.0.1:9/v1/images/edits");
    assert_eq!(plan.params["image[]"][0], anchor.display().to_string());
    assert_eq!(plan.images.len(), 2);
    assert_eq!(plan.images[0].role, "anchor");
    assert!(!plan.images[0].missing);
    assert_eq!(plan.images[0].bytes, Some(PNG_BYTES_A.len() as u64));
    assert_eq!(plan.images[0].mime.as_deref(), Some("image/png"));
    assert_eq!(plan.images[1].mime.as_deref(), Some("image/jpeg"));
    // Missing files are surfaced honestly in the plan.
    let ghost = ImageRef { role: "ref".into(), path: PathBuf::from("/nonexistent/ref.png") };
    let plan = client.plan_edits(&params("p", 1), &[ghost]);
    assert!(plan.images[0].missing);
    assert_eq!(plan.images[0].bytes, None);
}

// ---------------------------------------------------------------------------
// decode_b64_images unit behavior
// ---------------------------------------------------------------------------

#[test]
fn decode_handles_data_url_prefix_and_rejects_garbage() {
    let engine = base64::engine::general_purpose::STANDARD;
    let body = json!({ "data": [
        { "b64_json": format!("data:image/png;base64,{}", engine.encode(PNG_BYTES_A)) },
        { "b64_json": engine.encode(PNG_BYTES_B) },
    ]})
    .to_string()
    .into_bytes();
    let images = decode_b64_images(&body).unwrap();
    assert_eq!(images, vec![PNG_BYTES_A.to_vec(), PNG_BYTES_B.to_vec()]);

    for (bad, why) in [
        (json!({ "data": [] }).to_string(), "empty data"),
        (json!({ "data": [{}] }).to_string(), "no b64_json"),
        (json!({ "error": "nope" }).to_string(), "no data key"),
        (json!({ "data": [{ "b64_json": "!!!not-base64!!!" }] }).to_string(), "bad base64"),
        ("not json at all".to_string(), "not json"),
    ] {
        let err = decode_b64_images(bad.as_bytes().to_vec().as_slice()).unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE", "{why}");
    }
}

// ---------------------------------------------------------------------------
// Live-mode flows against the mock server
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mock_generate_success_decodes_b64() {
    let server = MockServer::start(|_req, _i| (200, b64_response(&[PNG_BYTES_A, PNG_BYTES_B])));
    let client = live_client(server.url());
    let out = client
        .run_generations(&params("board prompt", 2))
        .await
        .expect("mock returns 200");
    let images = out.images.expect("real run returns images");
    assert_eq!(images, vec![PNG_BYTES_A.to_vec(), PNG_BYTES_B.to_vec()]);

    let reqs = server.recorded();
    assert_eq!(reqs.len(), 1, "exactly one HTTP request");
    let req = &reqs[0];
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/v1/images/generations");
    assert_eq!(req.header("authorization"), Some("Bearer test-key"));
    assert!(req.header("content-type").unwrap_or_default().starts_with("application/json"));
    let sent: Value = serde_json::from_slice(&req.body).expect("JSON body");
    assert_eq!(sent["model"], MODEL);
    assert_eq!(sent["prompt"], "board prompt");
    assert_eq!(sent["n"], 2);
    assert_eq!(sent["seed"], 42);
    assert_eq!(sent["thinking"], "medium");
}

#[tokio::test]
async fn hard_timeout_covers_hanging_connection() {
    // A listener that accepts and never replies (proxy stall). The hard
    // per-attempt ceiling must convert the hang into API_UNREACHABLE within
    // the client timeout instead of waiting forever.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let _ = stream; // hold the socket open, never write
        }
    });
    let client = ImageClient::new(
        format!("http://{addr}"),
        Some("test-key".into()),
        false,
        Duration::from_millis(1),
        Duration::from_millis(500),
    )
    .expect("client builds");
    let started = std::time::Instant::now();
    let err = client
        .run_generations(&params("hang", 1))
        .await
        .expect_err("hanging connection must fail fast");
    assert_eq!(err.code(), "API_UNREACHABLE");
    assert_eq!(err.exit_code(), 2);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "hard ceiling applies (took {:?})",
        started.elapsed()
    );
}

#[tokio::test]
async fn mock_retry_on_429_then_500_then_success() {
    let server = MockServer::start(|_req, i| match i {
        0 => (429, b"{'error':'rate limited'}".to_vec()),
        1 => (500, b"{'error':'server boom'}".to_vec()),
        _ => (200, b64_response(&[PNG_BYTES_A])),
    });
    let client = live_client(server.url());
    let out = client
        .run_generations(&params("retry me", 1))
        .await
        .expect("third attempt succeeds");
    assert_eq!(out.images, Some(vec![PNG_BYTES_A.to_vec()]));
    assert_eq!(server.recorded().len(), 3, "initial + 2 retries");
}

#[tokio::test]
async fn mock_retry_on_malformed_2xx_then_success() {
    // A 2xx whose body has no b64_json (observed in the wild from proxies)
    // must be retried like 429/5xx instead of failing immediately.
    let server = MockServer::start(|_req, i| match i {
        0 => (200, br#"{"data":[{"url":"https://cdn.example/img.png"}]}"#.to_vec()),
        _ => (200, b64_response(&[PNG_BYTES_A])),
    });
    let client = live_client(server.url());
    let out = client
        .run_generations(&params("retry me", 1))
        .await
        .expect("second attempt succeeds");
    assert_eq!(out.images, Some(vec![PNG_BYTES_A.to_vec()]));
    assert_eq!(server.recorded().len(), 2, "malformed body retried once");
}

#[tokio::test]
async fn mock_malformed_2xx_exhausted_maps_to_bad_response() {
    let server = MockServer::start(|_req, _i| (200, br#"{"data":[{}]}"#.to_vec()));
    let client = live_client(server.url());
    let err = client
        .run_generations(&params("never succeeds", 1))
        .await
        .expect_err("bad body forever must fail");
    assert_eq!(err.code(), "BAD_RESPONSE");
    assert_eq!(err.exit_code(), 2);
    assert_eq!(
        server.recorded().len(),
        1 + MAX_RETRIES as usize,
        "initial attempt + at most 3 retries"
    );
}

#[tokio::test]
async fn mock_retry_exhausted_maps_to_rate_limited_exit2() {
    let server = MockServer::start(|_req, _i| (429, b"{'error':'quota exhausted'}".to_vec()));
    let client = live_client(server.url());
    let err = client
        .run_generations(&params("never succeeds", 1))
        .await
        .expect_err("429 forever must fail");
    match &err {
        RudderError::RateLimited { body_summary } => {
            assert!(body_summary.contains("quota"), "body summary kept: {body_summary}");
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
    assert_eq!(err.exit_code(), 2);
    assert_eq!(err.code(), "RATE_LIMITED");
    assert_eq!(
        server.recorded().len(),
        1 + MAX_RETRIES as usize,
        "initial attempt + at most 3 retries"
    );
}

#[tokio::test]
async fn mock_client_error_does_not_retry() {
    let server = MockServer::start(|_req, _i| {
        (400, br#"{"error":{"message":"bad size"}}"#.to_vec())
    });
    let client = live_client(server.url());
    let err = client
        .run_generations(&params("bad request", 1))
        .await
        .expect_err("400 must fail");
    match &err {
        RudderError::ApiError { status, body_summary } => {
            assert_eq!(*status, 400);
            assert!(body_summary.contains("bad size"), "summary: {body_summary}");
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
    assert_eq!(err.code(), "API_ERROR");
    assert_eq!(err.exit_code(), 2);
    assert_eq!(server.recorded().len(), 1, "4xx must not be retried");
}

#[tokio::test]
async fn mock_edits_sends_multipart_with_multiple_refs() {
    let server = MockServer::start(|_req, _i| (200, b64_response(&[PNG_BYTES_A])));
    let client = live_client(server.url());
    let anchor = temp_image("anchor", PNG_BYTES_A, "png");
    let r1 = temp_image("ref1", PNG_BYTES_B, "jpg");
    let refs = vec![
        ImageRef { role: "anchor".into(), path: anchor },
        ImageRef { role: "ref".into(), path: r1 },
    ];
    let out = client
        .run_edits(&params("page prompt", 2), &refs)
        .await
        .expect("mock edits succeeds");
    assert_eq!(out.images, Some(vec![PNG_BYTES_A.to_vec()]));

    let reqs = server.recorded();
    assert_eq!(reqs.len(), 1);
    let req = &reqs[0];
    assert_eq!(req.path, "/v1/images/edits");
    let content_type = req.header("content-type").expect("multipart content type").to_string();
    assert!(
        content_type.starts_with("multipart/form-data"),
        "got {content_type}"
    );
    let body = req.body_str();
    let image_parts = body.matches("name=\"image[]\"").count();
    assert_eq!(image_parts, 2, "anchor + one ref, got body:\n{body}");
    // Text fields travel alongside the images.
    for needle in [
        "name=\"model\"",
        "name=\"prompt\"",
        "name=\"size\"",
        "name=\"quality\"",
        "name=\"n\"",
        "name=\"seed\"",
        "name=\"thinking\"",
    ] {
        assert!(body.contains(needle), "multipart missing {needle}");
    }
    assert!(body.contains(MODEL));
    assert!(body.contains("page prompt"));
}

#[tokio::test]
async fn live_edits_without_key_fails_before_network() {
    let client = ImageClient::new(
        "http://127.0.0.1:9",
        None,
        false,
        Duration::from_millis(1),
        Duration::from_secs(5),
    )
    .unwrap();
    let anchor = temp_image("anchor", PNG_BYTES_A, "png");
    let err = client
        .run_edits(
            &params("p", 1),
            &[ImageRef { role: "anchor".into(), path: anchor }],
        )
        .await
        .expect_err("missing key must fail");
    assert_eq!(err.code(), "CREDENTIAL_MISSING");
    assert_eq!(err.exit_code(), 2);
}

#[tokio::test]
async fn live_edits_with_missing_ref_file_fails_before_network() {
    let client = live_client("http://127.0.0.1:9".into());
    let err = client
        .run_edits(
            &params("p", 1),
            &[ImageRef { role: "anchor".into(), path: PathBuf::from("/nonexistent/anchor.png") }],
        )
        .await
        .expect_err("missing anchor file must fail");
    assert_eq!(err.code(), "INVALID_ARG");
}

#[tokio::test]
async fn mock_unreachable_endpoint_maps_to_api_unreachable() {
    // Port 1 on localhost is reserved and refuses connections.
    let client = live_client("http://127.0.0.1:1".into());
    let err = client
        .run_generations(&params("p", 1))
        .await
        .expect_err("refused connection must fail");
    assert_eq!(err.code(), "API_UNREACHABLE");
    assert_eq!(err.exit_code(), 2);
}
