// Copyright (c) SM6WJM 2026

#[derive(Debug, Clone, PartialEq)]
pub struct CivPacket {
    pub dst: u8, // Usually 0xA4 for IC-705
    pub src: u8, // Usually 0xE0 for Controller
    pub cmd: u8, // Main command (e.g., 0x05 for frequency)
    pub sub_cmd: Option<u8>,
    pub data: Vec<u8>, // Variable length payload
}

impl CivPacket {
    const PREAMBLE: [u8; 2] = [0xfe, 0xfe];
    const EOM: u8 = 0xfd;

    /// Create a new packet for the IC-705
    pub fn new(cmd: u8, sub_cmd: Option<u8>, data: Vec<u8>) -> Self {
        Self {
            dst: 0xa4, // Default IC-705 Address
            src: 0xe0, // Default PC Address
            cmd,
            sub_cmd,
            data,
        }
    }

    /// Serialize into raw bytes for the serial port
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(10 + self.data.len());
        buf.extend_from_slice(&Self::PREAMBLE);
        buf.push(self.dst);
        buf.push(self.src);
        buf.push(self.cmd);

        if let Some(sub) = self.sub_cmd {
            buf.push(sub);
        }

        buf.extend_from_slice(&self.data);
        buf.push(Self::EOM);
        buf
    }
}
