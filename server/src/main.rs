//! ach-server — Achronyme Server-Side Compilation backend.
//!
//! Provides HTTP endpoints for compiling, running, and inspecting
//! Achronyme programs.  Powers the web playground at achrony.me.
//!
//! Usage:
//!   ACH_PORT=3100 ACH_CORS_ORIGIN=https://achrony.me cargo run

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, Method};
use axum::routing::{delete, get, post};
use axum::Router;
use tokio::net::TcpListener;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;

mod error;
mod pipeline;
mod prove_handler;
mod routes;
mod sandbox;
mod sanitize;
mod session;
mod templates;
mod workspace;

// Per-endpoint body limits. The playground never needs to POST more than a
// few KB of source; tight caps mean a hostile client can't tie up the
// 35s sandbox timeout by streaming megabytes into a CPU-heavy endpoint.
const BODY_PROVE: usize = 16 * 1024;
const BODY_COMPILE: usize = 64 * 1024;
const BODY_INSPECT: usize = 64 * 1024;
const BODY_RUN: usize = 64 * 1024;
const BODY_CIRCUIT: usize = 32 * 1024;
const BODY_FORMAT: usize = 128 * 1024;
const BODY_FS_WRITE: usize = 256 * 1024;
const BODY_DEFAULT: usize = 16 * 1024;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ach_server=info".into()),
        )
        .init();

    let port: u16 = env::var("ACH_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3100);

    let cors_origin =
        env::var("ACH_CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:4321".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            cors_origin
                .parse::<axum::http::HeaderValue>()
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

    // Session store (shared state)
    let store = session::SessionStore::new();
    store.spawn_reaper();

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

    let app = Router::new()
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
        ));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("ach-server listening on {addr}");
    tracing::info!("CORS origin: {cors_origin}");

    let listener = TcpListener::bind(addr).await.expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
