use clap::{Parser, Subcommand};

use airspyhf::Device;
use anyhow::{Context, Result};

pub mod spectrum_server;

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
        #[arg(short, long, default_value_t = 89_300_000.0)]
        frequency: f64,
        /// Sample rate
        #[arg(short, long, default_value_t = 768_000)]
        samplerate: u32,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let fft_len = 32768;
    let spectrum_server = spectrum_server::SpectrumServer::new(fft_len);

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
                    if dropped > 0 {
                        eprintln!("Dropped {} samples", dropped);
                    }

                    spectrum_server.process(samples);
                    // TODO: error handling

                    0 // = continue
                })
                .context("Failed to start device")?;

            // Sleep for 2 seconds
            std::thread::sleep(std::time::Duration::from_secs(100));

            // Stop streaming
            device.stop().context("Failed to stop device")?;
        }
    }

    Ok(())
}
