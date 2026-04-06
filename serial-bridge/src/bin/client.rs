// Copyright (c) SM6WJM 2026

use anyhow::Result;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

// CI-V codec for framing/unframing CI-V frames
use serial_bridge::icom_civ::codec::CivCodec;

#[derive(Parser, Debug)]
#[command(
    author = "Albin Stigo <albin@sm6wjm.se>",
    version = "0.1.0",
    about = "CI-V Client for IC-705 Serial Bridge",
    long_about = "A specialized tool to decode and monitor Icom IC-705 CI-V frames over a network bridge."
)]
struct Args {
    /// Address of the bridge server (e.g. 127.0.0.1:9000)
    #[arg(default_value = "shack:9000")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Connecting to IC-705 bridge at {}...", args.host);

    let stream = TcpStream::connect(&args.host).await?;
    let mut framed_client = Framed::new(stream, CivCodec);

    println!("Connected! Monitoring CI-V traffic...\n");

    let get_freq_cmd = vec![0xa4, 0xe0, 0x03];
    framed_client.send(get_freq_cmd.into()).await?;
    println!("Sent: Get Frequency Request");

    // Request S-Meter level
    let poll_smeter = vec![0xa4, 0xe0, 0x15, 0x02];
    framed_client.send(poll_smeter.into()).await?;

    while let Some(frame_res) = framed_client.next().await {
        match frame_res {
            Ok(frame) => {
                // Print command
                println!("{:?}", frame);
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
