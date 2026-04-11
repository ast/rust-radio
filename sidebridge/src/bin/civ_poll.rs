// Copyright (c) SM6WJM 2026
//
// Quick tool to poll frequency, S-meter, and scope data from an Icom radio via CI-V.
//
// Usage:
//   civ-poll tcp://shack:9000
//   civ-poll tcp://shack:9000 --scope
//   civ-poll serial:///dev/ic-705a
//   civ-poll serial:///dev/ic-705a?baud=9600

use anyhow::Result;
use clap::Parser;
use url::Url;

use sidebridge::{IcomRadio, Radio, RadioEvent, RadioMeter, RadioScope, ScopeFreq};

#[derive(Parser, Debug)]
#[command(author, version, about = "Poll Icom radio frequency, S-meter, and scope via CI-V")]
struct Args {
    /// Radio URL: serial:///dev/ic-705a?baud=115200 or tcp://host:port
    url: String,

    /// Enable scope/waterfall data output
    #[arg(long)]
    scope: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let url = Url::parse(&args.url)?;
    let radio = IcomRadio::connect(&url).await?;

    // Take the event stream before sending any read commands
    let mut rx = radio
        .take_event_stream()
        .expect("event stream already taken");

    // Request frequency and S-meter readings (results arrive as RadioEvents)
    radio.read_frequency().await?;
    radio.read_signal_strength().await?;

    let timeout = std::time::Duration::from_secs(2);
    for _ in 0..2 {
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(RadioEvent::Frequency(hz))) => println!("Frequency: {} Hz", hz),
            Ok(Some(RadioEvent::SignalMeter(val))) => println!("S-meter: {}", val),
            Ok(Some(event)) => println!("Event: {:?}", event),
            Ok(None) => break,
            Err(_) => break,
        }
    }

    if args.scope {
        eprintln!("Enabling scope and waveform data output...");
        radio.set_scope_output(true).await?;

        let mut frame_count = 0u64;
        let scope_timeout = std::time::Duration::from_secs(30);

        while let Ok(Some(event)) = tokio::time::timeout(scope_timeout, rx.recv()).await {
            if let RadioEvent::Scope(frame) = event {
                frame_count += 1;
                let freq_info = match &frame.freq {
                    ScopeFreq::Center { center_hz, span_hz } =>
                        format!("center={center_hz} Hz  span={span_hz} Hz"),
                    ScopeFreq::Fixed { lower_hz, upper_hz } =>
                        format!("lower={lower_hz} Hz  upper={upper_hz} Hz"),
                };
                println!(
                    "[frame {}] {}  bins={}  min={}  max={}",
                    frame_count,
                    freq_info,
                    frame.bins.len(),
                    frame.bins.iter().min().unwrap_or(&0),
                    frame.bins.iter().max().unwrap_or(&0),
                );
            }
        }

        eprintln!("Disabling scope waveform output...");
        radio.set_scope_output(false).await?;
        eprintln!("Received {} complete scope frames", frame_count);
    }

    Ok(())
}
