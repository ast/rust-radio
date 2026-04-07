// Copyright (c) SM6WJM 2026

use anyhow::Result;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use sidebridge::icom_civ::codec::CivCodec;

#[derive(Parser, Debug)]
#[command(
    author = "Albin Stigo <albin@sm6wjm.se>",
    version = "0.1.0",
    about = "CI-V Client — decode and monitor Icom CI-V frames"
)]
struct Args {
    /// Address of the bridge server (e.g. 127.0.0.1:9000)
    #[arg(default_value = "shack:9000")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Connecting to CI-V bridge at {}...", args.host);

    let stream = TcpStream::connect(&args.host).await?;
    let mut framed_client = Framed::new(stream, CivCodec);

    println!("Connected! Monitoring CI-V traffic...\n");

    let get_freq_cmd = vec![0xa4, 0xe0, 0x03];
    framed_client.send(get_freq_cmd.into()).await?;
    println!("Sent: Get Frequency Request");

    let poll_smeter = vec![0xa4, 0xe0, 0x15, 0x02];
    framed_client.send(poll_smeter.into()).await?;

    while let Some(frame_res) = framed_client.next().await {
        match frame_res {
            Ok(frame) => {
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
