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

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub token: String,
}

pub async fn validate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ValidateRequest>,
) -> impl IntoResponse {
    if state.sessions.get_username(&req.token).await.is_some() {
        StatusCode::OK.into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    tracing::info!("login attempt for user '{}'", req.username);

    let user = state.config.users.iter().find(|u| u.username == req.username);

    let Some(user) = user else {
        tracing::warn!("login failed: unknown user '{}'", req.username);
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    };

    match auth::verify_password(&req.password, &user.password_hash) {
        Ok(true) => {
            let token = auth::generate_token();
            state.sessions.insert(token.clone(), req.username.clone()).await;
            tracing::info!("login successful for user '{}'", req.username);
            Json(LoginResponse { token }).into_response()
        }
        Ok(false) => {
            tracing::warn!("login failed: wrong password for user '{}'", req.username);
            (StatusCode::UNAUTHORIZED, "invalid credentials").into_response()
        }
        Err(e) => {
            tracing::error!("login error for user '{}': {}", req.username, e);
            (StatusCode::UNAUTHORIZED, "invalid credentials").into_response()
        }
    }
}
