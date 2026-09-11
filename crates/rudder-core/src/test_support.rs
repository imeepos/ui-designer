//! Shared test infrastructure (test-only): a minimal HTTP/1.1 mock server
//! plus b64 helpers. Used by `image::tests` (protocol behavior) and
//! `ops::tests` (full-chain generation persistence). Not compiled outside
//! `cfg(test)`.

use base64::Engine as _;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Process-wide serialization for every test that mutates the process
/// environment (`RUDDER_KEYCHAIN`, `OPENAI_*`, …).
///
/// Env is a shared process global: two tests holding *module-local* locks
/// can still interleave `set_var`/`remove_var` with each other's reads —
/// that race produced FLAKE-1 (`cms_auth` kill-switch window observed a
/// concurrently removed `RUDDER_KEYCHAIN`). Every env-mutating guard must
/// hold THIS lock for its whole test body; `RUDDER_HOME` additionally stays
/// set-once (see `ops::tests::ensure_test_home`) and is never cleared in
/// tests, so reads without the lock remain stable.
pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone)]
pub(crate) struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>, // names lowercased
    pub body: Vec<u8>,
}

impl RecordedRequest {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub(crate) fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub(crate) struct MockServer {
    addr: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl MockServer {
    /// `responder(request, index) -> (status, body)` decides the canned reply.
    pub(crate) fn start(
        responder: impl Fn(&RecordedRequest, usize) -> (u16, Vec<u8>) + Send + Sync + 'static,
    ) -> MockServer {
        Self::start_with_headers(move |req, index| {
            let (status, body) = responder(req, index);
            (status, Vec::new(), body)
        })
    }

    /// Like [`MockServer::start`] but the responder also emits extra response
    /// headers (e.g. `Set-Cookie`) — `(status, headers, body)`.
    pub(crate) fn start_with_headers(
        responder: impl Fn(&RecordedRequest, usize) -> (u16, Vec<(String, String)>, Vec<u8>)
            + Send
            + Sync
            + 'static,
    ) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::new(Mutex::new(Vec::new()));
        let responder = Arc::new(responder);
        {
            let requests = requests.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { break };
                    let responder = responder.clone();
                    let requests = requests.clone();
                    std::thread::spawn(move || {
                        let Ok(Some(req)) = read_request(&mut stream) else { return };
                        let index = {
                            let mut guard = requests.lock().expect("request log lock");
                            guard.push(req.clone());
                            guard.len() - 1
                        };
                        let (status, headers, body) = responder(&req, index);
                        let _ = respond(&mut stream, status, &headers, &body);
                    });
                }
            });
        }
        MockServer { addr, requests }
    }

    pub(crate) fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub(crate) fn recorded(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("request log lock").clone()
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
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.push((name, value));
        }
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    Ok(Some(RecordedRequest { method, path, headers, body }))
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// A `{"data":[{"b64_json": …}, …]}` body carrying the given images.
pub(crate) fn b64_response(images: &[&[u8]]) -> Vec<u8> {
    let engine = base64::engine::general_purpose::STANDARD;
    let data: Vec<Value> = images
        .iter()
        .map(|b| json!({ "b64_json": engine.encode(b) }))
        .collect();
    json!({ "data": data }).to_string().into_bytes()
}
