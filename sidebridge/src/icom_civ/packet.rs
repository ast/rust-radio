// Copyright (c) SM6WJM 2026

use super::command_code::CivCommandCode;

/// Stack-allocated CI-V packet for constructing requests.
///
/// Default addresses: target=0xA4 (IC-705), source=0xE0 (controller).
#[derive(Debug, Clone, Copy)]
pub struct CivPacket {
    pub target: u8,
    pub source: u8,
    pub code: CivCommandCode,
    pub data: [u8; 16],
    pub data_len: usize,
}

/// Default IC-705 address.
pub const IC705_ADDR: u8 = 0xa4;
/// Default controller address.
pub const CONTROLLER_ADDR: u8 = 0xe0;

fn encode_freq_bcd(mut freq: u64, buf: &mut [u8; 16]) -> usize {
    for i in 0..5 {
        let low = (freq % 10) as u8;
        freq /= 10;
        let high = (freq % 10) as u8;
        freq /= 10;
        buf[i] = (high << 4) | low;
    }
    5
}

impl CivPacket {
    /// Create a packet with default IC-705 addresses.
    pub fn new(code: CivCommandCode) -> Self {
        Self {
            target: IC705_ADDR,
            source: CONTROLLER_ADDR,
            code,
            data: [0u8; 16],
            data_len: 0,
        }
    }

    /// Create a packet with custom addresses.
    pub fn with_addr(target: u8, source: u8, code: CivCommandCode) -> Self {
        Self {
            target,
            source,
            code,
            data: [0u8; 16],
            data_len: 0,
        }
    }

    /// Read the current frequency from the transceiver.
    pub fn read_frequency() -> Self {
        Self::new(CivCommandCode::ReadFrequency)
    }

    /// Set the transceiver frequency (BCD-encoded).
    pub fn set_frequency(freq: u64) -> Self {
        let mut pkt = Self::new(CivCommandCode::SetFrequency);
        pkt.data_len = encode_freq_bcd(freq, &mut pkt.data);
        pkt
    }

    /// Read the current operating mode and filter.
    pub fn read_mode() -> Self {
        Self::new(CivCommandCode::ReadMode)
    }

    /// Set the operating mode.
    pub fn set_mode(mode: u8, filter: u8) -> Self {
        let mut pkt = Self::new(CivCommandCode::SetMode);
        pkt.data[0] = mode;
        pkt.data[1] = filter;
        pkt.data_len = 2;
        pkt
    }

    /// Read the PTT state.
    pub fn read_ptt() -> Self {
        Self::new(CivCommandCode::Ptt)
    }

    /// Set PTT on or off.
    pub fn set_ptt(active: bool) -> Self {
        let mut pkt = Self::new(CivCommandCode::Ptt);
        pkt.data[0] = if active { 0x01 } else { 0x00 };
        pkt.data_len = 1;
        pkt
    }

    /// Read the S-meter level.
    pub fn read_smeter() -> Self {
        Self::new(CivCommandCode::ReadSmeter)
    }

    /// Read the SWR meter.
    pub fn read_swr() -> Self {
        Self::new(CivCommandCode::ReadSwr)
    }

    /// Read the ALC meter.
    pub fn read_alc() -> Self {
        Self::new(CivCommandCode::ReadAlc)
    }

    /// Read the RF power meter.
    pub fn read_rf_power() -> Self {
        Self::new(CivCommandCode::ReadRfPower)
    }

    /// Serialize into wire bytes: preamble + payload + EOM.
    pub fn serialize(&self) -> Vec<u8> {
        let subcmd = self.code.subcmd();
        let mut buf = Vec::with_capacity(6 + self.data_len + subcmd.is_some() as usize);
        buf.extend_from_slice(&[0xfe, 0xfe]);
        buf.push(self.target);
        buf.push(self.source);
        buf.push(self.code.cmd());
        if let Some(sub) = subcmd {
            buf.push(sub);
        }
        buf.extend_from_slice(&self.data[..self.data_len]);
        buf.push(0xfd);
        buf
    }

    /// Serialize the payload only (no preamble/EOM) for use with CivCodec.
    pub fn payload(&self) -> Vec<u8> {
        let subcmd = self.code.subcmd();
        let mut buf = Vec::with_capacity(3 + self.data_len + subcmd.is_some() as usize);
        buf.push(self.target);
        buf.push(self.source);
        buf.push(self.code.cmd());
        if let Some(sub) = subcmd {
            buf.push(sub);
        }
        buf.extend_from_slice(&self.data[..self.data_len]);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_frequency_serialize() {
        let pkt = CivPacket::read_frequency();
        assert_eq!(pkt.serialize(), vec![0xfe, 0xfe, 0xa4, 0xe0, 0x03, 0xfd]);
    }

    #[test]
    fn test_set_frequency_bcd() {
        let pkt = CivPacket::set_frequency(14_074_000);
        let bytes = pkt.serialize();
        // 14074000 → BCD little-endian: 00 40 07 14 00
        assert_eq!(
            bytes,
            vec![0xfe, 0xfe, 0xa4, 0xe0, 0x05, 0x00, 0x40, 0x07, 0x14, 0x00, 0xfd]
        );
    }

    #[test]
    fn test_read_smeter() {
        let pkt = CivPacket::read_smeter();
        assert_eq!(
            pkt.serialize(),
            vec![0xfe, 0xfe, 0xa4, 0xe0, 0x15, 0x02, 0xfd]
        );
    }

    #[test]
    fn test_set_ptt_on() {
        let pkt = CivPacket::set_ptt(true);
        assert_eq!(
            pkt.serialize(),
            vec![0xfe, 0xfe, 0xa4, 0xe0, 0x1c, 0x00, 0x01, 0xfd]
        );
    }

    #[test]
    fn test_payload_for_codec() {
        let pkt = CivPacket::read_frequency();
        // CivCodec wraps preamble/EOM itself, so payload is just target+source+cmd
        assert_eq!(pkt.payload(), vec![0xa4, 0xe0, 0x03]);
    }
}
