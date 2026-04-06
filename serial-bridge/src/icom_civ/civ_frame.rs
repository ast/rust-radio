// Copyright (c) SM6WJM 2026

use super::commands::CivCommand;

#[derive(Debug)]
pub struct CivFrame {
    pub dest: u8,
    pub src: u8,
    pub command: CivCommand,
}
