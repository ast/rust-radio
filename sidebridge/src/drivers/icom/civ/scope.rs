// Copyright (c) SM6WJM 2026

use bytes::Bytes;
use thiserror::Error;

use arrayvec::ArrayVec;

use super::parser::parse_bcd_to_u64;
use crate::traits::{ScopeFrame, ScopeFreq, ScopeMode, MAX_SCOPE_BINS};

/// Number of amplitude bins in a complete IC-705 scope frame.
pub const SCOPE_BINS: usize = 475;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Decode a scope mode byte from a CI-V frame.
pub fn scope_mode_from_byte(b: u8) -> Result<ScopeMode, ScopeParseError> {
    match b {
        0x00 => Ok(ScopeMode::Center),
        0x01 => Ok(ScopeMode::Fixed),
        other => Err(ScopeParseError::InvalidMode(other)),
    }
}

/// Frequency information from the waveform header.
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeFreqInfo {
    Center {
        center_hz: u64,
        /// Span in Hz (e.g. 100_000 for 100 kHz).
        span_hz: u64,
    },
    Fixed {
        lower_edge_hz: u64,
        upper_edge_hz: u64,
    },
}

/// A parsed waveform division from CI-V command 0x27 sub 0x00.
///
/// The IC-705 sends scope data in divisions:
/// - WLAN: 1 division with header + all 475 bins
/// - USB: 11 divisions — division 1 is header (no bins), divisions 2-11 carry bins
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeWaveData {
    /// Current division number (1-based).
    pub division: u8,
    /// Maximum number of divisions (1 for WLAN, 11 for USB).
    pub max_division: u8,
    /// Scope mode — only present in the header division.
    pub mode: Option<ScopeMode>,
    /// Frequency info — only present in the header division.
    pub freq_info: Option<ScopeFreqInfo>,
    /// Out-of-range flag — only present in the header division.
    pub out_of_range: Option<bool>,
    /// Waveform amplitude bins (0-160 range).
    /// Empty in USB header division, present otherwise.
    pub bins: Bytes,
}

/// Parsed scope setting from CI-V command 0x27 sub 0x10-0x1E.
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeSetting {
    /// Scope ON/OFF (sub 0x10).
    ScopeOnOff(bool),
    /// Waveform data output ON/OFF (sub 0x11).
    WaveOutputOnOff(bool),
    /// Center/Fixed mode (sub 0x14).
    CenterFixedMode(ScopeMode),
    /// Span in Hz (sub 0x15).
    Span(u64),
    /// Hold ON/OFF (sub 0x17).
    Hold(bool),
    /// Sweep speed: 0=FAST, 1=MID, 2=SLOW (sub 0x1A).
    SweepSpeed(u8),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Error, Debug)]
pub enum ScopeParseError {
    #[error("insufficient data: need {need} bytes, got {got}")]
    InsufficientData { need: usize, got: usize },
    #[error("invalid scope mode byte: {0:#04x}")]
    InvalidMode(u8),
}

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse waveform data from the bytes AFTER sub_cmd 0x00.
///
/// The input `data` starts at field 1 (a fixed 0x00 byte), followed by
/// the division byte (field 2) and max_division (field 3).
pub fn parse_scope_wave(data: Bytes) -> Result<ScopeWaveData, ScopeParseError> {
    // Field 1 is a fixed 0x00 byte — skip it
    if data.len() < 3 {
        return Err(ScopeParseError::InsufficientData { need: 3, got: data.len() });
    }

    let division = parse_bcd_u8(data[1]);
    let max_division = parse_bcd_u8(data[2]);

    let is_header = division == 1;
    let is_wlan = max_division == 1;

    if is_header {
        // Header division: fixed(1) + div(1) + max_div(1) + mode(1) = 4 bytes minimum
        if data.len() < 4 {
            return Err(ScopeParseError::InsufficientData { need: 4, got: data.len() });
        }

        let mode = scope_mode_from_byte(data[3])?;
        let (freq_info, freq_len) = parse_freq_info(mode, &data[4..])?;
        let after_freq = 4 + freq_len;

        // Out-of-range byte
        if data.len() <= after_freq {
            return Err(ScopeParseError::InsufficientData {
                need: after_freq + 1,
                got: data.len(),
            });
        }
        let out_of_range = data[after_freq] != 0x00;
        let bins_start = after_freq + 1;

        // WLAN single frame includes bins; USB header does not
        let bins = if is_wlan { data.slice(bins_start..) } else { Bytes::new() };

        Ok(ScopeWaveData {
            division,
            max_division,
            mode: Some(mode),
            freq_info: Some(freq_info),
            out_of_range: Some(out_of_range),
            bins,
        })
    } else {
        // Data division: skip fixed(1) + div(1) + max_div(1), rest is bins
        let bins = data.slice(3..);

        Ok(ScopeWaveData {
            division,
            max_division,
            mode: None,
            freq_info: None,
            out_of_range: None,
            bins,
        })
    }
}

