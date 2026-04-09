// Copyright SM6WJM 2026

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use civlink::app_state::AppState;
use civlink::auth;
use civlink::config::{Config, RadioConfig, ServerConfig, UserRecord};
use civlink::server::router::api_router;
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

fn test_config() -> Config {
    let password_hash = auth::hash_password("testpass").unwrap();
    Config {
        server: ServerConfig {
            listen_addr: "127.0.0.1:0".to_string(),
            frontend_dir: "frontend/dist".to_string(),
        },
        radio: RadioConfig {
            url: "tcp://localhost:9000".to_string(),
            audio_device: None,
            sample_rate: 48000,
        },
        users: vec![UserRecord {
            username: "testuser".to_string(),
            password_hash,
        }],
    }
}

fn test_app() -> Router {
    let state = AppState::new(test_config(), None, None);
    api_router(Arc::clone(&state))
}

#[tokio::test]
async fn login_with_valid_credentials() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"testuser","password":"testpass"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].is_string());
    assert!(!json["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn login_with_wrong_password() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"testuser","password":"wrongpass"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_unknown_user() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"nobody","password":"testpass"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
