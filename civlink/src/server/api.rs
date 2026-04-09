// Copyright SM6WJM 2026

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::auth;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = state.config.users.iter().find(|u| u.username == req.username);

    let Some(user) = user else {
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    };

    match auth::verify_password(&req.password, &user.password_hash) {
        Ok(true) => {
            let token = auth::generate_token();
            state.sessions.insert(token.clone(), req.username).await;
            Json(LoginResponse { token }).into_response()
        }
        _ => (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
    }
}
