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
use tokio_stream::StreamExt;
use url::Url;

use sidebridge::{IcomRadio, Radio, RadioMeter, RadioScope};

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

    let freq = radio.frequency().await?;
    println!("Frequency: {} Hz", freq);

    let smeter = radio.signal_strength().await?;
    println!("S-meter: {}", smeter);

    if args.scope {
        eprintln!("Enabling scope and waveform data output...");
        radio.set_scope_output(true).await?;

        let mut stream = radio.scope_stream();
        let mut frame_count = 0u64;
        let timeout = std::time::Duration::from_secs(30);

        while let Ok(Some(frame)) = tokio::time::timeout(timeout, stream.next()).await {
            frame_count += 1;
            println!(
                "[frame {}] center={} Hz  span={} Hz  bins={}  min={}  max={}",
                frame_count,
                frame.center_hz,
                frame.span_hz,
                frame.bins.len(),
                frame.bins.iter().min().unwrap_or(&0),
                frame.bins.iter().max().unwrap_or(&0),
            );
        }

        eprintln!("Disabling scope waveform output...");
        radio.set_scope_output(false).await?;
        eprintln!("Received {} complete scope frames", frame_count);
    }

    Ok(())
}
