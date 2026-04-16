use std::sync::Mutex;

use doublemap::{Consumer, Producer, ring_buffer_pair};
use num_complex::Complex32;

/// Fan-out of IQ sample blocks to N consumers.
///
/// A single upstream producer (airspyhf callback or fake source) calls
/// [`IqBroker::broadcast`] with a block of samples. Each subscriber owns a
/// [`doublemap`] SPSC ring and a paired [`Consumer`]; on broadcast we copy
/// the block into every live ring. Consumers that get dropped cause the
/// corresponding [`Producer`] to observe `Disconnected` on the next produce,
/// and we prune them.
///
/// Designed for N consumers from day one: MVP uses 2 (spectrum + shared FM),
/// per-client demod adds more without touching this type.
pub struct IqBroker {
    producers: Mutex<Vec<Producer<Complex32>>>,
    producer_required: usize,
}

impl IqBroker {
    /// `producer_required` is the block size the upstream source writes per
    /// broadcast call (airspyhf delivers 4096).
    pub fn new(producer_required: usize) -> Self {
        Self {
            producers: Mutex::new(Vec::new()),
            producer_required,
        }
    }

    /// Subscribe a new consumer. `consumer_required` is the minimum number
    /// of samples the consumer waits for before its closure is called
    /// (e.g. `fft_len` for the spectrum worker).
    pub fn subscribe(&self, consumer_required: usize) -> Consumer<Complex32> {
        let (producer, consumer) = ring_buffer_pair::<Complex32>(
            self.producer_required,
            consumer_required,
        );
        self.producers.lock().unwrap().push(producer);
        consumer
    }

    /// Broadcast a block of IQ samples to all live consumers.
    ///
    /// Non-blocking: if a consumer's ring doesn't have room for the block we
    /// drop it for that consumer rather than stalling the caller. The caller
    /// is the upstream USB callback thread (airspyhf), and blocking there
    /// stalls the whole capture — dropping samples for a slow listener is
    /// the correct real-time tradeoff. Subscriptions whose consumer has been
    /// dropped are pruned on the next broadcast.
    pub fn broadcast(&self, samples: &[Complex32]) {
        let mut producers = self.producers.lock().unwrap();
        let mut dropped = 0usize;
        producers.retain(|p| {
            match p.try_produce(|slice| {
                let n = samples.len().min(slice.len());
                slice[..n].copy_from_slice(&samples[..n]);
                n
            }) {
                Ok(true) => true,
                Ok(false) => {
                    dropped += 1;
                    true
                }
                Err(_) => false,
            }
        });
        if dropped > 0 {
            tracing::warn!(dropped, "iq_broker: slow consumer, dropped block(s)");
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.producers.lock().unwrap().len()
    }
}
