// Copyright SM6WJM 2026

use rand::Rng;

/// Real-time Transport Protocol (RTP) Header
#[derive(Debug, Clone)]
pub struct RtpHeader {
    pub padding: bool,
    pub extension: bool,
    pub csrc_count: u8,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence: u16,
    pub timestamp: u32,
    pub ssrc: u32,       // Sync Source ID (unique to this stream)
    pub csrc: [u32; 15], // Contributing Source IDs (optional, up to 15)
}

/// New RTP Header with random sequence, timestamp, and ssrc
impl RtpHeader {
    pub fn new(payload_type: u8) -> Self {
        let mut rng = rand::rng();
        let sequence: u16 = rng.random();
        let timestamp: u32 = rng.random();
        let ssrc: u32 = rng.random();

        Self {
            payload_type,
            sequence,
            timestamp,
            ssrc,
            ..Default::default()
        }
    }

    /// Advances the header state for the next packet in the stream.
    pub fn advance(&mut self, samples: u32) {
        // Use wrapping_add to handle overflow per RFC 3550
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(samples);
    }
}

/// Default RTP Header to Opus
impl Default for RtpHeader {
    fn default() -> Self {
        RtpHeader {
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 111, // Opus
            sequence: 0,
            timestamp: 0,
            ssrc: 0,
            csrc: [0; 15],
        }
    }
}

impl RtpHeader {
    pub fn serialize(&self) -> [u8; 12] {
        let mut header = [0u8; 12];

        header[0] = 0x80
            | (self.padding as u8) << 5
            | (self.extension as u8) << 4
            | (self.csrc_count & 0x0f);

        header[1] = ((self.marker as u8) << 7) | (self.payload_type & 0x7f);

        header[2..4].copy_from_slice(&self.sequence.to_be_bytes());
        header[4..8].copy_from_slice(&self.timestamp.to_be_bytes());
        header[8..12].copy_from_slice(&self.ssrc.to_be_bytes());

        // TODO: CSRCs are not included in this serialization

        header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rtp_header() {
        let rtp_header = RtpHeader::default();

        assert_eq!(rtp_header.payload_type, 111);
        assert_eq!(rtp_header.sequence, 0);
        assert_eq!(rtp_header.timestamp, 0);
        assert_eq!(rtp_header.ssrc, 0);
    }

    #[test]
    fn serialize_default_rtp_header() {
        let rtp_header = RtpHeader::default();

        let serialized = rtp_header.serialize();

        let expected: [u8; 12] = [
            0x80, 0x6f, 0x00, 0x00, // V=2,P=0,X=0,CC=0,M=0,PT=111, Sequence=0
            0x00, 0x00, 0x00, 0x00, // Timestamp=0
            0x00, 0x00, 0x00, 0x00, // SSRC=0
        ];

        assert_eq!(serialized, expected);
    }
}
