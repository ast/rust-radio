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
use futures::{SinkExt, StreamExt};
use tokio_util::codec::Framed;
use url::Url;

use sidebridge::icom_civ::codec::CivCodec;
use sidebridge::icom_civ::command::CivCommand;
use sidebridge::icom_civ::packet::CivPacket;
use sidebridge::icom_civ::scope::ScopeAssembler;
use sidebridge::Transport;

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
    let transport = Transport::connect(&url).await?;
    let mut framed = Framed::new(transport, CivCodec);

    // Always poll frequency and S-meter
    framed
        .send(CivPacket::read_frequency().payload().into())
        .await?;
    framed
        .send(CivPacket::read_smeter().payload().into())
        .await?;

    if args.scope {
        eprintln!("Enabling scope and waveform data output...");
        framed
            .send(CivPacket::scope_on_off(true).payload().into())
            .await?;
        framed
            .send(CivPacket::scope_wave_output(true).payload().into())
            .await?;
    }

    let mut assembler = ScopeAssembler::new();
    let mut frame_count = 0u64;

    let timeout = if args.scope {
        std::time::Duration::from_secs(30)
    } else {
        std::time::Duration::from_secs(2)
    };

    while let Ok(Some(frame_res)) = tokio::time::timeout(timeout, framed.next()).await {
        match frame_res {
            Ok(frame) => match &frame.command {
                CivCommand::ScopeWave(wave) => {
                    eprintln!(
                        "  scope div {}/{} bins={}",
                        wave.division,
                        wave.max_division,
                        wave.bins.len()
                    );
                    if let Some(ref info) = wave.freq_info {
                        eprintln!("  freq_info: {:?}", info);
                    }
                    if let Some(scope_frame) = assembler.push(wave.clone()) {
                        frame_count += 1;
                        println!(
                            "[frame {}] center={} Hz  span={} Hz  bins={}  min={}  max={}",
                            frame_count,
                            scope_frame.center_hz,
                            scope_frame.span_hz,
                            scope_frame.bins.len(),
                            scope_frame.bins.iter().min().unwrap_or(&0),
                            scope_frame.bins.iter().max().unwrap_or(&0),
                        );
                    }
                }
                CivCommand::ScopeSetting(setting) => {
                    eprintln!("  scope setting: {:?}", setting);
                }
                CivCommand::ScopeRaw { sub_cmd, data } => {
                    eprintln!("  scope raw sub={:#04x} len={}", sub_cmd, data.len());
                }
                _ => {
                    println!("{:?}", frame);
                }
            },
            Err(e) => {
                eprintln!("Parse error: {}", e);
                break;
            }
        }
    }

    if args.scope {
        eprintln!("Disabling scope waveform output...");
        framed
            .send(CivPacket::scope_wave_output(false).payload().into())
            .await?;
        eprintln!("Received {} complete scope frames", frame_count);
    }

    Ok(())
}
