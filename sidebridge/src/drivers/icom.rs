// Copyright SM6WJM 2026

mod radio;
mod radio_info;
mod radio_meter;
mod radio_scope;

use bytes::Bytes;
use futures::SinkExt;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use url::Url;

use crate::icom_civ::codec::CivCodec;
use crate::icom_civ::command::CivCommand;
use crate::icom_civ::packet::CivPacket;
use crate::icom_civ::scope::ScopeAssembler;
use crate::traits::*;
use crate::Transport;

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

struct Request {
    payload: Bytes,
    response_tx: oneshot::Sender<Result<CivCommand>>,
}

/// Icom radio driver using the CI-V protocol over serial or TCP.
///
/// Spawns a background I/O task that multiplexes command/response traffic
/// with continuous scope waveform and state data.
///
/// Unsolicited events (scope frames, frequency and mode changes) are delivered
/// through a single `RadioEvent` channel. Call `take_event_stream()` once to
/// consume it; fan-out to multiple listeners is the application's responsibility.
pub struct IcomRadio {
    id: String,
    request_tx: mpsc::Sender<Request>,
    event_rx: Mutex<Option<mpsc::Receiver<RadioEvent>>>,
}

impl IcomRadio {
    /// Connect to an Icom radio at the given URL.
    ///
    /// Supported schemes: `serial:///dev/ttyUSB0?baud=115200`, `tcp://host:port`.
    pub async fn connect(url: &Url) -> std::result::Result<Self, RadioError> {
        let transport = Transport::connect(url)
            .await
            .map_err(|e| RadioError::Io(std::io::Error::other(e)))?;
        let framed = Framed::new(transport, CivCodec);

        let (request_tx, request_rx) = mpsc::channel(16);
        let (event_tx, event_rx) = mpsc::channel(64);
        let id = url.to_string();

        tokio::spawn(Self::io_task(framed, request_rx, event_tx));

        Ok(Self {
            id,
            request_tx,
            event_rx: Mutex::new(Some(event_rx)),
        })
    }

    /// Send a CI-V command and wait for the response.
    pub(crate) async fn command(&self, packet: CivPacket) -> Result<CivCommand> {
        let (response_tx, response_rx) = oneshot::channel();
        self.request_tx
            .send(Request {
                payload: packet.payload().into(),
                response_tx,
            })
            .await
            .map_err(|_| RadioError::Io(std::io::Error::other("driver task stopped")))?;

        match tokio::time::timeout(REQUEST_TIMEOUT, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(RadioError::Io(std::io::Error::other("driver task stopped"))),
            Err(_) => Err(RadioError::Timeout("CI-V command timed out".into())),
        }
    }

    /// Send a command and expect OK/echo-back.
    pub(crate) async fn expect_ok(&self, packet: CivPacket) -> Result<()> {
        match self.command(packet).await? {
            CivCommand::NotGood => Err(RadioError::CommandFailed),
            _ => Ok(()),
        }
    }

    /// Send a CI-V command without waiting for a response.
    ///
    /// Used for fire-and-forget reads where the response arrives as a
    /// `RadioEvent` on the event stream.
    pub(crate) async fn send(&self, packet: CivPacket) -> Result<()> {
        let (response_tx, _) = oneshot::channel();
        self.request_tx
            .send(Request {
                payload: packet.payload().into(),
                response_tx,
            })
            .await
            .map_err(|_| RadioError::Io(std::io::Error::other("driver task stopped")))?;
        Ok(())
    }

    /// Background task: reads CI-V frames and dispatches them.
    ///
    /// Unsolicited events (scope, frequency, mode) are sent through the
    /// event channel. Command responses go to the pending oneshot caller.
    async fn io_task(
        mut framed: Framed<Transport, CivCodec>,
        mut request_rx: mpsc::Receiver<Request>,
        event_tx: mpsc::Sender<RadioEvent>,
    ) {
        let mut assembler = ScopeAssembler::new();
        let mut pending: Option<oneshot::Sender<Result<CivCommand>>> = None;

        loop {
            tokio::select! {
                frame = framed.next() => {
                    match frame {
                        Some(Ok(frame)) => match frame.command {
                            CivCommand::ScopeWave(wave) => {
                                if let Some(sf) = assembler.push(wave) {
                                    let _ = event_tx.try_send(RadioEvent::Scope(sf));
                                }
                            }
                            CivCommand::ScopeSetting(_) | CivCommand::ScopeRaw { .. } => {}
                            cmd => {
                                // Route events to the event channel
                                match &cmd {
                                    CivCommand::TransceiverFreq(freq) => {
                                        let _ = event_tx.try_send(RadioEvent::Frequency(*freq));
                                    }
                                    CivCommand::TransceiverMode { mode, filter } => {
                                        if let Ok(m) = mode_from_civ(*mode) {
                                            let _ = event_tx.try_send(RadioEvent::Mode(m, *filter));
                                        }
                                    }
                                    CivCommand::SignalMeter(val) => {
                                        let _ = event_tx.try_send(RadioEvent::SignalMeter(*val));
                                    }
                                    CivCommand::RfPower(val) => {
                                        let _ = event_tx.try_send(RadioEvent::RfPower(*val));
                                    }
                                    CivCommand::Swr(val) => {
                                        let _ = event_tx.try_send(RadioEvent::Swr(*val));
                                    }
                                    CivCommand::Alc(val) => {
                                        let _ = event_tx.try_send(RadioEvent::Alc(*val));
                                    }
                                    _ => {}
                                }
                                if let Some(tx) = pending.take() {
                                    let _ = tx.send(Ok(cmd));
                                }
                            }
                        },
                        Some(Err(_)) => {}
                        None => break,
                    }
                }
                req = request_rx.recv() => {
                    match req {
                        Some(request) => {
                            if framed.send(request.payload).await.is_err() {
                                let _ = request.response_tx.send(Err(RadioError::Io(
                                    std::io::Error::other("send failed"),
                                )));
                                break;
                            }
                            pending = Some(request.response_tx);
                        }
                        None => break,
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Mode mapping
// ---------------------------------------------------------------------------

pub(crate) fn mode_from_civ(byte: u8) -> Result<Mode> {
    match byte {
        0x00 => Ok(Mode::Lsb),
        0x01 => Ok(Mode::Usb),
        0x02 => Ok(Mode::Am),
        0x03 => Ok(Mode::Cw),
        0x04 => Ok(Mode::Rtty),
        0x05 => Ok(Mode::Fm),
        0x07 => Ok(Mode::CwR),
        0x08 => Ok(Mode::RttyR),
        _ => Err(RadioError::Protocol(format!(
            "unknown CI-V mode {:#04x}",
            byte
        ))),
    }
}

pub(crate) fn mode_to_civ(mode: &Mode) -> u8 {
    match mode {
        Mode::Lsb | Mode::DataLsb => 0x00,
        Mode::Usb | Mode::DataUsb => 0x01,
        Mode::Am => 0x02,
        Mode::Cw => 0x03,
        Mode::Rtty => 0x04,
        Mode::Fm | Mode::DataFm => 0x05,
        Mode::CwR => 0x07,
        Mode::RttyR => 0x08,
    }
}
