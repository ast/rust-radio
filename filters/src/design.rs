use num_complex::Complex32;
use std::f32::consts::PI;

/// Fred Harris' rule of thumb for FIR tap count. Returns the estimated
/// number of taps needed for an FIR with stopband attenuation `att_db`,
/// transition bandwidth `trans_hz`, at sample rate `fs_hz`.
///
/// See: f. harris, "Multirate Signal Processing for Communication Systems".
pub fn fred_harris_taps(att_db: f32, trans_hz: f32, fs_hz: f32) -> usize {
    let n = (att_db / 22.0) * (fs_hz / trans_hz);
    n.round().max(3.0) as usize
}

/// Blackman window of length `n`. Good ~74 dB sidelobe attenuation.
pub fn blackman_window(n: usize) -> Vec<f32> {
    let m = (n - 1) as f32;
    (0..n)
        .map(|i| {
            let x = 2.0 * PI * i as f32 / m;
            0.42 - 0.5 * x.cos() + 0.08 * (2.0 * x).cos()
        })
        .collect()
}

/// Windowed-sinc lowpass FIR. `cutoff_hz` is the -6 dB point (ideal sinc
/// cutoff); Blackman window gives ~-74 dB stopband and roughly
/// `5.5 / n_taps` normalized transition.
pub fn windowed_sinc_lp(n_taps: usize, fs_hz: f32, cutoff_hz: f32) -> Vec<f32> {
    let m = (n_taps - 1) as f32 / 2.0;
    let fc_norm = cutoff_hz / fs_hz; // cycles/sample
    let win = blackman_window(n_taps);
    let mut h: Vec<f32> = (0..n_taps)
        .map(|i| {
            let n = i as f32 - m;
            let v = if n == 0.0 {
                2.0 * fc_norm
            } else {
                (2.0 * PI * fc_norm * n).sin() / (PI * n)
            };
            v * win[i]
        })
        .collect();

    // Normalize DC gain to 1.
    let sum: f32 = h.iter().sum();
    for x in &mut h {
        *x /= sum;
    }
    h
}

/// Multiply a real LP kernel by exp(j·2π·shift_hz·n/fs) to obtain a
/// complex-coefficient FIR centered at `shift_hz`. Phase is centered on
/// the middle tap so the resulting filter has zero group-delay phase
/// shift at the center frequency (equivalent to a real bandpass of the
/// same width at shift_hz for the real part).
pub fn complex_shift_lp(lp: &[f32], fs_hz: f32, shift_hz: f32) -> Vec<Complex32> {
    let omega = 2.0 * PI * shift_hz / fs_hz;
    let m = (lp.len() - 1) as f32 / 2.0;
    lp.iter()
        .enumerate()
        .map(|(i, &c)| Complex32::from_polar(c, omega * (i as f32 - m)))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SsbSide {
    Upper,
    Lower,
}

/// Complex-coefficient SSB selection filter. Passband is `[0, width_hz]`
/// for USB or `[-width_hz, 0]` for LSB (centered on DC, which is the
/// demod zero after the rotator). Designed as a real LP of width
/// `width_hz/2` modulated to ±width_hz/2.
pub fn ssb_bandpass(
    n_taps: usize,
    fs_hz: f32,
    width_hz: f32,
    side: SsbSide,
) -> Vec<Complex32> {
    let lp = windowed_sinc_lp(n_taps, fs_hz, width_hz / 2.0);
    let shift = match side {
        SsbSide::Upper => width_hz / 2.0,
        SsbSide::Lower => -width_hz / 2.0,
    };
    complex_shift_lp(&lp, fs_hz, shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(x: f32) -> f32 {
        20.0 * x.abs().max(1e-12).log10()
    }

    fn freq_response_real(h: &[f32], fs: f32, f: f32) -> f32 {
        let omega = 2.0 * PI * f / fs;
        let m = (h.len() - 1) as f32 / 2.0;
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (i, &c) in h.iter().enumerate() {
            let phi = omega * (i as f32 - m);
            re += c * phi.cos();
            im += c * phi.sin();
        }
        (re * re + im * im).sqrt()
    }

    fn freq_response_cplx(h: &[Complex32], fs: f32, f: f32) -> f32 {
        let omega = 2.0 * PI * f / fs;
        let m = (h.len() - 1) as f32 / 2.0;
        let mut acc = Complex32::new(0.0, 0.0);
        for (i, &c) in h.iter().enumerate() {
            acc += c * Complex32::from_polar(1.0, -omega * (i as f32 - m));
        }
        acc.norm()
    }

    #[test]
    fn fred_harris_ballpark() {
        // Matches the worked example in docs/Close to optimal FIR decimator design.pdf
        assert_eq!(fred_harris_taps(60.0, 91_000.0, 768_000.0), 23);
        assert_eq!(fred_harris_taps(60.0, 500.0, 48_000.0), 262);
    }

    #[test]
    fn lp_unity_dc_and_stopband() {
        let h = windowed_sinc_lp(101, 48_000.0, 3_000.0);
        assert!((freq_response_real(&h, 48_000.0, 0.0) - 1.0).abs() < 1e-4);
        // Well into stopband.
        assert!(db(freq_response_real(&h, 48_000.0, 10_000.0)) < -60.0);
    }

    #[test]
    fn ssb_bandpass_asymmetric() {
        // 48k sample rate, 2.7 kHz SSB. Generous 500 Hz transition → taps ≈ 262.
        let n = fred_harris_taps(60.0, 500.0, 48_000.0);
        let usb = ssb_bandpass(n | 1, 48_000.0, 2_700.0, SsbSide::Upper);
        // Positive side passes, negative side is deep stopband.
        assert!(db(freq_response_cplx(&usb, 48_000.0, 1_000.0)) > -1.0);
        assert!(db(freq_response_cplx(&usb, 48_000.0, -1_000.0)) < -60.0);
        assert!(db(freq_response_cplx(&usb, 48_000.0, 5_000.0)) < -60.0);

        let lsb = ssb_bandpass(n | 1, 48_000.0, 2_700.0, SsbSide::Lower);
        assert!(db(freq_response_cplx(&lsb, 48_000.0, -1_000.0)) > -1.0);
        assert!(db(freq_response_cplx(&lsb, 48_000.0, 1_000.0)) < -60.0);
    }
}
