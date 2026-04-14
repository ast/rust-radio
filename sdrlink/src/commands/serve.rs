use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;

use crate::app_state::AppState;
use crate::config::Config;
use crate::sdr::SdrHandle;
use crate::server::router::api_router;
use crate::server::static_files::static_handler;

pub async fn run(config: Config) -> Result<()> {
    tracing::info!("starting sdrlink server");
    tracing::info!("listen addr: {}", config.server.listen_addr);
    tracing::info!(
        "sdr: center={} Hz samplerate={} fft_len={} fft_rate={}",
        config.sdr.center_hz,
        config.sdr.samplerate,
        config.sdr.fft_len,
        config.sdr.fft_rate_hz,
    );
    tracing::info!("configured users: {}", config.users.len());

    let sdr = SdrHandle::start_hardware(config.sdr.clone())?;
    tracing::info!("airspyhf source started");

    let state = AppState::new(config.clone(), sdr);

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
