// Copyright SM6WJM 2026

use std::sync::Arc;

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

    let app = Router::new()
        .nest("/api", api_router(Arc::clone(&state)))
        .fallback(static_handler);

    let listener = TcpListener::bind(&config.server.listen_addr).await?;
    tracing::info!("listening on {}", config.server.listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
