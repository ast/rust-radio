// Copyright (c) SM6WJM 2026

/// Typed CI-V command identifiers.
///
/// Each variant encodes the command byte and optional subcommand byte
/// for one logical CI-V operation. Use [`cmd()`] and [`subcmd()`] to
/// get the wire representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivCommandCode {
    // Frequency / Mode (no subcommand)
    SendFrequency,     // 0x00 — transceive
    SendMode,          // 0x01 — transceive
    ReadBandEdge,      // 0x02
    ReadFrequency,     // 0x03
    ReadMode,          // 0x04
    SetFrequency,      // 0x05
    SetMode,           // 0x06

    // VFO (cmd 0x07)
    SelectVfoA,        // 0x07 0x00
    SelectVfoB,        // 0x07 0x01
    EqualizeVfo,       // 0x07 0xA0
    ExchangeVfo,       // 0x07 0xB0

    // Memory (cmd 0x08, 0x09, 0x0A, 0x0B)
    SelectMemory,      // 0x08
    MemoryWrite,       // 0x09
    MemoryCopyToVfo,   // 0x0A
    MemoryClear,       // 0x0B

    // Scan (cmd 0x0E)
    CancelScan,        // 0x0E 0x00
    StartProgramScan,  // 0x0E 0x01
    StartDeltaFScan,   // 0x0E 0x03

    // Levels (cmd 0x14)
    RfGain,            // 0x14 0x02

    // Meters (cmd 0x15)
    ReadSquelchStatus, // 0x15 0x01
    ReadSmeter,        // 0x15 0x02
    ReadOvfStatus,     // 0x15 0x07
    ReadRfPower,       // 0x15 0x11
    ReadSwr,           // 0x15 0x12
    ReadAlc,           // 0x15 0x13
    ReadCompMeter,     // 0x15 0x14
    ReadVd,            // 0x15 0x15
    ReadId,            // 0x15 0x16

    // Settings (cmd 0x16)
    Preamp,            // 0x16 0x02
    AgcTime,           // 0x16 0x12
    NoiseBlanker,      // 0x16 0x22
    NoiseReduction,    // 0x16 0x40
    AutoNotch,         // 0x16 0x41

    // Transceiver power (cmd 0x18)
    PowerOff,          // 0x18 0x00
    PowerOn,           // 0x18 0x01

    // Transceiver ID (cmd 0x19)
    ReadTransceiverId, // 0x19 0x00

    // PTT / Tuner (cmd 0x1C)
    Ptt,               // 0x1C 0x00
    AntennaTuner,      // 0x1C 0x01

    // Scope (cmd 0x27)
    ScopeWaveData,     // 0x27 0x00
    ScopeOnOff,        // 0x27 0x10
    ScopeWaveOutput,   // 0x27 0x11
    ScopeCenterFixed,  // 0x27 0x14
    ScopeSpan,         // 0x27 0x15
    ScopeFixedEdge,    // 0x27 0x16
    ScopeHold,         // 0x27 0x17
    ScopeSweepSpeed,   // 0x27 0x1A

    // Response codes
    Ok,                // 0xFB
    NotGood,           // 0xFA
}

impl CivCommandCode {
    /// Command byte on the wire.
    pub fn cmd(&self) -> u8 {
        match self {
            Self::SendFrequency => 0x00,
            Self::SendMode => 0x01,
            Self::ReadBandEdge => 0x02,
            Self::ReadFrequency => 0x03,
            Self::ReadMode => 0x04,
            Self::SetFrequency => 0x05,
            Self::SetMode => 0x06,

            Self::SelectVfoA | Self::SelectVfoB | Self::EqualizeVfo | Self::ExchangeVfo => 0x07,

            Self::SelectMemory => 0x08,
            Self::MemoryWrite => 0x09,
            Self::MemoryCopyToVfo => 0x0a,
            Self::MemoryClear => 0x0b,

            Self::CancelScan | Self::StartProgramScan | Self::StartDeltaFScan => 0x0e,

            Self::RfGain => 0x14,

            Self::ReadSquelchStatus
            | Self::ReadSmeter
            | Self::ReadOvfStatus
            | Self::ReadRfPower
            | Self::ReadSwr
            | Self::ReadAlc
            | Self::ReadCompMeter
            | Self::ReadVd
            | Self::ReadId => 0x15,

            Self::Preamp
            | Self::AgcTime
            | Self::NoiseBlanker
            | Self::NoiseReduction
            | Self::AutoNotch => 0x16,

            Self::PowerOff | Self::PowerOn => 0x18,

            Self::ReadTransceiverId => 0x19,

            Self::Ptt | Self::AntennaTuner => 0x1c,

            Self::ScopeWaveData
            | Self::ScopeOnOff
            | Self::ScopeWaveOutput
            | Self::ScopeCenterFixed
            | Self::ScopeSpan
            | Self::ScopeFixedEdge
            | Self::ScopeHold
            | Self::ScopeSweepSpeed => 0x27,

            Self::Ok => 0xfb,
            Self::NotGood => 0xfa,
        }
    }

