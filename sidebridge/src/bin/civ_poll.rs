// Copyright (c) SM6WJM 2026
//
// Quick tool to poll frequency and S-meter from an Icom radio via CI-V.
//
// Usage:
//   civ-poll serial:///dev/ic-705a
//   civ-poll serial:///dev/ic-705a?baud=9600
//   civ-poll tcp://shack:9000

use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use tokio_util::codec::Framed;
use url::Url;

use sidebridge::Transport;
use sidebridge::icom_civ::codec::CivCodec;
use sidebridge::icom_civ::packet::CivPacket;

#[derive(Parser, Debug)]
#[command(author, version, about = "Poll Icom radio frequency and S-meter via CI-V")]
struct Args {
    /// Radio URL: serial:///dev/ic-705a?baud=115200 or tcp://host:port
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let url = Url::parse(&args.url)?;
    let transport = Transport::connect(&url).await?;
    let mut framed = Framed::new(transport, CivCodec);

    use futures::SinkExt;
    framed.send(CivPacket::read_frequency().payload().into()).await?;
    framed.send(CivPacket::read_smeter().payload().into()).await?;

    while let Ok(Some(frame_res)) =
        tokio::time::timeout(std::time::Duration::from_secs(2), framed.next()).await
    {
        match frame_res {
            Ok(frame) => println!("{:?}", frame),
            Err(e) => {
                eprintln!("Parse error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
