// Copyright SM6WJM 2026

use async_trait::async_trait;

use super::civ::packet::CivPacket;
use crate::traits::*;

use super::IcomRadio;

#[async_trait]
impl RadioScope for IcomRadio {
    async fn set_scope_output(&self, enable: bool) -> Result<()> {
        if enable {
            self.expect_ok(CivPacket::scope_on_off(true)).await?;
            self.expect_ok(CivPacket::scope_wave_output(true)).await?;
        } else {
            self.expect_ok(CivPacket::scope_wave_output(false)).await?;
        }
        Ok(())
    }

    async fn set_scope_span(&self, span_hz: u64) -> Result<()> {
        self.expect_ok(CivPacket::scope_span(span_hz)).await
    }
}
