//! Behaviour tests for the search client and the picker.
//!
//! The tests serve hand made response fixtures from a stub server and
//! assert which asset IDs reach the picker selection logic.

use demo_picker::{BASE_URL_ENV, ImmichClient, Picker, parse_search_response};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// One request captured by the stub server.
#[derive(Debug)]
struct SeenRequest {
    /// Request target path.
    path: String,
    /// Raw request body.
    body: String,
}

/// Locate the end of the HTTP headers in the buffered bytes.
fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

/// Read one HTTP request and split the path from the body.
fn read_request(stream: &mut TcpStream) -> SeenRequest {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        let count = stream.read(&mut chunk).expect("read stub headers");
        raw.extend_from_slice(&chunk[..count]);
        if let Some(end) = find_header_end(&raw) {
            break end;
        }
    };
    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let mut lines = head.lines();
    let target = lines.next().expect("read stub request line");
    let path = target
        .split_whitespace()
        .nth(1)
        .expect("read stub path")
        .to_string();
    let mut length = 0usize;
    for line in lines {
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().expect("read stub length");
        }
    }
    let mut body = raw[header_end..].to_vec();
    while body.len() < length {
        let count = stream.read(&mut chunk).expect("read stub body");
        body.extend_from_slice(&chunk[..count]);
    }
    body.truncate(length);
    SeenRequest {
        path,
        body: String::from_utf8_lossy(&body).to_string(),
    }
}

/// Serve the canned bodies in order and capture each request.
fn spawn_stub(bodies: Vec<String>) -> (String, Arc<Mutex<Vec<SeenRequest>>>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub listener");
    let addr = listener.local_addr().expect("read stub address");
    let seen: Arc<Mutex<Vec<SeenRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::clone(&seen);
    let handle = thread::spawn(move || {
        for body in bodies {
            let (mut stream, _) = listener.accept().expect("accept stub connection");
            let request = read_request(&mut stream);
            shared.lock().expect("record stub request").push(request);
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(reply.as_bytes())
                .expect("write stub reply");
        }
    });
    (format!("http://{addr}"), seen, handle)
}

/// Build a response fixture holding the given asset IDs.
fn fixture(ids: &[&str]) -> String {
    let items: Vec<String> = ids
       .iter()
        .enumerate()
        .map(|(index, id)| {
            format!(
                "{{\"id\": \"{id}\", \"deviceAssetId\": \"device-{index}\", \"deviceId\": \"phone\", \"type\": \"IMAGE\", \"originalFileName\": \"photo-{index}.jpg\", \"createdAt\": \"2024-01-01T00:00:00Z\"}}"
            )
        })
        .collect();
    format!(
        "{{\"albums\": {{\"total\": 0, \"count\": 0, \"items\": []}}, \"assets\": {{\"total\": {}, \"count\": {}, \"items\": [{}]}}}}",
        ids.len(),
        ids.len(),
        items.join(", ")
    )
}

/// Collect the picker selection as plain string slices.
fn selected(picker: &Picker) -> Vec<&str> {
    picker.selection().iter().map(String::as_str).collect()
}

#[test]
fn random_search_feeds_picker() {
    let (base, seen, handle) = spawn_stub(vec![fixture(&["asset-r1", "asset-r2"])]);
    let client = ImmichClient::new(&base);
    let response = client.search_random(1).expect("run random search");
    let mut picker = Picker::new();
    picker.add_response(&response);
    assert_eq!(
        selected(&picker),
        ["asset-r1", "asset-r2"],
        "random picker ids"
    );
    let seen = seen.lock().expect("read stub requests");
    assert_eq!(seen.len(), 1, "random request count");
    assert_eq!(seen[0].path, "/search/random", "random request path");
    assert!(
        seen[0].body.contains("\"page\":1"),
        "random request sends page"
    );
    handle.join().expect("stop stub server");
}

