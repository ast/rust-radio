// Copyright (c) SM6WJM 2026

use std::convert::TryFrom;
use thiserror::Error;

use nom::{
    IResult,
    bytes::complete::{tag, take_until},
    number::complete::u8,
};

use crate::icom_civ::civ_frame::CivFrame;
use crate::icom_civ::commands::CivCommand;

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

fn parse_bcd_to_u64(input: &[u8]) -> u64 {
    input.iter().rev().fold(0, |acc, &byte| {
        let low = (byte & 0x0f) as u64;
        let high = ((byte >> 4) & 0x0f) as u64;
        acc * 100 + high * 10 + low
    })
}

fn parse_bcd_u8(input: &[u8]) -> u8 {
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

        // Waterfall / Scope Data
        0x27 if !data.is_empty() => CivCommand::Waterfall {
            sub_cmd: data[0],
            scope_data: data[1..].to_vec(),
        },

        // Sub-commands
        0x15 if !data.is_empty() => match data[0] {
            0x02 => CivCommand::SignalMeter(parse_bcd_u8(&data[1..])),
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
