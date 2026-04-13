// Copyright (c) SM6WJM 2026
//
// CI-V debug client.
//
// Connects directly to a serial-attached Icom radio, sends a single CI-V
// command (either a named built-in or a raw hex payload), and prints every
// frame the radio returns for a short listen window.
//
// Designed to be scp-ed to a target machine and run over ssh. Useful for
// agents or operators to clarify undocumented protocol behaviour: send a
// command, see the raw bytes and decoded frames that come back.
//
// Examples:
//   civ-client --device /dev/ic-705a read-freq
//   civ-client --device /dev/ic-705a --address 0xB6 read-smeter
//   civ-client --device /dev/ic-705a raw 03              # payload only (read freq)
//   civ-client --device /dev/ic-705a raw fefea4e003fd    # full packet with preamble/EOM
//   civ-client --device /dev/ic-705a --timeout 3000 scope-span 10000

use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use futures::{SinkExt, StreamExt};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Framed;

use sidebridge::CivCodec;
use sidebridge::drivers::icom::civ::packet::CivPacket;

#[derive(Parser, Debug)]
#[command(
    author = "Albin Stigo <albin@sm6wjm.se>",
    version,
    about = "CI-V debug client - send one command to an Icom radio over serial and print every returned frame"
)]
struct Args {
    /// Serial device path (e.g. /dev/ic-705a, /dev/ttyUSB0)
    #[arg(long)]
    device: String,

    /// Serial baud rate
    #[arg(long, default_value_t = 115_200)]
    baud: u32,

    /// Radio CI-V address (IC-705=0xA4, IC-7300/MK2=0xB6). Accepts 0x-prefixed hex or decimal.
    #[arg(long, default_value = "0xA4", value_parser = parse_u8)]
    address: u8,

    /// Controller CI-V address (source field in outgoing packets).
    #[arg(long, default_value = "0xE0", value_parser = parse_u8)]
    controller: u8,

    /// Hard listen deadline after sending, in milliseconds. The tool exits this long after send,
    /// regardless of activity. Keeps runs bounded when the radio is streaming (e.g. scope data).
    #[arg(long, default_value_t = 800)]
    timeout: u64,

    /// Exit early once this many frames have been received.
    #[arg(long, default_value_t = 8)]
    max_frames: u32,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Send a raw CI-V byte sequence. Accepts either the payload (target/source/cmd/...)
    /// or a full framed packet starting with FE FE and ending in FD. Whitespace ignored.
    Raw {
        /// Hex bytes, e.g. "03" or "27 15 00 00 10 00 00 00" or "fefea4e003fd".
        hex: String,
    },

    /// Read current frequency (CI-V 03).
    ReadFreq,
    /// Read current mode + filter (CI-V 04).
    ReadMode,
    /// Read PTT state (CI-V 1C 00).
    ReadPtt,
    /// Read S-meter (CI-V 15 02).
    ReadSmeter,
    /// Read SWR meter (CI-V 15 12).
    ReadSwr,
    /// Read ALC meter (CI-V 15 13).
    ReadAlc,
    /// Read RF power meter (CI-V 15 11).
    ReadRfPower,

    /// Enable or disable the scope display (CI-V 27 10). Arg: on/off/1/0.
    ScopeOnOff { on: String },
    /// Enable or disable scope waveform data output (CI-V 27 11). Arg: on/off/1/0.
    ScopeWaveOutput { on: String },
    /// Set scope center (false) or fixed-edge (true) mode (CI-V 27 14). Arg: on/off/1/0 (on=fixed).
    ScopeCenterFixed { fixed: String },
    /// Set scope span in Hz (CI-V 27 15). Valid: 2500, 5000, 10000, 25000, 50000, 100000, 250000, 500000.
    ScopeSpan { span_hz: u64 },
}

fn parse_u8(s: &str) -> Result<u8, String> {
    let s = s.trim();
    let (body, radix) = if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (rest, 16)
    } else {
        (s, 10)
    };
    u8::from_str_radix(body, radix).map_err(|e| format!("invalid u8 '{s}': {e}"))
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" => Ok(true),
        "0" | "off" | "false" | "no" => Ok(false),
        other => Err(format!("expected on/off, got '{other}'")),
    }
}

fn parse_hex(s: &str) -> Result<Vec<u8>> {
    let cleaned: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        bail!("hex string has odd length ({} chars)", cleaned.len());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .with_context(|| format!("invalid hex byte at offset {i}: '{}'", &cleaned[i..i + 2]))
        })
        .collect()
}

