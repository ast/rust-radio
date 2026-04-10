// Copyright SM6WJM 2026

use async_trait::async_trait;

use crate::icom_civ::packet::CivPacket;
use crate::traits::*;

use super::IcomRadio;

#[async_trait]
impl RadioMeter for IcomRadio {
    async fn read_signal_strength(&self) -> Result<()> {
        self.send(CivPacket::read_smeter()).await
    }

    async fn read_swr(&self) -> Result<()> {
        self.send(CivPacket::read_swr()).await
    }

    async fn read_alc(&self) -> Result<()> {
        self.send(CivPacket::read_alc()).await
    }

    async fn read_rf_power(&self) -> Result<()> {
        self.send(CivPacket::read_rf_power()).await
    }
}