#[test]
fn smart_search_feeds_picker() {
    let (base, seen, handle) = spawn_stub(vec![fixture(&["asset-s1"])]);
    let client = ImmichClient::new(&base);
    let response = client.search_smart("sunset").expect("run smart search");
    let mut picker = Picker::new();
    picker.add_response(&response);
    assert_eq!(selected(&picker), ["asset-s1"], "smart picker ids");
    let seen = seen.lock().expect("read stub requests");
    assert_eq!(seen.len(), 1, "smart request count");
    assert_eq!(seen[0].path, "/search/smart", "smart request path");
    assert!(seen[0].body.contains("sunset"), "smart request sends query");
    handle.join().expect("stop stub server");
}

#[test]
fn metadata_search_feeds_picker() {
    let (base, seen, handle) = spawn_stub(vec![fixture(&["asset-m1", "asset-m2", "asset-m3"])]);
    let client = ImmichClient::new(&base);
    let response = client
        .search_metadata("photo-0.jpg")
        .expect("run metadata search");
    let mut picker = Picker::new();
    picker.add_response(&response);
    assert_eq!(
        selected(&picker),
        ["asset-m1", "asset-m2", "asset-m3"],
        "metadata picker ids"
    );
    let seen = seen.lock().expect("read stub requests");
    assert_eq!(seen.len(), 1, "metadata request count");
    assert_eq!(seen[0].path, "/search/metadata", "metadata request path");
    let body: serde_json::Value =
        serde_json::from_str(&seen[0].body).expect("parse metadata request body");
    assert_eq!(
        body.get("originalFileName")
            .and_then(|value| value.as_str()),
        Some("photo-0.jpg"),
        "metadata request sends file name"
    );
    assert!(
        body.get("query").is_none(),
        "metadata request has no unknown query field"
    );
    handle.join().expect("stop stub server");
}

#[test]
fn picker_more_pages_random_results() {
    let (base, seen, handle) = spawn_stub(vec![
        fixture(&["asset-p1"]),
        fixture(&["asset-p2", "asset-p3"]),
    ]);
    let client = ImmichClient::new(&base);
    let mut picker = Picker::new();
    assert_eq!(picker.next_page(), 1, "picker first page");
    picker.more(&client).expect("fetch first page");
    picker.more(&client).expect("fetch second page");
    assert_eq!(
        selected(&picker),
        ["asset-p1", "asset-p2", "asset-p3"],
        "picker more ids"
    );
    assert_eq!(picker.next_page(), 3, "picker next page");
    let seen = seen.lock().expect("read stub requests");
    assert_eq!(seen.len(), 2, "picker more request count");
    assert!(
        seen[0].body.contains("\"page\":1"),
        "picker first page request"
    );
    assert!(
        seen[1].body.contains("\"page\":2"),
        "picker second page request"
    );
    handle.join().expect("stop stub server");
}

#[test]
fn parse_fixture_file_from_disk() {
    let mut file = tempfile::NamedTempFile::new().expect("create fixture file");
    file.write_all(fixture(&["asset-f1"]).as_bytes())
        .expect("write fixture file");
    let body = std::fs::read_to_string(file.path()).expect("read fixture file");
    let response = parse_search_response(&body).expect("parse fixture file");
    let mut picker = Picker::new();
    picker.add_response(&response);
    assert_eq!(selected(&picker), ["asset-f1"], "file fixture picker ids");
}

#[test]
fn parse_rejects_invalid_body() {
    let result = parse_search_response("not json");
    assert!(result.is_err(), "invalid body errors");
}

#[test]
fn parse_reads_album_title() {
    let body = "{\"albums\": {\"count\": 1, \"items\": [{\"id\": \"a1\", \"albumName\": \"Trip\"}]}, \"assets\": {\"count\": 0, \"items\": []}}";
    let response = parse_search_response(body).expect("parse album title");
    assert_eq!(response.albums.items.len(), 1, "album item count");
    assert_eq!(response.albums.items[0].name, "Trip", "album title");
}

#[test]
fn client_from_env_reads_base_url() {
    // No other test touches this variable so the write is safe.
    unsafe {
        std::env::set_var(BASE_URL_ENV, "http://127.0.0.1:9999/");
    }
    let client = ImmichClient::from_env().expect("build client from environment");
    assert_eq!(client.base_url(), "http://127.0.0.1:9999", "env base url");
    // No other test touches this variable so the removal is safe.
    unsafe {
        std::env::remove_var(BASE_URL_ENV);
    }
}
