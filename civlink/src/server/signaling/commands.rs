// Copyright SM6WJM 2026

use sidebridge::{Radio, RadioCommand, RadioGain, RadioScope};

use crate::radio::RadioHandle;

/// Dispatch a single `RadioCommand` from the frontend to the connected radio.
/// Errors are logged and swallowed — a bad command should not tear down the
/// signaling session.
pub async fn dispatch(radio: &RadioHandle, cmd: RadioCommand) {
    match cmd {
        RadioCommand::SetFrequency(hz) => set_frequency(radio, hz).await,
        RadioCommand::SetMode(mode) => set_mode(radio, mode).await,
        RadioCommand::SetScopeSpan(hz) => {
            log_err("set_scope_span", radio.radio().set_scope_span(hz).await);
        }
        RadioCommand::SetScopeMode(mode) => {
            log_err("set_scope_mode", radio.radio().set_scope_mode(mode).await);
        }
        RadioCommand::SetScopeFixedEdge(edge) => {
            log_err("set_scope_fixed_edge", radio.radio().set_scope_fixed_edge(edge).await);
        }
        RadioCommand::SetRfGain(val) => {
            if log_err("set_rf_gain", radio.radio().set_rf_gain(val).await) {
                let _ = radio.radio().read_rf_gain().await;
            }
        }
    }
}

async fn set_frequency(radio: &RadioHandle, hz: u64) {
    if log_err("set_frequency", radio.radio().set_frequency(hz).await) {
        // set_frequency doesn't echo back; read to trigger a RadioEvent
        let _ = radio.radio().read_frequency().await;
    }
}

async fn set_mode(radio: &RadioHandle, mode: sidebridge::Mode) {
    if log_err("set_mode", radio.radio().set_mode(mode).await) {
        let _ = radio.radio().read_mode().await;
    }
}

/// Returns true on success so the caller can chain follow-up reads.
fn log_err<T, E: std::fmt::Display>(op: &str, result: Result<T, E>) -> bool {
    match result {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("{op} failed: {e}");
            false
        }
    }
}
