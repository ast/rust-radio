// Copyright (c) SM6WJM 2026

use nom::{
    IResult, bytes::complete::tag, bytes::complete::take, bytes::complete::take_until,
    number::complete::u8,
};

#[derive(Debug)]
pub enum CivCommand {
    TransceiverFreq(u64),                     // Cmd 0x00, 0x03, 0x05
    TransceiverMode { mode: u8, filter: u8 }, // Cmd 0x01, 0x04
    Unknown { cmd: u8, data: Vec<u8> },
}

#[derive(Debug)]
pub struct CivFrame {
    pub dest: u8,
    pub src: u8,
    pub command: CivCommand,
}

fn parse_bcd_to_u64(input: &[u8]) -> u64 {
    input.iter().rev().fold(0, |acc, &byte| {
        let low = (byte & 0x0f) as u64;
        let high = ((byte >> 4) & 0x0f) as u64;
        acc * 100 + high * 10 + low
    })
}

fn parse_civ_command(cmd_byte: u8, data: &[u8]) -> CivCommand {
    match cmd_byte {
        0x00 | 0x03 | 0x05 => CivCommand::TransceiverFreq(parse_bcd_to_u64(data)),
        0x01 | 0x04 if data.len() >= 2 => CivCommand::TransceiverMode {
            mode: data[0],
            filter: data[1],
        },
        _ => CivCommand::Unknown {
            cmd: cmd_byte,
            data: data.to_vec(),
        },
    }
}

/// Parse a CI-V frame from the input byte slice
pub fn parse_frame(input: &[u8]) -> IResult<&[u8], CivFrame> {
    // Consume the preamble
    let (input, _) = tag(&[0xfe, 0xfe][..])(input)?;
    // Identify everything up to the terminator
    let (input, payload) = take_until(&[0xfd][..])(input)?;
    // Consume the terminator
    let (input, _) = tag(&[0xfd][..])(input)?;

    // Parse the inner slice
    let (rest, dest) = u8(payload)?;
    let (rest, src) = u8(rest)?;
    let (rest, cmd_byte) = u8(rest)?;

    let command = parse_civ_command(cmd_byte, rest);

    Ok((input, CivFrame { dest, src, command }))
}
