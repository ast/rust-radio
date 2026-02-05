// Copyright (c) SM6WJM 2026

use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use std::io;
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Encoder, Framed};

// nom parser for CI-V frames
use serial_bridge::icom_civ::parser;

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

// --- The CI-V Codec ---
pub struct CivCodec;

impl Decoder for CivCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 1. Look for preamble [0xfe, 0xfe]
        if src.len() < 4 {
            return Ok(None);
        }

        let start_pos = match src.windows(2).position(|w| w == [0xFE, 0xFE]) {
            Some(pos) => pos,
            None => {
                // Prevent memory bloat on noisy lines
                if src.len() > 1024 {
                    src.advance(src.len() - 1);
                }
                return Ok(None);
            }
        };

        // 2. Clear any garbage bytes before the preamble
        if start_pos > 0 {
            src.advance(start_pos);
        }

        // 3. Find the terminator [0xFD]
        if let Some(end_pos) = src.iter().position(|&b| b == 0xFD) {
            // split_to is O(1) - it doesn't copy data, just adjusts pointers
            let frame = src.split_to(end_pos + 1).freeze();
            return Ok(Some(frame));
        }

        Ok(None)
    }
}

impl Encoder<Bytes> for CivCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len() + 3);
        dst.extend_from_slice(&[0xfe, 0xfe]);
        dst.extend_from_slice(&item);
        dst.extend_from_slice(&[0xfd]);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Connecting to IC-705 bridge at {}...", args.host);

    let stream = TcpStream::connect(&args.host).await?;
    let mut framed_client = Framed::new(stream, CivCodec);

    println!("Connected! Monitoring CI-V traffic...\n");

    // --- Send "Get Frequency" Command (Command 0x03) ---
    // FE FE (Preamble) A4 (Radio) E0 (PC) 03 (Cmd) FD (EOM)
    //let get_freq_cmd = vec![0xfe, 0xfe, 0xa4, 0xe0, 0x03, 0xfd];
    let get_freq_cmd = vec![0xa4, 0xe0, 0x03];
    framed_client.send(get_freq_cmd.into()).await?;
    println!("Sent: Get Frequency Request");

    while let Some(frame_res) = framed_client.next().await {
        match frame_res {
            Ok(frame) => {
                let command = parser::parse_frame(&frame);
                // Print command
                println!("{:?}", command);
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
