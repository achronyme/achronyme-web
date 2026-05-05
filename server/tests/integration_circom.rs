//! Integration tests for `.circom` workspace mode.
//!
//! Drives the same Router as `integration.rs` via `oneshot`. Each test
//! creates a fresh session, writes a tiny project (achronyme.toml,
//! src/main.circom, and a fake circomlib subset), and exercises one
//! route. The fixtures are inlined as string literals so the repo
//! doesn't need to vendor circomlib for the test suite to run.

use ach_server::{build_app, session::SessionStore, AppConfig};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn make_app() -> axum::Router {
    let store = SessionStore::new();
    let cfg = AppConfig {
        cors_origin: "http://localhost:4321".into(),
    };
    build_app(store, &cfg)
}

fn req() -> axum::http::request::Builder {
    Request::builder().header("x-forwarded-for", "127.0.0.1")
}

async fn read_json(resp: axum::response::Response) -> Value {
    let body = to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("body")
        .to_vec();
    serde_json::from_slice(&body).unwrap_or_else(|e| {
        panic!(
            "response was not valid JSON: {e}\n--- body ---\n{}",
            String::from_utf8_lossy(&body)
        )
    })
}

/// Stand-alone Num2Bits — minimal `bitify.circom` substitute.
const FAKE_BITIFY: &str = r#"
pragma circom 2.0.0;

template Num2Bits(n) {
    signal input in;
    signal output out[n];
    var lc1 = 0;
    var e2 = 1;
    for (var i = 0; i < n; i++) {
        out[i] <-- (in >> i) & 1;
        out[i] * (out[i] - 1) === 0;
        lc1 += out[i] * e2;
        e2 = e2 + e2;
    }
    lc1 === in;
}
"#;

/// Top-level circuit that includes the fake circomlib copy and uses Num2Bits.
/// Unqualified `include` matches circom's library-dir search convention —
/// the resolver looks under each `[circom] libs` entry until it finds the
/// file. `include "bitify.circom"` + `libs = ["circomlib/circuits"]`
/// resolves to `circomlib/circuits/bitify.circom` in the workspace.
const MAIN_CIRCOM: &str = r#"
pragma circom 2.0.0;
include "bitify.circom";
component main = Num2Bits(8);
"#;

const ACHRONYME_TOML: &str = r#"
[project]
entry = "src/main.circom"

[circom]
libs = ["circomlib/circuits"]
"#;

/// Helper: create a session, return its UUID.
async fn create_session(app: &axum::Router) -> String {
    let request = req()
        .method(Method::POST)
        .uri("/api/session/create")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    body["session_id"]
        .as_str()
        .expect("session_id in response")
        .to_string()
}

/// Helper: write a single file into the session workspace.
async fn write_file(app: &axum::Router, session_id: &str, path: &str, content: &str) {
    let payload = json!({ "path": path, "content": content });
    let request = req()
        .method(Method::POST)
        .uri("/api/fs/write")
        .header("content-type", "application/json")
        .header("x-ach-session", session_id)
        .body(Body::from(payload.to_string()))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert!(
        resp.status().is_success(),
        "fs/write({path}) returned {}",
        resp.status()
    );
}

/// Helper: bootstrap a session with the standard Num2Bits fixture.
async fn setup_circom_workspace(app: &axum::Router) -> String {
    let session = create_session(app).await;
    write_file(app, &session, "achronyme.toml", ACHRONYME_TOML).await;
    write_file(
        app,
        &session,
        "circomlib/circuits/bitify.circom",
        FAKE_BITIFY,
    )
    .await;
    write_file(app, &session, "src/main.circom", MAIN_CIRCOM).await;
    session
}

