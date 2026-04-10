// Copyright SM6WJM 2026

use tokio_stream::StreamExt;
use url::Url;

use crate::radio::RadioHandle;
use crate::{Config, Result};

pub async fn run(config: Config) -> Result<()> {
    let url = Url::parse(&config.radio.url)
        .map_err(|e| crate::CivlinkError::Config(format!("invalid radio URL: {e}")))?;

    let handle = RadioHandle::connect(&url).await?;
    let mut scope = handle.scope_stream();

    let mut total: u64 = 0;

    println!("Listening for scope frames (Ctrl+C to stop)...\n");

    loop {
        tokio::select! {
            frame = scope.next() => {
                let Some(frame) = frame else {
                    println!("scope stream ended");
                    break;
                };
                total += 1;

                let peak = frame.bins.iter().copied().max().unwrap_or(0);
                let mean: f32 = if frame.bins.is_empty() {
                    0.0
                } else {
                    frame.bins.iter().map(|&b| b as f32).sum::<f32>() / frame.bins.len() as f32
                };

                println!(
                    "frame {:>5} | center {:>10} Hz | span {:>8} Hz | {} bins | peak {:>3} | mean {:.1}",
                    total,
                    frame.center_hz,
                    frame.span_hz,
                    frame.bins.len(),
                    peak,
                    mean,
                );
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    println!("\n--- Summary ---");
    println!("Total frames: {total}");

    Ok(())
}
