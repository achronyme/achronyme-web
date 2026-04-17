//! Integration tests for ach-server.
//!
//! These tests drive the Router returned by `ach_server::build_app`
//! directly via `tower::ServiceExt::oneshot`, so we get real handler
//! wiring, body-limit layers, CORS, and error sanitization — without
//! opening a TCP listener or hitting the real prover pipeline.
//!
//! Tests that would depend on wall-clock time (rate limiting burst
//! behaviour, reaper TTL) are intentionally out of scope here. They
//! belong in a benchmarks / soak suite, not unit-grade CI.

use ach_server::{build_app, session::SessionStore, AppConfig, BODY_PROVE};
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

/// Convenience: build a fresh Router with a fresh session store for each
/// test so state doesn't bleed across cases.
fn make_app() -> axum::Router {
    let store = SessionStore::new();
    // Don't spawn the reaper — the test process exits before the first
    // tick would fire and the DashMap is empty anyway.
    let cfg = AppConfig {
        cors_origin: "http://localhost:4321".into(),
    };
    build_app(store, &cfg)
}

/// Start a `Request::builder` with an `X-Forwarded-For` header set so
/// tower-governor's `SmartIpKeyExtractor` has a key to rate-limit on.
/// In production nginx populates this; in tests we provide it ourselves.
fn req() -> axum::http::request::Builder {
    Request::builder().header("x-forwarded-for", "127.0.0.1")
}

#[tokio::test]
async fn health_returns_ok() {
    let app = make_app();
    let req = req()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .expect("valid request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body(), 128)
        .await
        .expect("body")
        .to_vec();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = make_app();
    let req = req()
        .method(Method::GET)
        .uri("/does-not-exist")
        .body(Body::empty())
        .expect("valid request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn prove_rejects_oversize_body_with_413() {
    // `/api/prove` has a 16 KB body cap (BODY_PROVE). Send 32 KB and
    // expect the layer to reject before the handler is entered — the
    // response should be 413 Payload Too Large.
    let app = make_app();
    let huge = vec![b'{'; BODY_PROVE * 2];
    let req = req()
        .method(Method::POST)
        .uri("/api/prove")
        .header("content-type", "application/json")
        // Claim the real size so axum's body limit triggers synchronously.
        .header("content-length", huge.len().to_string())
        .body(Body::from(huge))
        .expect("valid request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(
        resp.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "POST /api/prove with >BODY_PROVE bytes must be rejected by the \
         per-route body cap, not reach the handler."
    );
}

#[tokio::test]
async fn cors_preflight_allows_known_origin() {
    // OPTIONS preflight from the configured origin should succeed and
    // echo the allow-origin header back. This is what the browser
    // expects before it'll let the Astro dev server talk to the API.
    let app = make_app();
    let req = req()
        .method(Method::OPTIONS)
        .uri("/api/run")
        .header("origin", "http://localhost:4321")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type")
        .body(Body::empty())
        .expect("valid request");

    let resp = app.oneshot(req).await.expect("oneshot");
    assert!(
        resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT,
        "CORS preflight should succeed (got {})",
        resp.status()
    );
    let allow_origin = resp.headers().get("access-control-allow-origin");
    assert_eq!(
        allow_origin.and_then(|h| h.to_str().ok()),
        Some("http://localhost:4321"),
        "expected access-control-allow-origin to echo the configured CORS origin"
    );
}

#[tokio::test]
async fn bad_request_scrubs_session_path_from_error() {
    // Create a session, then hit /api/run with an invalid JSON body and
    // the X-Ach-Session header pointing at a non-existent session.
    // The BadRequest we get back should not contain `/tmp/ach-sessions/`.
    let app = make_app();

    let req = req()
        .method(Method::POST)
        .uri("/api/run")
        .header("content-type", "application/json")
        .header("x-ach-session", "ffffffff-ffff-ffff-ffff-ffffffffffff")
        .body(Body::from("{}"))
        .expect("valid request");

    let resp = app.oneshot(req).await.expect("oneshot");
    // BadRequest either from session-not-found or from empty body.
    // Either way, the message body should not contain the internal tmp prefix.
    let body = to_bytes(resp.into_body(), 8192)
        .await
        .expect("body")
        .to_vec();
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        !body_str.contains("/tmp/ach-sessions/"),
        "error body leaked internal path: {body_str}"
    );
}

#[tokio::test]
async fn health_is_exempt_from_rate_limiting() {
    // Fire ten rapid GETs at /health; every one should return 200.
    // If the caller accidentally wires /health under a governor layer
    // this test catches it before prod uptime probes start getting 429s.
    let app = make_app();
    for i in 0..10 {
        let request = req()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .expect("valid request");
        let resp = app.clone().oneshot(request).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "/health request #{i} unexpectedly returned {}",
            resp.status()
        );
    }
}
