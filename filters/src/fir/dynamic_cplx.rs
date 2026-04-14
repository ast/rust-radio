use num_complex::Complex32;

use crate::DelayLine;
use crate::ringbuffer::RingBuffer;

/// Runtime-sized FIR filter with complex-valued coefficients operating
/// on complex samples. Used for asymmetric bandpass filters (SSB
/// selection) where the passband is one-sided around DC.
pub struct DynFirComplex {
    h: Vec<Complex32>,
    z: RingBuffer<Complex32>,
}

impl DynFirComplex {
    pub fn new(mut h: Vec<Complex32>) -> Self {
        // RingBuffer::as_slice returns samples oldest-first, so h[i] · z[i]
        // applies the kernel in reverse time order. Reverse h so the runtime
        // convolution matches standard y[n] = Σ h[k]·x[n-k].
        h.reverse();
        let taps = h.len();
        DynFirComplex {
            h,
            z: RingBuffer::new(taps),
        }
    }

    #[inline]
    pub fn filter(&mut self, x: Complex32) -> Complex32 {
        self.z.push(x);
        let z = self.z.as_slice();
        let mut acc = Complex32::new(0.0, 0.0);
        for i in 0..self.h.len() {
            acc += self.h[i] * z[i];
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::{SsbSide, ssb_bandpass};
    use signals::ComplexOscillator;

    fn db(x: f32) -> f32 {
        20.0 * x.max(1e-12).log10()
    }

    fn rms(xs: &[Complex32]) -> f32 {
        let s: f32 = xs.iter().map(|z| z.norm_sqr()).sum();
        (s / xs.len() as f32).sqrt()
    }

    #[test]
    fn ssb_usb_passes_positive_rejects_negative() {
        let fs = 48_000.0;
        let n = 263;
        let h = ssb_bandpass(n, fs, 2_700.0, SsbSide::Upper);
        let mut flt = DynFirComplex::new(h);

        // Warm-up length to skip filter transient.
        let len = 8192;
        let warm = 1024;

        let mut pos: Vec<Complex32> = ComplexOscillator::new(1_000.0, fs).take(len).collect();
        for s in &mut pos {
            *s = flt.filter(*s);
        }
        let gain_pos = rms(&pos[warm..]);

        let mut flt2 = DynFirComplex::new(ssb_bandpass(n, fs, 2_700.0, SsbSide::Upper));
        let mut neg: Vec<Complex32> = ComplexOscillator::new(-1_000.0, fs).take(len).collect();
        for s in &mut neg {
            *s = flt2.filter(*s);
        }
        let gain_neg = rms(&neg[warm..]);

        assert!(db(gain_pos) > -1.0, "pass gain = {} dB", db(gain_pos));
        assert!(db(gain_neg) < -55.0, "reject gain = {} dB", db(gain_neg));
    }
}
