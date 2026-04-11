// Copyright SM6WJM 2026

use anyhow::{Context, Result};
use sidebridge::{RadioEvent, ScopeFreq};
use tokio_stream::StreamExt;
use url::Url;

use crate::Config;
use crate::radio::RadioHandle;

pub async fn run(config: Config) -> Result<()> {
    let url = Url::parse(&config.radio.url).context("invalid radio URL")?;

    let handle = RadioHandle::connect(&url).await?;
    handle.read_initial_state().await?;
    let mut events = handle.event_stream();

    println!("Listening for radio events (Ctrl+C to stop)...\n");

    let mut ctrl_c = std::pin::pin!(tokio::signal::ctrl_c());

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
                        let freq_info = match &frame.freq {
                            ScopeFreq::Center { center_hz, span_hz } =>
                                format!("\"center_hz\":{center_hz},\"span_hz\":{span_hz}"),
                            ScopeFreq::Fixed { lower_hz, upper_hz } =>
                                format!("\"lower_hz\":{lower_hz},\"upper_hz\":{upper_hz}"),
                        };
                        println!(
                            "{{\"type\":\"scope\",{},\"bins\":{},\"peak\":{}}}",
                            freq_info, frame.bins.len(), peak,
                        );
                    }
                    _ => {
                        println!("{}", serde_json::to_string(&event).unwrap());
                    }
                }
            }
            _ = &mut ctrl_c => {
                break;
            }
        }
    }

    Ok(())
}
