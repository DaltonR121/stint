//! Local HTTP API server for Stint.
//!
//! Provides a JSON API on localhost for editor plugins and integrations.

mod api;

use std::sync::{Arc, Mutex};

use axum::routing::{get, post};
use axum::Router;
use stint_core::service::StintService;
use stint_core::storage::sqlite::SqliteStorage;
use tokio::net::TcpListener;

/// Runs the local API server on the given port.
///
/// Blocks until SIGINT/SIGTERM is received.
pub fn run_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { serve(port).await })
}

/// Async server implementation.
async fn serve(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let path = SqliteStorage::default_path();
    let storage = SqliteStorage::open(&path)?;
    let service = StintService::new(storage);
    let state = Arc::new(Mutex::new(service));

    let app = Router::new()
        .route("/api/health", get(api::health))
        .route("/api/status", get(api::status))
        .route("/api/entries", get(api::entries))
        .route("/api/projects", get(api::projects))
        .route("/api/start", post(api::start))
        .route("/api/stop", post(api::stop))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    println!("Stint API listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");
    Ok(())
}

/// Waits for SIGINT or SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = ctrl_c => {},
            _ = sigterm.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}