/// Parse frequency info from the header based on scope mode.
/// Returns the parsed info and the number of bytes consumed.
fn parse_freq_info(
    mode: ScopeMode,
    data: &[u8],
) -> Result<(ScopeFreqInfo, usize), ScopeParseError> {
    match mode {
        ScopeMode::Center => {
            // 5 bytes center frequency + 5 bytes span = 10 bytes
            if data.len() < 10 {
                return Err(ScopeParseError::InsufficientData { need: 10, got: data.len() });
            }
            let center_hz = parse_bcd_to_u64(&data[..5]);
            let span_hz = parse_bcd_to_u64(&data[5..10]);
            Ok((ScopeFreqInfo::Center { center_hz, span_hz }, 10))
        }
        ScopeMode::Fixed => {
            // 5 bytes lower edge + 5 bytes upper edge = 10 bytes
            if data.len() < 10 {
                return Err(ScopeParseError::InsufficientData { need: 10, got: data.len() });
            }
            let lower_edge_hz = parse_bcd_to_u64(&data[..5]);
            let upper_edge_hz = parse_bcd_to_u64(&data[5..10]);
            Ok((ScopeFreqInfo::Fixed { lower_edge_hz, upper_edge_hz }, 10))
        }
    }
}

/// Parse a scope setting from its sub_cmd byte and data.
pub fn parse_scope_setting(sub_cmd: u8, data: &[u8]) -> Result<ScopeSetting, ScopeParseError> {
    match sub_cmd {
        0x10 => {
            // Scope ON/OFF: 1 byte
            if data.is_empty() {
                return Err(ScopeParseError::InsufficientData { need: 1, got: 0 });
            }
            Ok(ScopeSetting::ScopeOnOff(data[0] != 0x00))
        }
        0x11 => {
            // Wave data output ON/OFF: 1 byte
            if data.is_empty() {
                return Err(ScopeParseError::InsufficientData { need: 1, got: 0 });
            }
            Ok(ScopeSetting::WaveOutputOnOff(data[0] != 0x00))
        }
        0x14 => {
            // Center/Fixed mode: 2 BCD bytes (0x00 0x00 = center, 0x00 0x01 = fixed)
            if data.len() < 2 {
                return Err(ScopeParseError::InsufficientData { need: 2, got: data.len() });
            }
            let mode = scope_mode_from_byte(data[1])?;
            Ok(ScopeSetting::CenterFixedMode(mode))
        }
        0x15 => {
            // Span response: <selector=0x00> <5 BCD bytes> (same BCD layout as
            // the frequency command, covering 1 Hz through 1 GHz digits).
            if data.len() < 6 {
                return Err(ScopeParseError::InsufficientData { need: 6, got: data.len() });
            }
            let span_hz = parse_bcd_to_u64(&data[1..6]);
            Ok(ScopeSetting::Span(span_hz))
        }
        0x17 => {
            // Hold: 2 bytes (0x00 0x00 = off, 0x00 0x01 = on)
            if data.len() < 2 {
                return Err(ScopeParseError::InsufficientData { need: 2, got: data.len() });
            }
            Ok(ScopeSetting::Hold(data[1] != 0x00))
        }
        0x1a => {
            // Sweep speed: 2 bytes (0x00 + speed)
            if data.len() < 2 {
                return Err(ScopeParseError::InsufficientData { need: 2, got: data.len() });
            }
            Ok(ScopeSetting::SweepSpeed(data[1]))
        }
        _ => Err(ScopeParseError::InsufficientData { need: 0, got: 0 }),
    }
}

