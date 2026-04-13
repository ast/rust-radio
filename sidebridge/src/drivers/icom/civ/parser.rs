// Copyright (c) SM6WJM 2026

use std::convert::TryFrom;
use thiserror::Error;

use nom::{
    IResult,
    bytes::complete::{tag, take_until},
    number::complete::u8,
};

use super::command::CivCommand;
use super::frame::CivFrame;
use super::scope;

#[derive(Error, Debug)]
pub enum CivError {
    #[error("Incomplete CI-V frame (need more data)")]
    Incomplete,
    #[error("Invalid CI-V format or checksum")]
    InvalidFormat,
    #[error("NOM parsing error: {0}")]
    ParserError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub(crate) fn parse_bcd_to_u64(input: &[u8]) -> u64 {
    input.iter().rev().fold(0, |acc, &byte| {
        let low = (byte & 0x0f) as u64;
        let high = ((byte >> 4) & 0x0f) as u64;
        acc * 100 + high * 10 + low
    })
}

pub(crate) fn parse_bcd_u8(input: &[u8]) -> u8 {
    if input.is_empty() {
        return 0;
    }
    let byte = input[0];
    ((byte >> 4) * 10) + (byte & 0x0f)
}

fn parse_civ_command(cmd_byte: u8, data: &[u8]) -> CivCommand {
    match cmd_byte {
        // Direct Commands
        0x00 | 0x03 | 0x05 => CivCommand::TransceiverFreq(parse_bcd_to_u64(data)),
        0x01 | 0x04 | 0x06 => {
            if data.len() >= 2 {
                CivCommand::TransceiverMode {
                    mode: data[0],
                    filter: data[1],
                }
            } else if !data.is_empty() {
                CivCommand::TransceiverMode {
                    mode: data[0],
                    filter: 0,
                }
            } else {
                CivCommand::Unknown {
                    cmd: cmd_byte,
                    sub: None,
                    data: data.to_vec(),
                }
            }
        }
        0xfb => CivCommand::Ok,
        0xfa => CivCommand::NotGood,

        // Scope (cmd 0x27)
        0x27 if !data.is_empty() => {
            let sub_cmd = data[0];
            let rest = &data[1..];
            match sub_cmd {
                0x00 => match scope::parse_scope_wave(rest) {
                    Ok(wave) => CivCommand::ScopeWave(wave),
                    Err(_) => CivCommand::ScopeRaw { sub_cmd, data: rest.to_vec() },
                },
                0x10 | 0x11 | 0x14 | 0x15 | 0x17 | 0x1a => {
                    match scope::parse_scope_setting(sub_cmd, rest) {
                        Ok(setting) => CivCommand::ScopeSetting(setting),
                        Err(_) => CivCommand::ScopeRaw { sub_cmd, data: rest.to_vec() },
                    }
                }
                _ => CivCommand::ScopeRaw { sub_cmd, data: rest.to_vec() },
            }
        }

        // Sub-commands
        0x15 if !data.is_empty() => match data[0] {
            0x02 => CivCommand::SignalMeter(parse_bcd_u8(&data[1..])),
            0x11 => CivCommand::RfPower(parse_bcd_u8(&data[1..])),
            0x12 => CivCommand::Swr(parse_bcd_u8(&data[1..])),
            0x13 => CivCommand::Alc(parse_bcd_u8(&data[1..])),
            _ => CivCommand::Unknown {
                cmd: 0x15,
                sub: Some(data[0]),
                data: data[1..].to_vec(),
            },
        },

        0x1c if !data.is_empty() => match data[0] {
            0x00 => CivCommand::SetPtt(data.get(1) == Some(&0x01)),
            _ => CivCommand::Unknown {
                cmd: 0x1c,
                sub: Some(data[0]),
                data: data[1..].to_vec(),
            },
        },

        _ => CivCommand::Unknown {
            cmd: cmd_byte,
            sub: None,
            data: data.to_vec(),
        },
    }
}

pub fn parse_frame(input: &[u8]) -> IResult<&[u8], CivFrame> {
    let (input, _) = tag(&[0xfe, 0xfe][..])(input)?;
    let (input, payload) = take_until(&[0xfd][..])(input)?;
    let (input, _) = tag(&[0xfd][..])(input)?;

    let (rest, dest) = u8(payload)?;
    let (rest, src) = u8(rest)?;
    let (rest, cmd_byte) = u8(rest)?;

    let command = parse_civ_command(cmd_byte, rest);

    Ok((input, CivFrame { dest, src, command }))
}

impl TryFrom<&[u8]> for CivFrame {
    type Error = CivError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match parse_frame(value) {
            Ok((_remaining, frame)) => Ok(frame),
            Err(nom::Err::Incomplete(_)) => Err(CivError::Incomplete),
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                // Convert nom error to a string for the thiserror variant
                Err(CivError::ParserError(format!("{:?}", e.code)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::scope::ScopeSetting;
    use crate::traits::ScopeMode;

    #[test]
    fn test_parse_scope_on_off_frame() {
        // FE FE A4 E0 27 10 01 FD — scope ON
        let frame_bytes = [0xfe, 0xfe, 0xa4, 0xe0, 0x27, 0x10, 0x01, 0xfd];
        let frame = CivFrame::try_from(&frame_bytes[..]).unwrap();
        assert_eq!(frame.dest, 0xa4);
        assert_eq!(frame.src, 0xe0);
        assert!(matches!(
            frame.command,
            CivCommand::ScopeSetting(ScopeSetting::ScopeOnOff(true))
        ));
    }

    #[test]
    fn test_parse_scope_wave_data_frame() {
        // Build a minimal WLAN scope waveform frame
        let mut frame_bytes = vec![0xfe, 0xfe, 0xe0, 0xa4, 0x27, 0x00];
        // Fixed field ①, Division 1/1 (WLAN), center mode
        frame_bytes.push(0x00); // fixed field ①
        frame_bytes.push(0x01); // division = 1 (BCD)
        frame_bytes.push(0x01); // max_div = 1 (BCD)
        frame_bytes.push(0x00); // center mode
        // Center freq: 14.200.000 Hz in BCD LE = [0x00, 0x00, 0x20, 0x14, 0x00]
        frame_bytes.extend_from_slice(&[0x00, 0x00, 0x20, 0x14, 0x00]);
        // Span: 100000 Hz in BCD LE (5 bytes) = [0x00, 0x00, 0x10, 0x00, 0x00]
        frame_bytes.extend_from_slice(&[0x00, 0x00, 0x10, 0x00, 0x00]);
        // Out of range: no
        frame_bytes.push(0x00);
        // 475 bins
        frame_bytes.extend_from_slice(&vec![42u8; 475]);
        frame_bytes.push(0xfd);

        let frame = CivFrame::try_from(&frame_bytes[..]).unwrap();
        assert_eq!(frame.dest, 0xe0);
        assert_eq!(frame.src, 0xa4);

        match &frame.command {
            CivCommand::ScopeWave(wave) => {
                assert_eq!(wave.division, 1);
                assert_eq!(wave.max_division, 1);
                assert_eq!(wave.mode, Some(ScopeMode::Center));
                assert_eq!(wave.bins.len(), 475);
            }
            other => panic!("expected ScopeWave, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_scope_raw_fallback() {
        // Unknown scope subcmd 0x99
        let frame_bytes = [0xfe, 0xfe, 0xa4, 0xe0, 0x27, 0x99, 0x42, 0xfd];
        let frame = CivFrame::try_from(&frame_bytes[..]).unwrap();
        assert!(matches!(
            frame.command,
            CivCommand::ScopeRaw { sub_cmd: 0x99, .. }
        ));
    }
}
