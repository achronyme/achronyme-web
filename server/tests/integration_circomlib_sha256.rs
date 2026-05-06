//! E2E test for the `circomlib-sha256` template.
//!
//! Heaviest sibling of `integration_circomlib_demo.rs` and
//! `integration_circomlib_mimc.rs`: drives `/api/run` against a fresh
//! session populated with the SHA-256 preimage-bit template,
//! exercising circomlib's `Sha256(8)` (~28k R1CS constraints — one
//! full SHA-256 compression block) end-to-end through:
//!
//!   - `@circomlib` namespace resolution
//!   - `compiler.circom_lib_dirs` plumbing through `pipeline::run_inner`
//!   - the inlined-`<--` witness pipeline (every Num2Bits / sigma /
//!     compression-round hint is computed off-circuit by
//!     `circom::witness::compute_witness_hints` before R1CS witness
//!     generation)
//!   - the prove block, which emits a Groth16 artifact
//!
//! When the server prove handler skips `compute_witness_hints` the
//! prove block bails with `missing input: circom_call_X.<sub>.out_N`
//! because the inlined sub-component witness wires never get values.
//! That regression originally blocked shipping this template.

use std::path::PathBuf;
use std::sync::Once;

use ach_server::{build_app, session::SessionStore, AppConfig};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

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
    // SHA-256 proof JSON is comparable to MiMC's; bump the cap.
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

async fn create_blank_session(app: &axum::Router) -> String {
    let resp = app
        .clone()
        .oneshot(
            req()
                .method(Method::POST)
                .uri("/api/session/create")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    body["session_id"].as_str().expect("session_id").to_string()
}

async fn write_file(app: &axum::Router, session_id: &str, path: &str, content: &str) {
    let payload = json!({ "path": path, "content": content });
    let resp = app
        .clone()
        .oneshot(
            req()
                .method(Method::POST)
                .uri("/api/fs/write")
                .header("content-type", "application/json")
                .header("x-ach-session", session_id)
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "fs/write({path}) returned {}",
        resp.status()
    );
}

async fn create_sha256_session(app: &axum::Router) -> String {
    let payload = json!({ "template": "circomlib-sha256" });
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
async fn circomlib_sha256_template_runs_end_to_end() {
    ensure_circomlib_env();

    // Fail loudly if the operator forgot `git submodule update`.
    let circomlib = PathBuf::from(std::env::var("ACH_CIRCOMLIB_PATH").unwrap());
    assert!(
        circomlib.join("sha256/sha256.circom").exists(),
        "vendor/circomlib/circuits/sha256/sha256.circom is missing — \
         run `git submodule update --init --recursive`"
    );

    let app = make_app();
    let session = create_sha256_session(&app).await;

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
        "circomlib-sha256 run reported failure: {body}"
    );

    let output = body["output"].as_str().unwrap_or_default();
    assert!(
        output.contains("Public SHA-256 bit 0:"),
        "expected public-bit print line, got: {output}"
    );
    assert!(
        output.contains("Witness verified"),
        "expected post-prove confirmation, got: {output}"
    );

    // Prove block ran, so a Groth16 artifact must be in the response.
    let proofs = body["proofs"].as_array().expect("proofs array");
    assert_eq!(
        proofs.len(),
        1,
        "expected exactly one proof, got {proofs:?}"
    );
    // Plonkish shares the same `compute_witness_hints` step (gated
    // before the R1CS / Plonkish split in `ServerProveHandler`), so a
    // separate Plonkish run only re-tests the witness-hint plumbing
    // for ~5s of compile time. The R1CS run pins the bug; the
    // template ships R1CS-first per playground default backend.
    assert_eq!(proofs[0]["backend"], "r1cs", "expected r1cs backend");
}

/// Inline-call shape pin: `Sha256(N)` invoked directly inside a prove
/// block (no wrapper template), mirroring the cli regression test
/// `cli/tests/sha256_via_ach_prove.rs::sha256_8_compiles`. Keeps the
/// witness-hint regression visible even if the template's wrapper
/// shape changes — the wrapper-based test above can mask the bug if
/// the Sha256 sub-component naming inside the wrapper shifts.
const INLINE_ACHRONYME_TOML: &str = r#"[project]
name = "sha256_inline"
version = "0.1.0"
entry = "src/main.ach"

[build]
backend = "r1cs"

[circom]
libs = ["@circomlib"]
"#;

const INLINE_SHA_CIRCOM: &str = r#"pragma circom 2.0.0;

include "sha256/sha256.circom";

template Sha256BitArr() {
    signal input in[8];
    signal output out;

    component s = Sha256(8);
    for (var i = 0; i < 8; i++) {
        s.in[i] <== in[i];
    }
    out <== s.out[0];
}
"#;

const INLINE_MAIN_ACH: &str = r#"import { Sha256BitArr } from "./sha.circom"

prove() {
    let _r = Sha256BitArr()([0p0, 0p1, 0p0, 0p1, 0p0, 0p1, 0p0, 0p1])
}
"#;

#[tokio::test]
async fn sha256_inline_call_in_prove_block_succeeds() {
    ensure_circomlib_env();

    let circomlib = PathBuf::from(std::env::var("ACH_CIRCOMLIB_PATH").unwrap());
    assert!(
        circomlib.join("sha256/sha256.circom").exists(),
        "vendor/circomlib/circuits/sha256/sha256.circom is missing — \
         run `git submodule update --init --recursive`"
    );

    let app = make_app();
    let session = create_blank_session(&app).await;
    write_file(&app, &session, "achronyme.toml", INLINE_ACHRONYME_TOML).await;
    write_file(&app, &session, "src/sha.circom", INLINE_SHA_CIRCOM).await;
    write_file(&app, &session, "src/main.ach", INLINE_MAIN_ACH).await;

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
    assert_eq!(resp.status(), StatusCode::OK, "run returned {}", resp.status());

    let body = read_json(resp).await;
    assert_eq!(
        body["success"], true,
        "Sha256(8) inline prove block reported failure: {body}"
    );
}