/// Decode a single BCD byte to its decimal value.
fn parse_bcd_u8(byte: u8) -> u8 {
    ((byte >> 4) * 10) + (byte & 0x0f)
}

// ---------------------------------------------------------------------------
// Assembler
// ---------------------------------------------------------------------------

/// Collects multi-division scope frames into a complete `ScopeFrame`.
///
/// USB transport sends 11 divisions: division 1 is the header (no bins),
/// divisions 2-11 carry amplitude bins. WLAN sends a single division
/// with header + all bins.
pub struct ScopeAssembler {
    header: Option<ScopeWaveData>,
    bin_slots: Vec<Bytes>,
    expected_divisions: u8,
    received_mask: u16,
}

impl ScopeAssembler {
    pub fn new() -> Self {
        Self {
            header: None,
            bin_slots: Vec::new(),
            expected_divisions: 0,
            received_mask: 0,
        }
    }

    /// Feed a parsed `ScopeWaveData` division. Returns `Some(ScopeFrame)` when
    /// all divisions have been received and the frame is complete.
    pub fn push(&mut self, data: ScopeWaveData) -> Option<ScopeFrame> {
        let max_div = data.max_division;

        // WLAN single-frame: return immediately
        if max_div == 1 && data.division == 1 {
            return self.build_frame(&data, &data.bins[..]);
        }

        // USB multi-division
        if data.division == 1 {
            // New sweep — discard any partial frame
            self.reset();
            self.expected_divisions = max_div;
            self.bin_slots = vec![Bytes::new(); max_div as usize];
            self.header = Some(data);
            self.received_mask = 1; // bit 0 = division 1
            return None;
        }

        // Data division (2-11)
        if self.header.is_none() || max_div != self.expected_divisions {
            // No header yet or mismatched sweep — discard
            return None;
        }

        let idx = data.division as usize;
        if idx <= self.bin_slots.len() {
            self.bin_slots[idx - 1] = data.bins;
            self.received_mask |= 1 << (data.division - 1);
        }

        // Check if all divisions received
        let all_mask = (1u16 << max_div) - 1;
        if self.received_mask == all_mask {
            // Assemble bins in division order (skip division 1 which has no bins)
            let total: usize = self.bin_slots[1..].iter().map(|b| b.len()).sum();
            let mut bins: Vec<u8> = Vec::with_capacity(total);
            for slot in &self.bin_slots[1..] {
                bins.extend_from_slice(slot);
            }
            let header = self.header.take().unwrap();
            self.reset();
            return self.build_frame(&header, &bins);
        }

        None
    }

    /// Reset the assembler, discarding any partial frame.
    pub fn reset(&mut self) {
        self.header = None;
        self.bin_slots.clear();
        self.expected_divisions = 0;
        self.received_mask = 0;
    }

    fn build_frame(&self, header: &ScopeWaveData, bins: &[u8]) -> Option<ScopeFrame> {
        let freq_info = header.freq_info.as_ref()?;
        let freq = match freq_info {
            ScopeFreqInfo::Center { center_hz, span_hz } => ScopeFreq::Center {
                center_hz: *center_hz,
                span_hz: *span_hz,
            },
            ScopeFreqInfo::Fixed { lower_edge_hz, upper_edge_hz } => ScopeFreq::Fixed {
                lower_hz: *lower_edge_hz,
                upper_hz: *upper_edge_hz,
            },
        };
        let mut arr = ArrayVec::<u8, MAX_SCOPE_BINS>::new();
        let len = bins.len().min(MAX_SCOPE_BINS);
        // SAFETY: we just clamped len to capacity
        unsafe { arr.set_len(len); }
        arr[..len].copy_from_slice(&bins[..len]);
        Some(ScopeFrame { freq, bins: arr })
    }
}

