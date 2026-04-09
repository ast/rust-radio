// Copyright SM6WJM 2026

use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use crate::app_state::AppState;
use crate::config::Config;
use crate::server::router::api_router;
use crate::Result;

pub async fn run(config: Config) -> Result<()> {
    let state = AppState::new(config.clone());

    let app = Router::new()
        .nest("/api", api_router(Arc::clone(&state)))
        .fallback_service(ServeDir::new(&config.server.frontend_dir));

    let listener = TcpListener::bind(&config.server.listen_addr).await?;
    tracing::info!("listening on {}", config.server.listen_addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::CivlinkError::Io(e))?;

    Ok(())
}
