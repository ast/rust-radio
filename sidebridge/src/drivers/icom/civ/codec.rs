// Copyright (c) SM6WJM 2026

use bytes::{Buf, Bytes, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

use super::frame::CivFrame;
use super::parser::CivError;

pub struct CivCodec;

impl Decoder for CivCodec {
    type Item = CivFrame;
    type Error = CivError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 1. Look for preamble [0xfe, 0xfe]
        if src.len() < 4 {
            return Ok(None);
        }

        let start_pos = match src.windows(2).position(|w| w == [0xFE, 0xFE]) {
            Some(pos) => pos,
            None => {
                // Prevent memory bloat on noisy lines
                if src.len() > 1024 {
                    src.advance(src.len() - 1);
                }
                return Ok(None);
            }
        };

        // 2. Clear any garbage bytes before the preamble
        if start_pos > 0 {
            src.advance(start_pos);
        }

        // 3. Find the terminator [0xFD]
        if let Some(end_pos) = src.iter().position(|&b| b == 0xFD) {
            // split_to is O(1) - it doesn't copy data, just adjusts pointers
            let frame_bytes = src.split_to(end_pos + 1).freeze();

            // 2. Use your TryFrom implementation!
            let frame = CivFrame::try_from(&frame_bytes[start_pos..])?;
            return Ok(Some(frame));
        }

        Ok(None)
    }
}

impl Encoder<Bytes> for CivCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len() + 3);
        dst.extend_from_slice(&[0xfe, 0xfe]);
        dst.extend_from_slice(&item);
        dst.extend_from_slice(&[0xfd]);
        Ok(())
    }
}
