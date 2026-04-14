use std::time::Duration;

use sdrlink::config::SdrConfig;
use sdrlink::sdr::SdrHandle;
use tokio::time::timeout;

/// A fake-sourced handle should produce at least one spectrum frame containing
/// a peak at the injected tone's bin.
#[tokio::test]
async fn fake_source_produces_spectrum_frames() {
    // Small FFT so this runs fast. Tone at +100 kHz lands clearly off DC.
    let samplerate: u32 = 768_000;
    let fft_len: u32 = 1024;
    let tone_hz: f32 = 100_000.0;

    let config = SdrConfig {
        center_hz: 100_000_000.0,
        samplerate,
        fft_len,
        fft_rate_hz: 50,
    };

    let handle = SdrHandle::start_fake(config, tone_hz);
    let mut rx = handle.subscribe_spectrum();

    let frame = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for spectrum frame")
        .expect("channel closed");

    assert_eq!(frame.len(), fft_len as usize);

    // Expected bin: DC is at fft_len/2 (analyzer shifts DC to center), tone at
    // +100 kHz offsets by tone/bin_hz bins.
    let bin_hz = samplerate as f32 / fft_len as f32;
    let expected = (fft_len as f32 / 2.0 + tone_hz / bin_hz).round() as usize;

    let peak_bin = frame
        .iter()
        .enumerate()
        .max_by_key(|(_, v)| **v)
        .map(|(i, _)| i)
        .unwrap();

    // Allow a couple of bins of leakage.
    let diff = (peak_bin as isize - expected as isize).abs();
    assert!(
        diff <= 2,
        "peak bin {peak_bin} not within 2 bins of expected {expected}"
    );
}
