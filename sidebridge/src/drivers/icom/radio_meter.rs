// Copyright SM6WJM 2026

use async_trait::async_trait;

use crate::icom_civ::command::CivCommand;
use crate::icom_civ::packet::CivPacket;
use crate::traits::*;

use super::IcomRadio;

#[async_trait]
impl RadioMeter for IcomRadio {
    async fn signal_strength(&self) -> Result<u8> {
        match self.command(CivPacket::read_smeter()).await? {
            CivCommand::SignalMeter(val) => Ok(val),
            CivCommand::NotGood => Err(RadioError::CommandFailed),
            other => Err(RadioError::Protocol(format!(
                "expected S-meter, got {:?}",
                other
            ))),
        }
    }

    async fn swr(&self) -> Result<Option<f32>> {
        read_meter(&self, CivPacket::read_swr()).await
    }

    async fn alc(&self) -> Result<Option<f32>> {
        read_meter(&self, CivPacket::read_alc()).await
    }

    async fn rf_power(&self) -> Result<Option<f32>> {
        read_meter(&self, CivPacket::read_rf_power()).await
    }
}

/// Read a meter value (SWR, ALC, RF power).
///
/// The CI-V parser currently returns these as `Unknown` since only
/// sub-command 0x02 (S-meter) is fully parsed.
async fn read_meter(radio: &IcomRadio, packet: CivPacket) -> Result<Option<f32>> {
    match radio.command(packet).await? {
        CivCommand::Unknown {
            cmd: 0x15, data, ..
        } => {
            if data.is_empty() {
                return Ok(None);
            }
            let raw = ((data[0] >> 4) * 10 + (data[0] & 0x0f)) as f32;
            Ok(Some(raw))
        }
        CivCommand::NotGood => Ok(None),
        _ => Ok(None),
    }
}
