// Copyright (c) SM6WJM 2026

use bytes::Bytes;

use super::scope::{ScopeSetting, ScopeWaveData};

#[derive(Debug, PartialEq)]
pub enum CivCommand {
    TransceiverFreq(u64), // 0x00, 0x03, 0x05
    TransceiverMode {
        mode: u8,
        filter: u8,
    }, // 0x01, 0x04, 0x06
    SetPtt(bool),         // 0x1c 0x00
    SignalMeter(u8),      // 0x15 0x02
    RfPower(u8),          // 0x15 0x11
    Swr(u8),              // 0x15 0x12
    Alc(u8),              // 0x15 0x13
    ScopeWave(ScopeWaveData),       // 0x27 0x00 — parsed waveform division
    ScopeSetting(ScopeSetting),     // 0x27 0x10-0x1E — parsed scope setting
    ScopeRaw {                      // 0x27 — fallback for unknown/failed parse
        sub_cmd: u8,
        data: Vec<u8>,
    },
    Ok,      // 0xfb (ACK)
    NotGood, // 0xfa (NAK)
    Unknown {
        cmd: u8,
        sub: Option<u8>,
        data: Vec<u8>,
    },
}

fn encode_u64_to_bcd(mut freq: u64) -> Vec<u8> {
    let mut bcd = Vec::new();
    // Icom usually expects 5 bytes for frequency (10 digits)
    for _ in 0..5 {
        let low = (freq % 10) as u8;
        freq /= 10;
        let high = (freq % 10) as u8;
        freq /= 10;
        bcd.push((high << 4) | low);
    }
    bcd
}

impl CivCommand {
    pub fn serialize(&self) -> (u8, Bytes) {
        match self {
            CivCommand::TransceiverFreq(freq) => (0x05, Bytes::from(encode_u64_to_bcd(*freq))),
            CivCommand::Ok => (0xfb, Bytes::new()),
            _ => (0x00, Bytes::new()),
        }
    }
}
