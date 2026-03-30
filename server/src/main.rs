//! ach-server — Achronyme Server-Side Compilation backend.
//!
//! Provides HTTP endpoints for compiling, running, and inspecting
//! Achronyme programs.  Powers the web playground at achrony.me.
//!
//! Usage:
//!   ACH_PORT=3100 ACH_CORS_ORIGIN=https://achrony.me cargo run

use std::env;
use std::net::SocketAddr;
use std::time::Duration;

use axum::http::{header, Method};
use axum::routing::{delete, get, post};
use axum::Router;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

mod error;
mod pipeline;
mod prove_handler;
mod routes;
mod sandbox;
mod session;
mod templates;
mod workspace;

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

    let cors_origin = env::var("ACH_CORS_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:4321".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            cors_origin
                .parse::<axum::http::HeaderValue>()
                .expect("invalid CORS origin"),
        )
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, "X-Ach-Session".parse().unwrap()])
        .expose_headers(["X-Ach-Session".parse().unwrap()]);

    // Session store (shared state)
    let store = session::SessionStore::new();
    store.spawn_reaper();

    let app = Router::new()
        // Existing endpoints
        .route("/api/run", post(routes::run::handler))
        .route("/api/compile", post(routes::compile::handler))
        .route("/api/inspect", post(routes::inspect::handler))
        .route("/api/prove", post(routes::prove::handler))
        .route("/api/format", post(routes::format::handler))
        // Session endpoints
        .route("/api/session/create", post(routes::session::create))
        .route("/api/session", delete(routes::session::delete))
        // File system endpoints
        .route("/api/fs/write", post(routes::fs::write))
        .route("/api/fs/read", post(routes::fs::read))
        .route("/api/fs/delete", post(routes::fs::delete))
        .route("/api/fs/list", get(routes::fs::list))
        .route("/api/fs/rename", post(routes::fs::rename))
        .route("/api/fs/mkdir", post(routes::fs::mkdir))
        // Health
        .route("/health", get(|| async { "ok" }))
        .with_state(store)
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(64 * 1024))
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
