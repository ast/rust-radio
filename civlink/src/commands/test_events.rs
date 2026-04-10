// Copyright SM6WJM 2026

use sidebridge::RadioEvent;
use tokio_stream::StreamExt;
use url::Url;

use crate::radio::RadioHandle;
use crate::{Config, Result};

pub async fn run(config: Config) -> Result<()> {
    let url = Url::parse(&config.radio.url)
        .map_err(|e| crate::CivlinkError::Config(format!("invalid radio URL: {e}")))?;

    let handle = RadioHandle::connect(&url).await?;
    handle.read_initial_state().await?;
    let mut events = handle.event_stream();

    println!("Listening for radio events (Ctrl+C to stop)...\n");

    loop {
        tokio::select! {
            event = events.next() => {
                let Some(event) = event else {
                    println!("event stream ended");
                    break;
                };
                match &event {
                    RadioEvent::Scope(frame) => {
                        let peak = frame.bins.iter().copied().max().unwrap_or(0);
                        println!(
                            "{{\"type\":\"scope\",\"center_hz\":{},\"span_hz\":{},\"bins\":{},\"peak\":{}}}",
                            frame.center_hz, frame.span_hz, frame.bins.len(), peak,
                        );
                    }
                    _ => {
                        println!("{}", serde_json::to_string(&event).unwrap());
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    Ok(())
}
