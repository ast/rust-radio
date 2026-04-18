// Copyright SM6WJM 2026

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use url::Url;

use crate::app_state::AppState;
use crate::audio::AudioCapture;
use crate::config::Config;
use crate::radio::RadioHandle;
use crate::server::router::api_router;
use crate::server::static_files::static_handler;

pub async fn run(config: Config) -> Result<()> {
    tracing::info!("starting civlink server");
    tracing::info!("radio url: {}", config.radio.url);
    tracing::info!(
        "audio device: {}",
        config.radio.audio_device.as_deref().unwrap_or("default")
    );
    tracing::info!("configured users: {}", config.users.len());

    // Connect to radio
    let radio = match Url::parse(&config.radio.url) {
        Ok(url) => match RadioHandle::connect(&url).await {
            Ok(handle) => {
                tracing::info!("radio connected");
                Some(handle)
            }
            Err(e) => {
                tracing::warn!("radio unavailable: {e}");
                None
            }
        },
        Err(e) => {
            tracing::warn!("invalid radio URL: {e}");
            None
        }
    };

    // Start audio capture
    let audio = match AudioCapture::start(
        config.radio.audio_device.as_deref(),
        config.radio.sample_rate,
        2, // stereo
    ) {
        Ok(capture) => {
            tracing::info!("audio capture started");
            Some(capture)
        }
        Err(e) => {
            tracing::warn!("audio capture unavailable: {e}");
            None
        }
    };

    let state = AppState::new(config.clone(), audio, radio);

    let sweep_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(15 * 60));
        ticker.tick().await; // skip immediate first tick
        loop {
            ticker.tick().await;
            sweep_state.sessions.remove_expired().await;
            tracing::debug!("session sweep complete");
        }
    });

    let app = Router::new()
        .nest("/api", api_router(Arc::clone(&state)))
        .fallback(static_handler);

    let listener = TcpListener::bind(&config.server.listen_addr).await?;
    let local = listener.local_addr()?;
    // Terminals render http:// URLs as clickable; rewrite a wildcard bind to
    // localhost so the link opens the right place.
    let host = match local.ip() {
        std::net::IpAddr::V4(ip) if ip.is_unspecified() => "localhost".to_string(),
        std::net::IpAddr::V6(ip) if ip.is_unspecified() => "localhost".to_string(),
        std::net::IpAddr::V6(ip) => format!("[{ip}]"),
        std::net::IpAddr::V4(ip) => ip.to_string(),
    };
    tracing::info!("listening on http://{host}:{}", local.port());

    axum::serve(listener, app).await?;

    Ok(())
}
