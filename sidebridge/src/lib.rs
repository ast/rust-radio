// Copyright SM6WJM 2026

pub mod drivers;
pub mod icom_civ;
pub mod traits;
pub mod transport;

// Trait and type re-exports
pub use traits::{
    Capabilities, Mode, Radio, RadioError, RadioInfo, RadioMeter, RadioScope, Result, ScopeFrame,
};

// CI-V re-exports
pub use icom_civ::command::CivCommand;
pub use icom_civ::command_code::CivCommandCode;
pub use icom_civ::frame::CivFrame;
pub use icom_civ::scope::{ScopeAssembler, ScopeFreqInfo, ScopeMode, ScopeSetting, ScopeWaveData};
pub use transport::Transport;

// Driver re-exports
pub use drivers::icom::IcomRadio;
