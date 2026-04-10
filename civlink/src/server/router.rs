// Copyright SM6WJM 2026

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use crate::app_state::AppState;
use crate::server::api;
use crate::server::signaling;

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/login", post(api::login))
        .route("/validate", post(api::validate))
        .route("/ws", get(signaling::ws_handler))
        .with_state(state)
}
