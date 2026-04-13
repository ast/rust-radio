// Copyright SM6WJM 2026

pub mod drivers;
pub mod traits;
pub mod transport;

// Trait and type re-exports
pub use traits::{
    Capabilities, Mode, Radio, RadioCommand, RadioError, RadioEvent, RadioInfo, RadioMeter,
    RadioScope, Result, ScopeFrame, ScopeFreq, MAX_SCOPE_BINS,
};

// CI-V re-exports
pub use drivers::icom::civ::codec::CivCodec;
pub use drivers::icom::civ::command::CivCommand;
pub use drivers::icom::civ::command_code::CivCommandCode;
pub use drivers::icom::civ::frame::CivFrame;
pub use drivers::icom::civ::scope::{
    ScopeAssembler, ScopeFreqInfo, ScopeMode, ScopeSetting, ScopeWaveData,
};
pub use transport::Transport;

// Driver re-exports
pub use drivers::icom::IcomRadio;
