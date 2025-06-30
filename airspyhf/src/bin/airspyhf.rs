use clap::{Parser, Subcommand};

use airspyhf::Device;

use anyhow::{Context, Result};
use num_complex::Complex32;
use std::path::PathBuf;

use std::io::{BufWriter, Write};

#[derive(Parser)]
#[command(name = "airspyhf")]
#[command(author = "Albin Stigo <albin@sm6wjm.se>")]
#[command(version = "1.0")]
#[command(about = "Using AirSpyHF+ with Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the version of the AirspyHF library
    LibVersion,

    /// List all available devices
    List,

    Start {
        /// Frequency in Hz
        #[arg(short, long, default_value_t = 7_200_000.0)]
        frequency: f64,
        /// Samplerate
        #[arg(short, long, default_value_t = 912_000)]
        samplerate: u32,
        /// Duration
        #[arg(short, long, default_value_t = 2)]
        duration: u64,
        #[arg(short, long)]
        record: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::LibVersion => {
            let version = airspyhf::lib_version();

            println!(
                "AirspyHF version: {}.{}.{}",
                version.0, version.1, version.2
            );

            Ok(())
        }
        Commands::List => {
            let serials = airspyhf::list_devices().context("Failed to list devices")?;

            for serial in serials {
                println!("Serial: {:016x}", serial);

                // Open this device to list more info
                let device = Device::open_sn(serial).expect("Failed to open device");

                // Get version string
                let version = device.version_string().expect("Failed to get version");
                println!("  Version: {}", version);

                // Get sample rates
                let samplerates = device.get_samplerates().expect("Failed to get samplerates");
                println!("  Sample rates: {:?}", samplerates);

                // Output size
                let output_size = device.output_size().expect("Failed to get output size");
                println!("  Output size: {}", output_size);
            }

            Ok(())
        }
        Commands::Start {
            frequency,
            samplerate,
            duration,
            record,
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

            // If recording is specified, set up file output
            if let Some(record_path) = record {
                let file =
                    std::fs::File::create(record_path).context("Failed to create output file")?;
                let mut writer = BufWriter::new(file);

                // Start streaming
                device
                    .start(move |samples, dropped| {
                        // To slice
                        // TODO: refactor into function
                        let byte_slice = unsafe {
                            std::slice::from_raw_parts(
                                samples.as_ptr() as *const u8,
                                samples.len() * std::mem::size_of::<Complex32>(),
                            )
                        };
                        assert_eq!(byte_slice.len() / 8, samples.len());

                        writer
                            .write_all(byte_slice)
                            .expect("Failed to write samples");

                        println!("Wrote {} samples, dropped {}", samples.len(), dropped);

                        // 0 = continue
                        0
                    })
                    .context("Failed to start device")?;
            } else {
                // Start streaming without recording
                device
                    .start(move |samples, dropped| {
                        println!("Received {} samples, dropped {}", samples.len(), dropped);

                        // 0 = continue
                        0
                    })
                    .context("Failed to start device")?;
            }

            // Sleep for 2 seconds
            std::thread::sleep(std::time::Duration::from_secs(*duration));

            // Stop streaming
            device.stop().context("Failed to stop device")?;

            Ok(())
        }
    }
}
