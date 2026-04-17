//! ach-server — Achronyme Server-Side Compilation backend.
//!
//! Thin binary wrapper around the [`ach_server`] library. All of the
//! real wiring lives in `lib.rs` so integration tests can exercise
//! the same router without a TCP listener.

use std::env;
use std::net::SocketAddr;

use ach_server::{build_app, session::SessionStore, AppConfig};
use tokio::net::TcpListener;

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

    let cfg = AppConfig {
        cors_origin: env::var("ACH_CORS_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:4321".to_string()),
    };

    let store = SessionStore::new();
    store.spawn_reaper();

    let app = build_app(store, &cfg);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("ach-server listening on {addr}");
    tracing::info!("CORS origin: {}", cfg.cors_origin);

    let listener = TcpListener::bind(addr).await.expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
