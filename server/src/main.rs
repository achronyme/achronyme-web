//! ach-server — Achronyme Server-Side Compilation backend.
//!
//! Thin binary wrapper around the [`ach_server`] library. All of the
//! real wiring lives in `lib.rs` so integration tests can exercise
//! the same router without a TCP listener.

use std::env;
use std::net::{IpAddr, SocketAddr};

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

    // Default to loopback so bare-metal deploys never accidentally expose
    // the API. The container image overrides this to 0.0.0.0 via ENV so
    // docker's port forwarder can reach the listener.
    let bind: IpAddr = env::var("ACH_BIND")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));

    let cfg = AppConfig {
        cors_origin: env::var("ACH_CORS_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:4321".to_string()),
    };

    let store = SessionStore::new();
    store.spawn_reaper();

    let app = build_app(store, &cfg);

    let addr = SocketAddr::new(bind, port);
    tracing::info!("ach-server listening on {addr}");
    tracing::info!("CORS origin: {}", cfg.cors_origin);

    let listener = TcpListener::bind(addr).await.expect("failed to bind");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    tracing::info!("ach-server shutdown complete");
}

/// Resolve when the process receives SIGTERM (systemd stop) or SIGINT
/// (Ctrl-C). Axum stops accepting new connections and lets in-flight
/// requests finish within `TimeoutStopSec`. Session workspaces under
/// `/tmp/ach-sessions/<uuid>` persist across restart by design — the
/// reaper task is a fire-and-forget tokio task that is dropped with
/// the runtime, no explicit cleanup needed.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received SIGINT, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}
