// Copyright (c) SM6WJM 2026

use std::convert::TryInto;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    LSB = 0x00,
    USB = 0x01,
    AM = 0x02,
    CW = 0x03,
    RTTY = 0x04,
    FM = 0x05,
    WFM = 0x06,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    F1 = 0x01,
    F2 = 0x02,
    F3 = 0x03,
}

#[derive(Debug, Clone)]
pub enum CivCommand {
    /// Read operating frequency (Cmd 03)
    ReadFreq,
    /// Set operating frequency (Cmd 05) - Provide Hz (e.g., 14250000)
    SetFreq(u64),
    /// Read operating mode and filter (Cmd 04)
    ReadMode,
    /// Set operating mode and filter (Cmd 06)
    SetMode(Mode, Filter),
    /// Read AF Gain (Cmd 14 01)
    ReadAFGain,
    /// Set AF Gain (Cmd 14 01) - Range 0..=255
    SetAFGain(u8),
    /// Read S-Meter level (Cmd 15 02)
    ReadSMeter,
    /// Set Power On/Off (Cmd 18)
    SetPower(bool),
}

impl CivCommand {
    /// Serializes the command into the data payload portion of a CI-V packet.
    /// Does NOT include the FE FE preamble or the FD terminator.
    pub fn serialize_payload(&self) -> Vec<u8> {
        match self {
            CivCommand::ReadFreq => vec![0x03],
            CivCommand::SetFreq(hz) => {
                let mut data = vec![0x05];
                data.extend_from_slice(&to_icom_bcd(*hz));
                data
            }
            CivCommand::ReadMode => vec![0x04],
            CivCommand::SetMode(m, f) => vec![0x06, *m as u8, *f as u8],
            CivCommand::ReadAFGain => vec![0x14, 0x01],
            CivCommand::SetAFGain(val) => vec![0x14, 0x01, *val],
            CivCommand::ReadSMeter => vec![0x15, 0x02],
            CivCommand::SetPower(on) => vec![0x18, if *on { 0x01 } else { 0x00 }],
        }
    }
}

/// Helper: Converts Hz to Icom's 5-byte Little-Endian BCD format.
/// Example: 14.250.000 -> [0x00, 0x00, 0x25, 0x14, 0x00]
fn to_icom_bcd(hz: u64) -> [u8; 5] {
    let mut bcd = [0u8; 5];
    let mut n = hz;
    for i in 0..5 {
        let digit_low = (n % 10) as u8;
        n /= 10;
        let digit_high = (n % 10) as u8;
        n /= 10;
        bcd[i] = (digit_high << 4) | digit_low;
    }
    bcd
}
