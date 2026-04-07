// Copyright (c) SM6WJM 2026
//
// Quick tool to poll frequency and S-meter from an Icom radio via CI-V.

use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Framed;

use sidebridge::icom_civ::codec::CivCodec;
use sidebridge::icom_civ::packet::CivPacket;

#[derive(Parser, Debug)]
#[command(author, version, about = "Poll IC-705 frequency and S-meter via CI-V")]
struct Args {
    /// Serial port path
    #[arg(short, long, default_value = "/dev/ic-705a")]
    serial: String,

    /// Baud rate
    #[arg(short, long, default_value_t = 115200)]
    baud: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let serial = tokio_serial::new(&args.serial, args.baud).open_native_async()?;
    let mut framed = Framed::new(serial, CivCodec);

    use futures::SinkExt;
    framed.send(CivPacket::read_frequency().payload().into()).await?;
    framed.send(CivPacket::read_smeter().payload().into()).await?;

    // Read all responses for 2 seconds
    while let Ok(Some(frame_res)) =
        tokio::time::timeout(std::time::Duration::from_secs(2), framed.next()).await
    {
        match frame_res {
            Ok(frame) => {
                println!("{:?}", frame);
            }
            Err(e) => {
                eprintln!("Parse error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