    /// Subcommand byte on the wire, if any.
    pub fn subcmd(&self) -> Option<u8> {
        match self {
            // No subcommand
            Self::SendFrequency
            | Self::SendMode
            | Self::ReadBandEdge
            | Self::ReadFrequency
            | Self::ReadMode
            | Self::SetFrequency
            | Self::SetMode
            | Self::SelectMemory
            | Self::MemoryWrite
            | Self::MemoryCopyToVfo
            | Self::MemoryClear
            | Self::Ok
            | Self::NotGood => None,

            // Levels
            Self::RfGain => Some(0x02),

            // Scope
            Self::ScopeWaveData => Some(0x00),
            Self::ScopeOnOff => Some(0x10),
            Self::ScopeWaveOutput => Some(0x11),
            Self::ScopeCenterFixed => Some(0x14),
            Self::ScopeSpan => Some(0x15),
            Self::ScopeFixedEdge => Some(0x16),
            Self::ScopeHold => Some(0x17),
            Self::ScopeSweepSpeed => Some(0x1a),

            // VFO
            Self::SelectVfoA => Some(0x00),
            Self::SelectVfoB => Some(0x01),
            Self::EqualizeVfo => Some(0xa0),
            Self::ExchangeVfo => Some(0xb0),

            // Scan
            Self::CancelScan => Some(0x00),
            Self::StartProgramScan => Some(0x01),
            Self::StartDeltaFScan => Some(0x03),

            // Meters
            Self::ReadSquelchStatus => Some(0x01),
            Self::ReadSmeter => Some(0x02),
            Self::ReadOvfStatus => Some(0x07),
            Self::ReadRfPower => Some(0x11),
            Self::ReadSwr => Some(0x12),
            Self::ReadAlc => Some(0x13),
            Self::ReadCompMeter => Some(0x14),
            Self::ReadVd => Some(0x15),
            Self::ReadId => Some(0x16),

            // Settings
            Self::Preamp => Some(0x02),
            Self::AgcTime => Some(0x12),
            Self::NoiseBlanker => Some(0x22),
            Self::NoiseReduction => Some(0x40),
            Self::AutoNotch => Some(0x41),

            // Power
            Self::PowerOff => Some(0x00),
            Self::PowerOn => Some(0x01),

            // ID
            Self::ReadTransceiverId => Some(0x00),

            // PTT / Tuner
            Self::Ptt => Some(0x00),
            Self::AntennaTuner => Some(0x01),
        }
    }