impl Default for ScopeAssembler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a frequency as 5 BCD bytes (little-endian, Icom format).
    fn encode_freq_bcd(mut freq: u64) -> Vec<u8> {
        let mut buf = vec![0u8; 5];
        for b in buf.iter_mut() {
            let low = (freq % 10) as u8;
            freq /= 10;
            let high = (freq % 10) as u8;
            freq /= 10;
            *b = (high << 4) | low;
        }
        buf
    }

    /// Encode a span as 5 BCD bytes (matches waveform header format).
    fn encode_span_bcd(span: u64) -> Vec<u8> {
        encode_bcd_n(span, 5)
    }

    /// Encode a value as N BCD bytes (little-endian).
    fn encode_bcd_n(mut val: u64, n: usize) -> Vec<u8> {
        let mut buf = vec![0u8; n];
        for b in buf.iter_mut() {
            let low = (val % 10) as u8;
            val /= 10;
            let high = (val % 10) as u8;
            val /= 10;
            *b = (high << 4) | low;
        }
        buf
    }

    /// Encode a u8 value as BCD.
    fn encode_bcd_u8(val: u8) -> u8 {
        ((val / 10) << 4) | (val % 10)
    }

    /// Build a center-mode header division (no bins).
    fn make_center_header(division: u8, max_div: u8, freq: u64, span: u64, out_of_range: bool) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0x00); // fixed field ①
        data.push(encode_bcd_u8(division));
        data.push(encode_bcd_u8(max_div));
        data.push(0x00); // center mode
        data.extend_from_slice(&encode_freq_bcd(freq));
        data.extend_from_slice(&encode_span_bcd(span));
        data.push(if out_of_range { 0x01 } else { 0x00 });
        data
    }

    /// Build a data division with bins.
    fn make_data_division(division: u8, max_div: u8, bins: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0x00); // fixed field ①
        data.push(encode_bcd_u8(division));
        data.push(encode_bcd_u8(max_div));
        data.extend_from_slice(bins);
        data
    }

    // --- parse_scope_wave tests ---

    #[test]
    fn test_parse_wave_center_wlan() {
        let freq = 14_200_000u64;
        let span = 100_000u64;
        let bins: Vec<u8> = (0..SCOPE_BINS).map(|i| (i % 161) as u8).collect();

        let mut data = make_center_header(1, 1, freq, span, false);
        data.extend_from_slice(&bins);

        let result = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        assert_eq!(result.division, 1);
        assert_eq!(result.max_division, 1);
        assert_eq!(result.mode, Some(ScopeMode::Center));
        assert_eq!(
            result.freq_info,
            Some(ScopeFreqInfo::Center { center_hz: freq, span_hz: span })
        );
        assert_eq!(result.out_of_range, Some(false));
        assert_eq!(result.bins.len(), SCOPE_BINS);
        assert_eq!(result.bins, bins);
    }

    #[test]
    fn test_parse_wave_center_usb_header() {
        let freq = 7_074_000u64;
        let span = 50_000u64;

        let data = make_center_header(1, 11, freq, span, false);
        let result = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();

        assert_eq!(result.division, 1);
        assert_eq!(result.max_division, 11);
        assert_eq!(result.mode, Some(ScopeMode::Center));
        assert_eq!(
            result.freq_info,
            Some(ScopeFreqInfo::Center { center_hz: freq, span_hz: span })
        );
        assert_eq!(result.out_of_range, Some(false));
        assert!(result.bins.is_empty());
    }

    #[test]
    fn test_parse_wave_usb_data_division() {
        let bins: Vec<u8> = vec![42; 48];
        let data = make_data_division(5, 11, &bins);

        let result = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        assert_eq!(result.division, 5);
        assert_eq!(result.max_division, 11);
        assert_eq!(result.mode, None);
        assert_eq!(result.freq_info, None);
        assert_eq!(result.out_of_range, None);
        assert_eq!(result.bins, bins);
    }

    #[test]
    fn test_parse_wave_fixed_mode() {
        let lower = 7_000_000u64;
        let upper = 7_300_000u64;

        let mut data = Vec::new();
        data.push(0x00);              // fixed field ①
        data.push(encode_bcd_u8(1));  // division
        data.push(encode_bcd_u8(1));  // max_div (WLAN)
        data.push(0x01);              // fixed mode
        data.extend_from_slice(&encode_freq_bcd(lower));
        data.extend_from_slice(&encode_freq_bcd(upper));
        data.push(0x00);              // not out of range
        let bins: Vec<u8> = vec![80; SCOPE_BINS];
        data.extend_from_slice(&bins);

        let result = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        assert_eq!(result.mode, Some(ScopeMode::Fixed));
        assert_eq!(
            result.freq_info,
            Some(ScopeFreqInfo::Fixed { lower_edge_hz: lower, upper_edge_hz: upper })
        );
        assert_eq!(result.bins.len(), SCOPE_BINS);
    }

    #[test]
    fn test_parse_wave_out_of_range() {
        let data = make_center_header(1, 11, 14_200_000, 100_000, true);
        let result = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        assert_eq!(result.out_of_range, Some(true));
    }

    #[test]
    fn test_parse_wave_insufficient_data() {
        let result = parse_scope_wave(Bytes::copy_from_slice(&[0x01]));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_wave_invalid_mode() {
        let mut data = Vec::new();
        data.push(0x00);              // fixed field ①
        data.push(encode_bcd_u8(1));
        data.push(encode_bcd_u8(1));
        data.push(0x05); // invalid mode
        // Enough padding for the parser to try
        data.extend_from_slice(&[0; 20]);

        let result = parse_scope_wave(Bytes::copy_from_slice(&data));
        assert!(matches!(result, Err(ScopeParseError::InvalidMode(0x05))));
    }

    // --- parse_scope_setting tests ---

    #[test]
    fn test_parse_setting_scope_on() {
        let result = parse_scope_setting(0x10, &[0x01]).unwrap();
        assert_eq!(result, ScopeSetting::ScopeOnOff(true));
    }

    #[test]
    fn test_parse_setting_scope_off() {
        let result = parse_scope_setting(0x10, &[0x00]).unwrap();
        assert_eq!(result, ScopeSetting::ScopeOnOff(false));
    }

    #[test]
    fn test_parse_setting_wave_output() {
        let result = parse_scope_setting(0x11, &[0x01]).unwrap();
        assert_eq!(result, ScopeSetting::WaveOutputOnOff(true));
    }

    #[test]
    fn test_parse_setting_center_mode() {
        let result = parse_scope_setting(0x14, &[0x00, 0x00]).unwrap();
        assert_eq!(result, ScopeSetting::CenterFixedMode(ScopeMode::Center));
    }

    #[test]
    fn test_parse_setting_fixed_mode() {
        let result = parse_scope_setting(0x14, &[0x00, 0x01]).unwrap();
        assert_eq!(result, ScopeSetting::CenterFixedMode(ScopeMode::Fixed));
    }

    #[test]
    fn test_parse_setting_span() {
        // Span setting (cmd 27 15) response: selector byte + 5 BCD bytes
        // 50000 Hz = 50 kHz
        let mut span_bytes = vec![0x00];
        span_bytes.extend_from_slice(&encode_bcd_n(50_000, 5));
        let result = parse_scope_setting(0x15, &span_bytes).unwrap();
        assert_eq!(result, ScopeSetting::Span(50_000));
    }

    #[test]
    fn test_parse_setting_hold() {
        let result = parse_scope_setting(0x17, &[0x00, 0x01]).unwrap();
        assert_eq!(result, ScopeSetting::Hold(true));
    }

    #[test]
    fn test_parse_setting_sweep_speed() {
        let result = parse_scope_setting(0x1a, &[0x00, 0x02]).unwrap();
        assert_eq!(result, ScopeSetting::SweepSpeed(2));
    }

    #[test]
    fn test_parse_setting_insufficient_data() {
        let result = parse_scope_setting(0x10, &[]);
        assert!(result.is_err());
    }

    // --- ScopeAssembler tests ---

    #[test]
    fn test_assembler_wlan_single_frame() {
        let mut asm = ScopeAssembler::new();
        let freq = 14_200_000u64;
        let span = 100_000u64;
        let bins: Vec<u8> = vec![100; SCOPE_BINS];

        let mut data = make_center_header(1, 1, freq, span, false);
        data.extend_from_slice(&bins);

        let wave = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        let frame = asm.push(wave).expect("should produce frame");

        assert!(matches!(frame.freq, ScopeFreq::Center { center_hz, span_hz } if center_hz == freq && span_hz == span));
        assert_eq!(frame.bins.len(), SCOPE_BINS);
    }

    #[test]
    fn test_assembler_usb_11_divisions() {
        let mut asm = ScopeAssembler::new();
        let freq = 7_074_000u64;
        let span = 50_000u64;

        // Division 1: header
        let header_data = make_center_header(1, 11, freq, span, false);
        let wave = parse_scope_wave(Bytes::copy_from_slice(&header_data)).unwrap();
        assert!(asm.push(wave).is_none());

        // Divisions 2-10: bins
        let bins_per_div = 48;
        for div in 2..=10 {
            let bins: Vec<u8> = vec![div as u8; bins_per_div];
            let data = make_data_division(div, 11, &bins);
            let wave = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
            assert!(asm.push(wave).is_none());
        }

        // Division 11: last bins — should complete
        let last_bins: Vec<u8> = vec![11; bins_per_div];
        let data = make_data_division(11, 11, &last_bins);
        let wave = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        let frame = asm.push(wave).expect("should produce frame on last division");

        assert!(matches!(frame.freq, ScopeFreq::Center { center_hz, span_hz } if center_hz == freq && span_hz == span));
        // 10 data divisions * 48 bins each = 480
        assert_eq!(frame.bins.len(), bins_per_div * 10);
    }

    #[test]
    fn test_assembler_reset_on_new_sweep() {
        let mut asm = ScopeAssembler::new();
        let freq1 = 14_200_000u64;
        let freq2 = 7_074_000u64;
        let span = 100_000u64;

        // Start first sweep: divisions 1-5
        let header = make_center_header(1, 11, freq1, span, false);
        asm.push(parse_scope_wave(Bytes::copy_from_slice(&header)).unwrap());
        for div in 2..=5 {
            let data = make_data_division(div, 11, &vec![div as u8; 48]);
            asm.push(parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap());
        }

        // New sweep starts — should discard partial
        let header2 = make_center_header(1, 11, freq2, span, false);
        asm.push(parse_scope_wave(Bytes::copy_from_slice(&header2)).unwrap());

        // Complete the second sweep
        for div in 2..=10 {
            let data = make_data_division(div, 11, &vec![div as u8; 48]);
            assert!(asm.push(parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap()).is_none());
        }
        let data = make_data_division(11, 11, &vec![11; 48]);
        let frame = asm.push(parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap()).expect("should complete second sweep");

        // Should have freq2, not freq1
        assert!(matches!(frame.freq, ScopeFreq::Center { center_hz, .. } if center_hz == freq2));
    }

    #[test]
    fn test_assembler_fixed_mode_frame() {
        let mut asm = ScopeAssembler::new();
        let lower = 7_000_000u64;
        let upper = 7_300_000u64;

        let mut data = Vec::new();
        data.push(0x00);              // fixed field ①
        data.push(encode_bcd_u8(1));
        data.push(encode_bcd_u8(1)); // WLAN
        data.push(0x01);             // fixed mode
        data.extend_from_slice(&encode_freq_bcd(lower));
        data.extend_from_slice(&encode_freq_bcd(upper));
        data.push(0x00);
        let bins: Vec<u8> = vec![80; SCOPE_BINS];
        data.extend_from_slice(&bins);

        let wave = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        let frame = asm.push(wave).expect("should produce frame");

        assert!(matches!(frame.freq, ScopeFreq::Fixed { lower_hz, upper_hz } if lower_hz == lower && upper_hz == upper));
        assert_eq!(frame.bins.len(), SCOPE_BINS);
    }

    #[test]
    fn test_assembler_data_without_header_ignored() {
        let mut asm = ScopeAssembler::new();
        let data = make_data_division(5, 11, &vec![42; 48]);
        let wave = parse_scope_wave(Bytes::copy_from_slice(&data)).unwrap();
        assert!(asm.push(wave).is_none());
    }
}
