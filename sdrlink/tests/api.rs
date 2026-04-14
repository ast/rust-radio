use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sdrlink::app_state::AppState;
use sdrlink::auth;
use sdrlink::config::{Config, SdrConfig, ServerConfig, UserRecord};
use sdrlink::sdr::SdrHandle;
use sdrlink::server::router::api_router;
use serde_json::Value;
use tower::ServiceExt;

fn test_config() -> Config {
    let password_hash = auth::hash_password("testpass").unwrap();
    Config {
        server: ServerConfig {
            listen_addr: "127.0.0.1:0".to_string(),
        },
        sdr: SdrConfig {
            center_hz: 100_000_000.0,
            samplerate: 768_000,
            fft_len: 1024,
            fft_rate_hz: 20,
        },
        users: vec![UserRecord {
            username: "testuser".to_string(),
            password_hash,
        }],
    }
}

fn test_app() -> Router {
    let config = test_config();
    let sdr = SdrHandle::start_fake(config.sdr.clone(), 50_000.0);
    let state = AppState::new(config, sdr);
    api_router(Arc::clone(&state))
}

#[tokio::test]
async fn login_with_valid_credentials() {
    let response = test_app()
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
    let response = test_app()
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
    let response = test_app()
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
