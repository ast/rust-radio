// Copyright (c) SM6WJM 2026

#[derive(Debug, Clone, Copy)] // Copy is now cheap because it's all on the stack
pub struct CivPacket {
    pub target: u8,
    pub source: u8,
    pub command: u8,
    pub subcommand: Option<u8>,
    pub data: [u8; 16], // Fixed size, no heap allocation
    pub data_len: usize,
}
