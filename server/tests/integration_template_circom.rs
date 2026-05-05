//! E2E test for the `circom` tutorial template.
//!
//! Verifies that requesting `template=circom` in `/api/session/create`
//! populates a workspace whose `main.ach` actually compiles + runs:
//! the `.ach` pipeline imports the sibling `.circom` file via a
//! relative path, dispatches `Square()(secret)` through the
//! CallCircomTemplate opcode in VM mode, then enters the prove block
//! and emits a Groth16 artifact.
//!
//! If this test breaks, the playground's "Template: Circom (E2E)"
//! option is probably broken too — they share the populate logic.

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

async fn create_circom_template_session(app: &axum::Router) -> String {
    let payload = json!({ "template": "circom" });
    let request = req()
        .method(Method::POST)
        .uri("/api/session/create")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .expect("valid request");
    let resp = app.clone().oneshot(request).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "session/create returned {}",
        resp.status()
    );
    let body = read_json(resp).await;

    // Sanity-check the populate step landed both files where main.ach
    // expects them.
    let files: Vec<String> = body["files"]
        .as_array()
        .expect("files array")
        .iter()
        .filter_map(|v| v["path"].as_str().map(str::to_string))
        .collect();
    assert!(
        files.iter().any(|p| p == "src/main.ach"),
        "missing src/main.ach in session: {files:?}"
    );
    assert!(
        files.iter().any(|p| p == "src/square.circom"),
        "missing src/square.circom in session: {files:?}"
    );

    body["session_id"]
        .as_str()
        .expect("session_id in response")
        .to_string()
}

#[tokio::test]
async fn circom_template_runs_end_to_end() {
    let app = make_app();
    let session = create_circom_template_session(&app).await;

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
        StatusCode::OK,
        "run returned {}",
        resp.status()
    );

    let body = read_json(resp).await;
    assert_eq!(
        body["success"], true,
        "run reported failure for circom template: {body}"
    );

    // VM-mode evaluation must execute Square(7) = 49 and the prove
    // block must succeed; both messages are inside main.ach.
    let output = body["output"].as_str().unwrap_or_default();
    assert!(
        output.contains("Public square:"),
        "expected VM-mode print line, got: {output}"
    );
    assert!(
        output.contains("Witness verified"),
        "expected post-prove confirmation, got: {output}"
    );
}
