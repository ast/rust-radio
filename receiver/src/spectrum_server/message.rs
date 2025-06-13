use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, byteorder::big_endian::U16 as U16be,
    byteorder::big_endian::U32 as U32be, byteorder::big_endian::U64 as U64be,
};

/// Message kind
#[repr(u8)]
#[derive(Debug, Copy, Clone, IntoBytes, Immutable)]
pub enum MsgType {
    Spectrum = 0x01,
}

// Header for the spectrum server messages.
#[repr(C)]
#[derive(Debug, Copy, Clone, IntoBytes, Immutable)]
pub struct MsgHeader {
    pub prefix: U32be,
    pub version: u8,
    pub msg_type: MsgType,
    pub sequence_number: U16be,
    pub ntp_time: U64be,
    pub length: U32be,
}

impl MsgHeader {
    pub const PREFIX: u32 = 0xC0DE_0073;
    pub const VERSION: u8 = 1;

    pub fn new(msg_type: MsgType, length: usize) -> Self {
        MsgHeader {
            prefix: U32be::new(0xC0DE_0073),
            version: Self::VERSION,
            msg_type,
            sequence_number: U16be::new(0),
            ntp_time: U64be::new(0),
            length: U32be::new(length.try_into().expect("Length must fit in u32")),
        }
    }
}
