use clap::{Parser, Subcommand};

use airspyhf::Device;
use anyhow::{Context, Result};

use doublemap::ring_buffer_pair;

//use doublemap::Consumer;
//use doublemap::Producer;

use num_complex::Complex32;
// use std::path::PathBuf;
//use std::io::{BufWriter, Write};

use std::thread;

#[derive(Parser)]
#[command(name = "receiver")]
#[command(about = "Rust SDR receiver", long_about = None)]
#[command(author = "Albin Stigo <albin@sm6wjm.se>")]
#[command(version = "0.1")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        /// Frequency in Hz
        #[arg(short, long, default_value_t = 7_200_000.0)]
        frequency: f64,
        /// Samplerate
        #[arg(short, long, default_value_t = 768_000)]
        samplerate: u32,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Ring buffer
    let (producer, consumer) = ring_buffer_pair::<Complex32>(2 * 32768);

    let fft_len = 32768;

    // Consumer thread
    let consumer_thread = thread::spawn(move || {
        println!("Starting consumer thread with FFT length: {}", fft_len);

        loop {
            consumer.consume(fft_len, |slice| {
                println!("Got {} samples", slice.len());
                fft_len
            });
        }
    });

    match &cli.command {
        Commands::Start {
            frequency,
            samplerate,
        } => {
            let mut device = Device::open().context("Failed to open device")?;

            // Set frequency
            device
                .set_frequency(*frequency)
                .context("Failed to set frequency")?;

            // Set samplerate
            device
                .set_samplerate(*samplerate)
                .context("Failed to set samplerate")?;

            // Start streaming without recording
            device
                .start(move |samples, dropped| {
                    // println!("Received {} samples, dropped {}", samples.len(), dropped);

                    producer.produce(samples.len(), |slice| {
                        // Copy samples to slice
                        slice[..samples.len()].copy_from_slice(samples);
                        samples.len()
                    });
                    0 // = continue
                })
                .context("Failed to start device")?;

            // Sleep for 2 seconds
            std::thread::sleep(std::time::Duration::from_secs(5));

            // Stop streaming
            device.stop().context("Failed to stop device")?;
        }
    }

    consumer_thread.join().expect("Consumer thread panicked");

    Ok(())
}