#[tokio::test]
async fn compile_circom_workspace_succeeds() {
    let app = make_app();
    let session = setup_circom_workspace(&app).await;

    let request = req()
        .method(Method::POST)
        .uri("/api/compile")
        .header("content-type", "application/json")
        .header("x-ach-session", &session)
        .body(Body::from("{}"))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "compile failed with status {}",
        resp.status()
    );

    let body = read_json(resp).await;
    assert_eq!(
        body["success"], true,
        "compile reported failure: {}",
        body
    );
}

#[tokio::test]
async fn circuit_circom_workspace_produces_r1cs() {
    let app = make_app();
    let session = setup_circom_workspace(&app).await;

    // Num2Bits(8) needs `in` as input. Decimal value < 256 → fits in 8 bits.
    let payload = json!({
        "inputs": { "in": "13" },
        "backend": "r1cs"
    });
    let request = req()
        .method(Method::POST)
        .uri("/api/circuit")
        .header("content-type", "application/json")
        .header("x-ach-session", &session)
        .body(Body::from(payload.to_string()))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "circuit failed with status {}",
        resp.status()
    );

    let body = read_json(resp).await;
    assert_eq!(body["success"], true, "circuit reported failure: {}", body);
    assert_eq!(body["backend"], "r1cs");
    let constraints = body["constraints"].as_u64().expect("constraints field");
    assert!(constraints > 0, "expected non-zero R1CS constraints");
    assert!(
        body["r1cs"].is_string(),
        "expected base64 R1CS payload in response"
    );
}

#[tokio::test]
async fn compile_circom_returns_diagnostic_on_invalid_template() {
    let app = make_app();
    let session = create_session(&app).await;

    write_file(&app, &session, "achronyme.toml", ACHRONYME_TOML).await;
    // Empty bitify.circom → Num2Bits is undefined when main tries to use it.
    write_file(&app, &session, "circomlib/circuits/bitify.circom", "").await;
    write_file(&app, &session, "src/main.circom", MAIN_CIRCOM).await;

    let request = req()
        .method(Method::POST)
        .uri("/api/compile")
        .header("content-type", "application/json")
        .header("x-ach-session", &session)
        .body(Body::from("{}"))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = read_json(resp).await;
    assert_eq!(body["success"], false, "expected compile failure");
    let diags = body["diagnostics"]
        .as_array()
        .expect("diagnostics array in response");
    assert!(
        !diags.is_empty(),
        "expected at least one diagnostic for missing template"
    );
}

#[tokio::test]
async fn run_rejects_circom_workspace() {
    let app = make_app();
    let session = setup_circom_workspace(&app).await;

    let request = req()
        .method(Method::POST)
        .uri("/api/run")
        .header("content-type", "application/json")
        .header("x-ach-session", &session)
        .body(Body::from("{}"))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "expected 400 for /api/run on .circom workspace"
    );

    let body = read_json(resp).await;
    let msg = body["error"].as_str().unwrap_or("");
    assert!(
        msg.contains("circom") || msg.contains("not supported"),
        "expected error to mention circom unsupported: {msg}"
    );
}

#[tokio::test]
async fn compile_rejects_workspace_with_absolute_libs() {
    // achronyme.toml is user-controlled; an absolute lib path must be
    // rejected at workspace-config-load time, before circom is invoked.
    let app = make_app();
    let session = create_session(&app).await;

    let bad_toml = r#"
[project]
entry = "src/main.circom"

[circom]
libs = ["/etc"]
"#;
    write_file(&app, &session, "achronyme.toml", bad_toml).await;
    write_file(&app, &session, "src/main.circom", MAIN_CIRCOM).await;

    let request = req()
        .method(Method::POST)
        .uri("/api/compile")
        .header("content-type", "application/json")
        .header("x-ach-session", &session)
        .body(Body::from("{}"))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "expected 400 for absolute libs path"
    );

    let body = read_json(resp).await;
    let msg = body["error"].as_str().unwrap_or("");
    assert!(
        msg.contains("relative") || msg.contains("circom"),
        "expected error to mention path validation: {msg}"
    );
}
