//! ach-server library entry point.
//!
//! The binary's `main` is a thin shell over this crate: it parses env
//! config and hands off to [`build_app`], which returns a fully wired
//! `Router`. Integration tests in `tests/` drive the same router via
//! `tower::ServiceExt::oneshot`, so regressions in rate limiting, body
//! caps, CORS, and path sanitization surface during `cargo test`
//! without needing a live TCP listener.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method};
use axum::routing::{delete, get, post};
use axum::Router;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;

pub mod error;
pub mod pipeline;
pub mod prove_handler;
pub mod routes;
pub mod sandbox;
pub mod sanitize;
pub mod session;
pub mod templates;
pub mod workspace;

// Per-endpoint body limits. The playground never needs to POST more than a
// few KB of source; tight caps mean a hostile client can't tie up the
// 35s sandbox timeout by streaming megabytes into a CPU-heavy endpoint.
pub const BODY_PROVE: usize = 16 * 1024;
pub const BODY_COMPILE: usize = 64 * 1024;
pub const BODY_INSPECT: usize = 64 * 1024;
pub const BODY_RUN: usize = 64 * 1024;
pub const BODY_CIRCUIT: usize = 32 * 1024;
pub const BODY_FORMAT: usize = 128 * 1024;
pub const BODY_FS_WRITE: usize = 256 * 1024;
pub const BODY_DEFAULT: usize = 16 * 1024;

/// Tuning knobs for [`build_app`]. Production reads these from env vars
/// inside `main`; tests construct them directly.
pub struct AppConfig {
    /// Exact origin string allowed by CORS. No wildcards.
    pub cors_origin: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            cors_origin: "http://localhost:4321".to_string(),
        }
    }
}

/// Build the fully-wired Axum router. Shared between `main` and tests.
///
/// Returns the router on its own so the caller decides how to serve it
/// (real TCP listener in prod, `ServiceExt::oneshot` in tests).
pub fn build_app(store: session::SessionStore, cfg: &AppConfig) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            cfg.cors_origin
                .parse::<HeaderValue>()
                .expect("invalid CORS origin"),
        )
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, "X-Ach-Session".parse().unwrap()])
        .expose_headers(["X-Ach-Session".parse().unwrap()]);

    // ── Rate limiting ─────────────────────────────────────────────────────
    //
    // Two buckets, both keyed by client IP (SmartIpKeyExtractor reads
    // X-Forwarded-For so we get the real caller even behind nginx):
    //
    //   heavy  — prove/circuit: CPU-bound, can saturate the prover. Cap
    //            at burst 3, then 1 request every 12s (~5/min sustained).
    //   normal — everything else. Burst 30, then 1 every 2s (~30/min).
    //
    // 429s carry a `Retry-After` header automatically.
    let heavy_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(12)
            .burst_size(3)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid heavy governor config"),
    );
    let normal_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(30)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid normal governor config"),
    );

    // Periodically drop stale bucket entries so memory doesn't grow with
    // churn from one-shot clients.
    let heavy_limiter = heavy_conf.limiter().clone();
    let normal_limiter = normal_conf.limiter().clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.tick().await; // skip the immediate first tick
        loop {
            ticker.tick().await;
            heavy_limiter.retain_recent();
            normal_limiter.retain_recent();
        }
    });

    // ── Heavy routes: tight rate limit, tight body cap ────────────────────
    let heavy_routes = Router::new()
        .route(
            "/api/prove",
            post(routes::prove::handler).layer(DefaultBodyLimit::max(BODY_PROVE)),
        )
        .route(
            "/api/circuit",
            post(routes::circuit::handler).layer(DefaultBodyLimit::max(BODY_CIRCUIT)),
        )
        .layer(GovernorLayer::new(heavy_conf));

    // ── Normal routes: relaxed rate limit, per-endpoint body cap ──────────
    let normal_routes = Router::new()
        .route(
            "/api/run",
            post(routes::run::handler).layer(DefaultBodyLimit::max(BODY_RUN)),
        )
        .route(
            "/api/compile",
            post(routes::compile::handler).layer(DefaultBodyLimit::max(BODY_COMPILE)),
        )
        .route(
            "/api/inspect",
            post(routes::inspect::handler).layer(DefaultBodyLimit::max(BODY_INSPECT)),
        )
        .route(
            "/api/format",
            post(routes::format::handler).layer(DefaultBodyLimit::max(BODY_FORMAT)),
        )
        .route(
            "/api/session/create",
            post(routes::session::create).layer(DefaultBodyLimit::max(BODY_DEFAULT)),
        )
        .route("/api/session", delete(routes::session::delete))
        .route(
            "/api/fs/write",
            post(routes::fs::write).layer(DefaultBodyLimit::max(BODY_FS_WRITE)),
        )
        .route(
            "/api/fs/read",
            post(routes::fs::read).layer(DefaultBodyLimit::max(BODY_DEFAULT)),
        )
        .route(
            "/api/fs/delete",
            post(routes::fs::delete).layer(DefaultBodyLimit::max(BODY_DEFAULT)),
        )
        .route("/api/fs/list", get(routes::fs::list))
        .route(
            "/api/fs/rename",
            post(routes::fs::rename).layer(DefaultBodyLimit::max(BODY_DEFAULT)),
        )
        .route(
            "/api/fs/mkdir",
            post(routes::fs::mkdir).layer(DefaultBodyLimit::max(BODY_DEFAULT)),
        )
        .layer(GovernorLayer::new(normal_conf));

    Router::new()
        .merge(heavy_routes)
        .merge(normal_routes)
        // /health intentionally outside both rate limiters so uptime probes
        // don't get throttled.
        .route("/health", get(|| async { "ok" }))
        .with_state(store)
        .layer(cors)
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(35),
        ))
}
