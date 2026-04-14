/// Per-client view window onto the full spectrum: a contiguous bin range
/// decimated to `pixels` output columns.
///
/// Constructed from frequency bounds via [`Viewport::from_hz`]; pure
/// [`Viewport::decimate`] fills caller-provided min/max buffers so the WS
/// per-connection task can reuse buffers across frames.
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// Inclusive start bin in the full PDS.
    pub start_bin: usize,
    /// Exclusive end bin in the full PDS.
    pub stop_bin: usize,
    pub pixels: u16,
}

impl Viewport {
    /// Build a viewport from an absolute frequency window. The analyzer
    /// centers DC at `fft_len/2`, so `bin = fft_len/2 + f_rel * fft_len /
    /// samplerate`. Clamped to `[0, fft_len]`; returns `None` if the window
    /// is empty or inverted.
    pub fn from_hz(
        start_hz: f64,
        stop_hz: f64,
        pixels: u16,
        center_hz: f64,
        samplerate: u32,
        fft_len: usize,
    ) -> Option<Self> {
        if pixels == 0 || stop_hz <= start_hz {
            return None;
        }
        let hz_per_bin = samplerate as f64 / fft_len as f64;
        let dc = fft_len as f64 / 2.0;
        let start = (dc + (start_hz - center_hz) / hz_per_bin)
            .floor()
            .clamp(0.0, fft_len as f64) as usize;
        let stop = (dc + (stop_hz - center_hz) / hz_per_bin)
            .ceil()
            .clamp(0.0, fft_len as f64) as usize;

        if stop <= start {
            return None;
        }

        // Each output column must cover at least one input bin; otherwise
        // shrink pixels so we never widen the data.
        let span = stop - start;
        let pixels = (pixels as usize).min(span) as u16;
        if pixels == 0 {
            return None;
        }

        Some(Self {
            start_bin: start,
            stop_bin: stop,
            pixels,
        })
    }

    /// Min/max decimate `pds[start_bin..stop_bin]` into `out_min`/`out_max`.
    /// Both output buffers must be `pixels` long.
    pub fn decimate(&self, pds: &[u8], out_min: &mut [u8], out_max: &mut [u8]) {
        debug_assert_eq!(out_min.len(), self.pixels as usize);
        debug_assert_eq!(out_max.len(), self.pixels as usize);
        debug_assert!(self.stop_bin <= pds.len());

        let span = (self.stop_bin - self.start_bin) as f64;
        let pixels = self.pixels as usize;

        for p in 0..pixels {
            let lo = self.start_bin + ((p as f64) * span / pixels as f64).floor() as usize;
            let hi = (self.start_bin + (((p + 1) as f64) * span / pixels as f64).ceil() as usize)
                .min(self.stop_bin);
            let hi = hi.max(lo + 1);

            let mut mn = u8::MAX;
            let mut mx = u8::MIN;
            for &v in &pds[lo..hi] {
                if v < mn {
                    mn = v;
                }
                if v > mx {
                    mx = v;
                }
            }
            out_min[p] = mn;
            out_max[p] = mx;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_band_viewport() {
        let vp = Viewport::from_hz(
            100_000_000.0 - 384_000.0,
            100_000_000.0 + 384_000.0,
            1024,
            100_000_000.0,
            768_000,
            32768,
        )
        .unwrap();
        assert_eq!(vp.start_bin, 0);
        assert_eq!(vp.stop_bin, 32768);
        assert_eq!(vp.pixels, 1024);
    }

    #[test]
    fn zoom_preserves_peak_via_max() {
        // Construct a PDS with a single peak mid-band.
        let fft_len = 32;
        let mut pds = vec![10u8; fft_len];
        pds[20] = 200;

        let vp = Viewport {
            start_bin: 0,
            stop_bin: fft_len,
            pixels: 8,
        };
        let mut mn = vec![0u8; 8];
        let mut mx = vec![0u8; 8];
        vp.decimate(&pds, &mut mn, &mut mx);

        // 32 bins / 8 pixels = 4 bins per pixel; bin 20 lands in pixel 5.
        assert_eq!(mx[5], 200);
        assert!(mn.iter().all(|&v| v == 10));
    }

    #[test]
    fn pixels_capped_to_span() {
        // Asking for more pixels than bins shrinks pixels to span.
        let vp = Viewport::from_hz(
            100_000_000.0 - 1000.0,
            100_000_000.0 + 1000.0,
            1920,
            100_000_000.0,
            768_000,
            32768,
        )
        .unwrap();
        let span = vp.stop_bin - vp.start_bin;
        assert!(vp.pixels as usize <= span);
    }

    #[test]
    fn clamps_outside_passband() {
        let vp = Viewport::from_hz(
            0.0,
            200_000_000.0,
            256,
            100_000_000.0,
            768_000,
            32768,
        )
        .unwrap();
        assert_eq!(vp.start_bin, 0);
        assert_eq!(vp.stop_bin, 32768);
    }

    #[test]
    fn inverted_window_rejected() {
        assert!(
            Viewport::from_hz(1.0, 0.0, 10, 100_000_000.0, 768_000, 32768).is_none()
        );
    }
}
