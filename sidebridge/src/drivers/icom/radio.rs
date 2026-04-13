// Copyright SM6WJM 2026

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::civ::command::CivCommand;
use super::civ::packet::CivPacket;
use crate::traits::*;

use super::{IcomRadio, mode_to_civ};

#[async_trait]
impl Radio for IcomRadio {
    fn id(&self) -> &str {
        &self.id
    }

    async fn set_frequency(&self, hz: u64) -> Result<()> {
        self.expect_ok(CivPacket::set_frequency(hz)).await
    }

    async fn read_frequency(&self) -> Result<()> {
        self.send(CivPacket::read_frequency()).await
    }

    async fn set_mode(&self, mode: Mode) -> Result<()> {
        self.expect_ok(CivPacket::set_mode(mode_to_civ(&mode), 0x01))
            .await
    }

    async fn read_mode(&self) -> Result<()> {
        self.send(CivPacket::read_mode()).await
    }

    async fn set_ptt(&self, active: bool) -> Result<()> {
        self.expect_ok(CivPacket::set_ptt(active)).await
    }

    async fn ptt(&self) -> Result<bool> {
        match self.command(CivPacket::read_ptt()).await? {
            CivCommand::SetPtt(active) => Ok(active),
            CivCommand::NotGood => Err(RadioError::CommandFailed),
            other => Err(RadioError::Protocol(format!(
                "expected PTT, got {:?}",
                other
            ))),
        }
    }

    fn take_event_stream(&self) -> Option<mpsc::Receiver<RadioEvent>> {
        self.event_rx.lock().unwrap().take()
    }
}
