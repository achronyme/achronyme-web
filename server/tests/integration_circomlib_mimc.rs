//! E2E test for the `circomlib-mimc` template.
//!
//! Mid-weight sibling of `integration_circomlib_demo.rs`: drives
//! `/api/run` against a fresh session populated with the MiMC
//! preimage template, exercising circomlib's `MiMCSponge(2, 220, 1)`
//! (~3,087 R1CS constraints — the same shape Tornado Cash uses for
//! its Merkle commitments) end-to-end through:
//!
//!   - `@circomlib` namespace resolution + `OnceLock` startup canon
//!   - `compiler.circom_lib_dirs` plumbing through `pipeline::run_inner`
//!   - the .ach circom witness handler (CallCircomTemplate dispatch)
//!   - the prove block, which emits a Groth16 artifact
//!
//! Constraint count is ~12× the Poseidon demo's; if the lowering or
//! witness path regresses on heavier primitives, this test will
//! catch it before the playground does.

use std::path::PathBuf;
use std::sync::Once;

use ach_server::{build_app, session::SessionStore, AppConfig};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Point `ACH_CIRCOMLIB_PATH` at the in-tree submodule before any test
/// triggers `circomlib_path()` to initialize its `OnceLock`. Each
/// `tests/*.rs` file compiles to its own binary, so the only readers
/// of this env var live inside this file.
fn ensure_circomlib_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if std::env::var_os("ACH_CIRCOMLIB_PATH").is_some() {
            return;
        }
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let circomlib = manifest
            .parent()
            .expect("server/ has a parent")
            .join("vendor/circomlib/circuits");
        std::env::set_var("ACH_CIRCOMLIB_PATH", circomlib);
    });
}

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
    // MiMC proof JSON is larger than Poseidon's; bump the cap.
    let body = to_bytes(resp.into_body(), 16 * 1024 * 1024)
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

async fn create_mimc_session(app: &axum::Router) -> String {
    let payload = json!({ "template": "circomlib-mimc" });
    let resp = app
        .clone()
        .oneshot(
            req()
                .method(Method::POST)
                .uri("/api/session/create")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "session/create returned {}",
        resp.status()
    );
    let body = read_json(resp).await;

    let files: Vec<String> = body["files"]
        .as_array()
        .expect("files array")
        .iter()
        .filter_map(|v| v["path"].as_str().map(str::to_string))
        .collect();
    for required in ["achronyme.toml", "src/main.ach", "src/hash.circom"] {
        assert!(
            files.iter().any(|p| p == required),
            "missing {required} in session: {files:?}"
        );
    }

    body["session_id"].as_str().expect("session_id").to_string()
}

#[tokio::test]
async fn circomlib_mimc_template_runs_end_to_end() {
    ensure_circomlib_env();

    // Fail loudly if the operator forgot `git submodule update`.
    let circomlib = PathBuf::from(std::env::var("ACH_CIRCOMLIB_PATH").unwrap());
    assert!(
        circomlib.join("mimcsponge.circom").exists(),
        "vendor/circomlib/circuits/mimcsponge.circom is missing — \
         run `git submodule update --init --recursive`"
    );

    let app = make_app();
    let session = create_mimc_session(&app).await;

    let start = std::time::Instant::now();
    let resp = app
        .clone()
        .oneshot(
            req()
                .method(Method::POST)
                .uri("/api/run")
                .header("content-type", "application/json")
                .header("x-ach-session", &session)
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let elapsed = start.elapsed();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "run returned {} after {:?}",
        resp.status(),
        elapsed
    );

    let body = read_json(resp).await;
    assert_eq!(
        body["success"], true,
        "circomlib-mimc run reported failure: {body}"
    );

    let output = body["output"].as_str().unwrap_or_default();
    assert!(
        output.contains("Public MiMC(a, b)"),
        "expected MiMC hash print line, got: {output}"
    );
    assert!(
        output.contains("Witness verified"),
        "expected post-prove confirmation, got: {output}"
    );

    eprintln!("circomlib-mimc E2E completed in {elapsed:?}");
}
