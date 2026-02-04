// Copyright (c) SM6WJM 2026

use anyhow::Result;
use bytes::{Buf, BytesMut};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use std::io;
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Encoder, Framed};

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
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Look for preamble fe fe
        let preamble = [0xfe, 0xfe];
        if src.len() < 4 {
            // Not enough data for preamble + addr + cmd + eom
            return Ok(None);
        }

        if let Some(start_pos) = src.windows(2).position(|w| w == preamble) {
            if start_pos > 0 {
                src.advance(start_pos);
            }
        } else {
            if src.len() > 1024 {
                src.clear();
            }
            return Ok(None);
        }

        // Look for EOM 0xfd
        if let Some(end_pos) = src.iter().position(|&b| b == 0xfd) {
            let frame = src.split_to(end_pos + 1).to_vec();
            return Ok(Some(frame));
        }
        Ok(None)
    }
}

impl Encoder<Vec<u8>> for CivCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Reserve space for Preamble (2) + Data (N) + Terminator (1)
        dst.reserve(item.len() + 3);

        dst.extend_from_slice(&[0xfe, 0xfe]); // Add Preamble
        dst.extend_from_slice(&item); // Add Addresses + Command + Data
        dst.extend_from_slice(&[0xfd]); // Add Terminator

        Ok(())
    }
}

// --- Helper: BCD to Frequency ---
fn parse_bcd_freq(data: &[u8]) -> u64 {
    let mut freq: u64 = 0;
    let mut multiplier: u64 = 1;
    // Icom frequency is LSB first
    for &byte in data {
        let low = (byte & 0x0f) as u64;
        let high = ((byte >> 4) & 0x0f) as u64;
        freq += low * multiplier;
        multiplier *= 10;
        freq += high * multiplier;
        multiplier *= 10;
    }
    freq
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
    framed_client.send(get_freq_cmd).await?;
    println!("Sent: Get Frequency Request");

    while let Some(frame_res) = framed_client.next().await {
        match frame_res {
            Ok(frame) => {
                // frame[0,1] = fe fe
                // frame[2]   = dest
                // frame[3]   = source
                // frame[4]   = command
                if frame.len() >= 6 {
                    let cmd = frame[4];
                    print!("[Cmd {:02x}] Raw: {:02x?}", cmd, frame);

                    // Special handling for frequency (Cmd 0x00, 0x03, or 0x05)
                    if (cmd == 0x00 || cmd == 0x03 || cmd == 0x05) && frame.len() >= 10 {
                        let freq_bytes = &frame[5..frame.len() - 1];
                        let hz = parse_bcd_freq(freq_bytes);
                        print!(" -> Freq: {:.3} MHz", hz as f64 / 1_000_000.0);
                    }
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