fn hex_dump(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Turn a user-supplied CLI command into the CivCodec payload bytes
/// (target + source + cmd [+ subcmd] + data). The codec wraps FE FE ... FD
/// itself, so we strip those if present in a raw input.
fn build_payload(args: &Args) -> Result<Vec<u8>> {
    let pkt_to_payload = |mut pkt: CivPacket| {
        pkt.target = args.address;
        pkt.source = args.controller;
        pkt.payload()
    };

    Ok(match &args.cmd {
        Cmd::Raw { hex } => {
            let bytes = parse_hex(hex)?;
            // If this looks like a full packet with preamble+EOM, extract the payload.
            if bytes.len() >= 5
                && bytes.starts_with(&[0xfe, 0xfe])
                && bytes.last() == Some(&0xfd)
            {
                bytes[2..bytes.len() - 1].to_vec()
            } else if bytes.len() >= 3 && bytes[0] == args.address {
                // Already a payload (target/source/cmd/...)
                bytes
            } else {
                // Bare command bytes - prepend target/source.
                let mut p = vec![args.address, args.controller];
                p.extend_from_slice(&bytes);
                p
            }
        }
        Cmd::ReadFreq => pkt_to_payload(CivPacket::read_frequency()),
        Cmd::ReadMode => pkt_to_payload(CivPacket::read_mode()),
        Cmd::ReadPtt => pkt_to_payload(CivPacket::read_ptt()),
        Cmd::ReadSmeter => pkt_to_payload(CivPacket::read_smeter()),
        Cmd::ReadSwr => pkt_to_payload(CivPacket::read_swr()),
        Cmd::ReadAlc => pkt_to_payload(CivPacket::read_alc()),
        Cmd::ReadRfPower => pkt_to_payload(CivPacket::read_rf_power()),
        Cmd::ScopeOnOff { on } => {
            pkt_to_payload(CivPacket::scope_on_off(parse_bool(on).map_err(anyhow::Error::msg)?))
        }
        Cmd::ScopeWaveOutput { on } => pkt_to_payload(CivPacket::scope_wave_output(
            parse_bool(on).map_err(anyhow::Error::msg)?,
        )),
        Cmd::ScopeCenterFixed { fixed } => pkt_to_payload(CivPacket::scope_center_fixed(
            parse_bool(fixed).map_err(anyhow::Error::msg)?,
        )),
        Cmd::ScopeSpan { span_hz } => pkt_to_payload(CivPacket::scope_span(*span_hz)),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let payload = build_payload(&args)?;

    // Wire bytes include preamble + payload + EOM (what actually goes on the bus).
    let wire: Vec<u8> = std::iter::once(0xfe_u8)
        .chain(std::iter::once(0xfe))
        .chain(payload.iter().copied())
        .chain(std::iter::once(0xfd))
        .collect();

    println!("device:    {} @ {} baud", args.device, args.baud);
    println!("address:   radio=0x{:02x} controller=0x{:02x}", args.address, args.controller);
    println!("payload:   {}", hex_dump(&payload));
    println!("wire:      {}", hex_dump(&wire));
    println!();

    let serial = tokio_serial::new(&args.device, args.baud)
        .open_native_async()
        .with_context(|| format!("failed to open {}", args.device))?;
    let mut framed = Framed::new(serial, CivCodec);

    framed
        .send(payload.into())
        .await
        .context("failed to send CI-V payload")?;

    let deadline = tokio::time::Instant::now() + Duration::from_millis(args.timeout);
    let mut frame_n = 0u32;

    // Listen until the hard deadline, the frame cap, or the port closes.
    while frame_n < args.max_frames {
        match tokio::time::timeout_at(deadline, framed.next()).await {
            Ok(Some(Ok(frame))) => {
                frame_n += 1;
                println!(
                    "[frame {frame_n}] dest=0x{:02x} src=0x{:02x}",
                    frame.dest, frame.src
                );
                println!("           command: {:?}", frame.command);
            }
            Ok(Some(Err(e))) => {
                eprintln!("[frame {}] decode error: {e}", frame_n + 1);
            }
            Ok(None) => {
                eprintln!("serial port closed");
                break;
            }
            Err(_) => {
                // Deadline elapsed.
                break;
            }
        }
    }

    println!();
    println!("received {frame_n} frame(s)");
    Ok(())
}