    /// Decode from wire bytes. Returns `None` for unrecognized combinations.
    pub fn from_bytes(cmd: u8, subcmd: Option<u8>) -> Option<Self> {
        match (cmd, subcmd) {
            (0x00, _) => Some(Self::SendFrequency),
            (0x01, _) => Some(Self::SendMode),
            (0x02, _) => Some(Self::ReadBandEdge),
            (0x03, _) => Some(Self::ReadFrequency),
            (0x04, _) => Some(Self::ReadMode),
            (0x05, _) => Some(Self::SetFrequency),
            (0x06, _) => Some(Self::SetMode),

            (0x07, Some(0x00) | None) => Some(Self::SelectVfoA),
            (0x07, Some(0x01)) => Some(Self::SelectVfoB),
            (0x07, Some(0xa0)) => Some(Self::EqualizeVfo),
            (0x07, Some(0xb0)) => Some(Self::ExchangeVfo),

            (0x08, _) => Some(Self::SelectMemory),
            (0x09, _) => Some(Self::MemoryWrite),
            (0x0a, _) => Some(Self::MemoryCopyToVfo),
            (0x0b, _) => Some(Self::MemoryClear),

            (0x0e, Some(0x00) | None) => Some(Self::CancelScan),
            (0x0e, Some(0x01)) => Some(Self::StartProgramScan),
            (0x0e, Some(0x03)) => Some(Self::StartDeltaFScan),

            (0x14, Some(0x02)) => Some(Self::RfGain),

            (0x15, Some(0x01)) => Some(Self::ReadSquelchStatus),
            (0x15, Some(0x02)) => Some(Self::ReadSmeter),
            (0x15, Some(0x07)) => Some(Self::ReadOvfStatus),
            (0x15, Some(0x11)) => Some(Self::ReadRfPower),
            (0x15, Some(0x12)) => Some(Self::ReadSwr),
            (0x15, Some(0x13)) => Some(Self::ReadAlc),
            (0x15, Some(0x14)) => Some(Self::ReadCompMeter),
            (0x15, Some(0x15)) => Some(Self::ReadVd),
            (0x15, Some(0x16)) => Some(Self::ReadId),

            (0x16, Some(0x02)) => Some(Self::Preamp),
            (0x16, Some(0x12)) => Some(Self::AgcTime),
            (0x16, Some(0x22)) => Some(Self::NoiseBlanker),
            (0x16, Some(0x40)) => Some(Self::NoiseReduction),
            (0x16, Some(0x41)) => Some(Self::AutoNotch),

            (0x18, Some(0x00) | None) => Some(Self::PowerOff),
            (0x18, Some(0x01)) => Some(Self::PowerOn),

            (0x19, Some(0x00) | None) => Some(Self::ReadTransceiverId),

            (0x1c, Some(0x00) | None) => Some(Self::Ptt),
            (0x1c, Some(0x01)) => Some(Self::AntennaTuner),

            (0x27, Some(0x00)) => Some(Self::ScopeWaveData),
            (0x27, Some(0x10)) => Some(Self::ScopeOnOff),
            (0x27, Some(0x11)) => Some(Self::ScopeWaveOutput),
            (0x27, Some(0x14)) => Some(Self::ScopeCenterFixed),
            (0x27, Some(0x15)) => Some(Self::ScopeSpan),
            (0x27, Some(0x16)) => Some(Self::ScopeFixedEdge),
            (0x27, Some(0x17)) => Some(Self::ScopeHold),
            (0x27, Some(0x1a)) => Some(Self::ScopeSweepSpeed),

            (0xfb, _) => Some(Self::Ok),
            (0xfa, _) => Some(Self::NotGood),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_simple() {
        let code = CivCommandCode::ReadFrequency;
        assert_eq!(code.cmd(), 0x03);
        assert_eq!(code.subcmd(), None);
        assert_eq!(CivCommandCode::from_bytes(0x03, None), Some(code));
    }

    #[test]
    fn test_roundtrip_with_subcmd() {
        let code = CivCommandCode::ReadSmeter;
        assert_eq!(code.cmd(), 0x15);
        assert_eq!(code.subcmd(), Some(0x02));
        assert_eq!(CivCommandCode::from_bytes(0x15, Some(0x02)), Some(code));
    }

    #[test]
    fn test_roundtrip_all_variants() {
        let all = [
            CivCommandCode::SendFrequency,
            CivCommandCode::SendMode,
            CivCommandCode::ReadBandEdge,
            CivCommandCode::ReadFrequency,
            CivCommandCode::ReadMode,
            CivCommandCode::SetFrequency,
            CivCommandCode::SetMode,
            CivCommandCode::SelectVfoA,
            CivCommandCode::SelectVfoB,
            CivCommandCode::EqualizeVfo,
            CivCommandCode::ExchangeVfo,
            CivCommandCode::SelectMemory,
            CivCommandCode::MemoryWrite,
            CivCommandCode::MemoryCopyToVfo,
            CivCommandCode::MemoryClear,
            CivCommandCode::CancelScan,
            CivCommandCode::StartProgramScan,
            CivCommandCode::StartDeltaFScan,
            CivCommandCode::RfGain,
            CivCommandCode::ReadSquelchStatus,
            CivCommandCode::ReadSmeter,
            CivCommandCode::ReadOvfStatus,
            CivCommandCode::ReadRfPower,
            CivCommandCode::ReadSwr,
            CivCommandCode::ReadAlc,
            CivCommandCode::ReadCompMeter,
            CivCommandCode::ReadVd,
            CivCommandCode::ReadId,
            CivCommandCode::Preamp,
            CivCommandCode::AgcTime,
            CivCommandCode::NoiseBlanker,
            CivCommandCode::NoiseReduction,
            CivCommandCode::AutoNotch,
            CivCommandCode::PowerOff,
            CivCommandCode::PowerOn,
            CivCommandCode::ReadTransceiverId,
            CivCommandCode::Ptt,
            CivCommandCode::AntennaTuner,
            CivCommandCode::ScopeWaveData,
            CivCommandCode::ScopeOnOff,
            CivCommandCode::ScopeWaveOutput,
            CivCommandCode::ScopeCenterFixed,
            CivCommandCode::ScopeSpan,
            CivCommandCode::ScopeFixedEdge,
            CivCommandCode::ScopeHold,
            CivCommandCode::ScopeSweepSpeed,
            CivCommandCode::Ok,
            CivCommandCode::NotGood,
        ];
        for code in all {
            let decoded = CivCommandCode::from_bytes(code.cmd(), code.subcmd());
            assert_eq!(decoded, Some(code), "roundtrip failed for {:?}", code);
        }
    }

    #[test]
    fn test_unknown_returns_none() {
        assert_eq!(CivCommandCode::from_bytes(0xff, None), None);
        assert_eq!(CivCommandCode::from_bytes(0x15, Some(0xff)), None);
    }
}
