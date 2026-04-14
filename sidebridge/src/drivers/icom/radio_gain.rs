// Copyright SM6WJM 2026

use async_trait::async_trait;

use super::civ::packet::CivPacket;
use crate::traits::*;

use super::IcomRadio;

#[async_trait]
impl RadioGain for IcomRadio {
    async fn set_rf_gain(&self, value: u8) -> Result<()> {
        self.expect_ok(CivPacket::set_rf_gain(value)).await
    }

    async fn read_rf_gain(&self) -> Result<()> {
        self.send(CivPacket::read_rf_gain()).await
    }
}
