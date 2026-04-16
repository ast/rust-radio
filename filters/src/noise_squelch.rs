use crate::{Biquad, Q_BUTTERWORTH};

/// Noise-power-based audio squelch with hysteresis.
///
/// The raw demod output is fed to [`NoiseSquelch::update`] once per audio
/// sample. A biquad HPF extracts energy above the voice band (typically
/// >4 kHz); squared and smoothed with a 1-pole IIR this gives a running
/// noise-power envelope that drops sharply under FM quieting.
///
/// [`NoiseSquelch::gate`] is applied to the *post-filter* audio — it only
/// selects between the audio sample and silence, so muting happens after
/// the deemphasis and audio BPF.
pub struct NoiseSquelch {
    noise_hpf: Biquad,
    envelope: f32,
    alpha: f32,
    open_threshold: f32,
    close_threshold: f32,
    gate_open: bool,
}

impl NoiseSquelch {
    pub fn new(
        sample_rate_hz: f32,
        noise_hpf_hz: f32,
        envelope_tau_s: f32,
        open_threshold: f32,
        close_threshold: f32,
    ) -> Self {
        assert!(close_threshold >= open_threshold, "close must be ≥ open for hysteresis");
        let dt = 1.0 / sample_rate_hz;
        let alpha = dt / (envelope_tau_s + dt);
        Self {
            noise_hpf: Biquad::highpass(sample_rate_hz, noise_hpf_hz, Q_BUTTERWORTH),
            envelope: 0.0,
            alpha,
            open_threshold,
            close_threshold,
            gate_open: false,
        }
    }

    pub fn update(&mut self, raw: f32) {
        let n = self.noise_hpf.process(raw);
        let p = n * n;
        self.envelope += self.alpha * (p - self.envelope);
        if self.gate_open {
            if self.envelope > self.close_threshold {
                self.gate_open = false;
            }
        } else if self.envelope < self.open_threshold {
            self.gate_open = true;
        }
    }

    #[inline]
    pub fn gate(&self, audio: f32) -> f32 {
        if self.gate_open { audio } else { 0.0 }
    }

    pub fn is_open(&self) -> bool {
        self.gate_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn drive(sq: &mut NoiseSquelch, samples: &[f32]) {
        for &s in samples {
            sq.update(s);
        }
    }

    #[test]
    fn silent_input_opens_gate() {
        let mut sq = NoiseSquelch::new(48_000.0, 4_000.0, 0.03, 1e-3, 3e-3);
        drive(&mut sq, &vec![0.0; 48_000]);
        assert!(sq.is_open(), "gate should open on silence");
        assert_eq!(sq.gate(0.5), 0.5);
    }

    #[test]
    fn high_freq_noise_closes_gate() {
        let mut sq = NoiseSquelch::new(48_000.0, 4_000.0, 0.03, 1e-3, 3e-3);
        // Pure 8 kHz tone sits well inside the noise HPF passband → closes gate.
        let tone: Vec<f32> = (0..48_000)
            .map(|k| (TAU * 8_000.0 * k as f32 / 48_000.0).sin() * 0.3)
            .collect();
        drive(&mut sq, &tone);
        assert!(!sq.is_open(), "gate should close on strong high-freq energy");
        assert_eq!(sq.gate(0.5), 0.0);
    }

    #[test]
    fn voiceband_signal_leaves_gate_open() {
        let mut sq = NoiseSquelch::new(48_000.0, 4_000.0, 0.03, 1e-3, 3e-3);
        // 1 kHz voice-band tone — below the noise HPF, so the noise envelope
        // stays low and the gate should remain open.
        let tone: Vec<f32> = (0..48_000)
            .map(|k| (TAU * 1_000.0 * k as f32 / 48_000.0).sin() * 0.8)
            .collect();
        drive(&mut sq, &tone);
        assert!(sq.is_open(), "gate should stay open on voice-band audio");
    }

    #[test]
    fn hysteresis_prevents_chatter() {
        let mut sq = NoiseSquelch::new(48_000.0, 4_000.0, 0.03, 1e-3, 3e-3);
        // Settle open on silence.
        drive(&mut sq, &vec![0.0; 48_000]);
        assert!(sq.is_open());

        // Noise at a level that sits between the two thresholds: once the gate
        // opens on silence it should stay open until noise exceeds the close
        // threshold. Target envelope ≈ 2e-3 (mid-band) for a clean test.
        // HPF passes ~half the tone's power → |a|²/2 ≈ 2e-3 → a ≈ 0.063.
        let mid: Vec<f32> = (0..48_000)
            .map(|k| (TAU * 8_000.0 * k as f32 / 48_000.0).sin() * 0.063)
            .collect();
        drive(&mut sq, &mid);
        assert!(sq.is_open(), "gate should stay open in hysteresis band");
    }
}
