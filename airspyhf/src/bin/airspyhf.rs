use clap::{Parser, Subcommand};

use airspyhf::Device;

/// Example CLI app using clap derive and subcommands
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
    Version,

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
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Version => {
            let version = airspyhf::lib_version();

            println!(
                "AirspyHF version: {}.{}.{}",
                version.0, version.1, version.2
            );
        }
        Commands::List => {
            let serials = airspyhf::list_devices().unwrap_or_else(|_| {
                println!("No devices found.");
                std::process::exit(1);
            });

            for serial in serials {
                println!("Serial: {:016x}", serial);

                // Open this device to list more info
                let device = Device::open_sn(serial).expect("Failed to open device");

                // Get version string
                let version = device.version_string().expect("Failed to get version");
                println!("  Version: {}", version);

                // Get sample rates
                let samplerates = device.get_samplerates().expect("Failed to get samplerates");
                println!("  Samplerates: {:?}", samplerates);

                // Output size
                let output_size = device.output_size().expect("Failed to get output size");
                println!("  Output size: {}", output_size);
            }
        }
        Commands::Start {
            frequency,
            samplerate,
            duration,
        } => {
            let mut device = Device::open().expect("Failed to open device");

            // Set frequency
            device
                .set_frequency(*frequency)
                .expect("Failed to set frequency");

            // Set samplerate
            device
                .set_samplerate(*samplerate)
                .expect("Failed to set samplerate");

            // Start streaming
            device
                .start(|samples, dropped| {
                    println!("Received {} samples, dropped {}", samples.len(), dropped);

                    // 0 = continue
                    0
                })
                .expect("Failed to start device");

            // Sleep for 2 seconds
            std::thread::sleep(std::time::Duration::from_secs(*duration));

            // Stop streaming
            device.stop().expect("Failed to stop device");
        }
    }
}
