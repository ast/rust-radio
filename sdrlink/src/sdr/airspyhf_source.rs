use std::ops::ControlFlow;
use std::sync::Arc;

use airspyhf::Device;
use tracing::{info, warn};

use crate::error::{Result, SdrLinkError};

use super::IqBroker;

/// Open an AirspyHF+, tune it, and start streaming IQ into `broker`.
///
/// The returned [`Device`] owns the USB worker thread; drop it (or call
/// [`Device::stop`]) to tear down. [`SdrHandle`] holds onto it for exactly
/// that reason.
pub fn start(
    broker: Arc<IqBroker>,
    center_hz: f64,
    samplerate: u32,
) -> Result<Device> {
    let mut device = Device::open()
        .map_err(|e| SdrLinkError::Sdr(format!("open airspyhf: {e}")))?;

    device
        .set_samplerate(samplerate)
        .map_err(|e| SdrLinkError::Sdr(format!("set samplerate {samplerate}: {e}")))?;
    device
        .set_frequency(center_hz)
        .map_err(|e| SdrLinkError::Sdr(format!("set frequency {center_hz}: {e}")))?;

    info!(center_hz, samplerate, "airspyhf streaming");

    device
        .start(move |samples, dropped| {
            if dropped > 0 {
                warn!(dropped, "airspyhf dropped samples");
            }
            broker.broadcast(samples);
            ControlFlow::Continue(())
        })
        .map_err(|e| SdrLinkError::Sdr(format!("start airspyhf: {e}")))?;

    Ok(device)
}
