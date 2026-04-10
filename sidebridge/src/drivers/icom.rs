// Copyright SM6WJM 2026

mod radio;
mod radio_info;
mod radio_meter;
mod radio_scope;

use bytes::Bytes;
use futures::SinkExt;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tokio_util::codec::Framed;
use url::Url;

use crate::icom_civ::codec::CivCodec;
use crate::icom_civ::command::CivCommand;
use crate::icom_civ::packet::CivPacket;
use crate::icom_civ::scope::ScopeAssembler;
use crate::traits::*;
use crate::Transport;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

struct Request {
    payload: Bytes,
    response_tx: oneshot::Sender<Result<CivCommand>>,
}

/// Icom radio driver using the CI-V protocol over serial or TCP.
///
/// Spawns a background I/O task that multiplexes command/response traffic
/// with continuous scope waveform data.
pub struct IcomRadio {
    id: String,
    request_tx: mpsc::Sender<Request>,
    pub(crate) scope_tx: broadcast::Sender<ScopeFrame>,
    freq_tx: broadcast::Sender<u64>,
    mode_tx: broadcast::Sender<(Mode, u8)>,
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
        let (scope_tx, _) = broadcast::channel(64);
        let (freq_tx, _) = broadcast::channel(16);
        let (mode_tx, _) = broadcast::channel(16);
        let id = url.to_string();

        tokio::spawn(Self::io_task(
            framed,
            request_rx,
            scope_tx.clone(),
            freq_tx.clone(),
            mode_tx.clone(),
        ));

        Ok(Self {
            id,
            request_tx,
            scope_tx,
            freq_tx,
            mode_tx,
        })
    }

    /// Enable or disable scope waveform output on the radio.
    pub async fn set_scope_output(&self, enable: bool) -> Result<()> {
        if enable {
            self.expect_ok(CivPacket::scope_on_off(true)).await?;
            self.expect_ok(CivPacket::scope_wave_output(true)).await?;
        } else {
            self.expect_ok(CivPacket::scope_wave_output(false)).await?;
        }
        Ok(())
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

    /// Background task: reads CI-V frames and dispatches them.
    ///
    /// Scope waveform data is assembled and broadcast to subscribers.
    /// All other responses are forwarded to the pending request caller.
    /// Subscribe to frequency updates (both solicited and unsolicited).
    pub fn freq_stream(
        &self,
    ) -> Box<dyn tokio_stream::Stream<Item = u64> + Send + Unpin> {
        let rx = self.freq_tx.subscribe();
        Box::new(tokio_stream::StreamExt::filter_map(
            BroadcastStream::new(rx),
            |r: std::result::Result<u64, _>| r.ok(),
        ))
    }

    /// Subscribe to mode updates (both solicited and unsolicited).
    /// Returns (Mode, filter_width) tuples.
    pub fn mode_stream(
        &self,
    ) -> Box<dyn tokio_stream::Stream<Item = (Mode, u8)> + Send + Unpin> {
        let rx = self.mode_tx.subscribe();
        Box::new(tokio_stream::StreamExt::filter_map(
            BroadcastStream::new(rx),
            |r: std::result::Result<(Mode, u8), _>| r.ok(),
        ))
    }

    /// Background task: reads CI-V frames and dispatches them.
    ///
    /// Scope waveform data is assembled and broadcast to subscribers.
    /// Frequency updates are always broadcast (including unsolicited changes).
    /// All other responses are forwarded to the pending request caller.
    async fn io_task(
        mut framed: Framed<Transport, CivCodec>,
        mut request_rx: mpsc::Receiver<Request>,
        scope_tx: broadcast::Sender<ScopeFrame>,
        freq_tx: broadcast::Sender<u64>,
        mode_tx: broadcast::Sender<(Mode, u8)>,
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
                                    let _ = scope_tx.send(sf);
                                }
                            }
                            CivCommand::ScopeSetting(_) | CivCommand::ScopeRaw { .. } => {}
                            cmd => {
                                // Broadcast frequency updates to all subscribers
                                if let CivCommand::TransceiverFreq(freq) = &cmd {
                                    let _ = freq_tx.send(*freq);
                                }
                                // Broadcast mode updates to all subscribers
                                if let CivCommand::TransceiverMode { mode, filter } = &cmd {
                                    if let Ok(m) = mode_from_civ(*mode) {
                                        let _ = mode_tx.send((m, *filter));
                                    }
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
