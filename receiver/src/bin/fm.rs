use clap::{Parser, Subcommand};

use airspyhf::Device;

use filters::{Decimator, FirDecimator};

use filters::kernels::FM_32;

use num_complex::Complex32;

use std::net::SocketAddr;
use std::net::UdpSocket;

use procfs::process::Process;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

#[derive(Parser)]
#[command(name = "WFM receiver")]
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

    //let mut fir_dec: FirDecimator<Complex32, 64, 2> = FirDecimator::new(FM_64);
    let mut fir_dec: FirDecimator<Complex32, 32, 2> = FirDecimator::new(FM_32);

    let mut output = Vec::<Complex32>::with_capacity(1024);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let target_addr: SocketAddr = "127.0.0.1:7373".parse()?;

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

                    // It doesn't work without these.
                    // net.core.rmem_default=26214400
                    // net.core.rmem_max=104857600
                    // net.core.wmem_default=65536
                    // net.core.wmem_max=104857600

                    // Clear output buffer
                    output.clear();
                    // Filter into output buffer
                    output.extend(samples.iter().filter_map(|&x| fir_dec.decimate(x)));

                    // Send samples with
                    let bytes: &[u8] = unsafe {
                        std::slice::from_raw_parts(
                            output.as_ptr() as *const u8,
                            output.len() * std::mem::size_of::<Complex32>(),
                        )
                    };

                    // Send as chunks
                    for chunk in bytes.chunks(1024) {
                        if let Err(e) = socket.send_to(chunk, target_addr) {
                            eprintln!("Failed to send UDP chunk: {}", e);
                        } else {
                            // Optional: log how many bytes you sent
                            // println!("Sent {} bytes", chunk.len());
                        }
                    }

                    0 // = continue
                })
                .context("Failed to start device")?;

            // Wait for user input to stop
            println!("Press Enter to stop...");

            let start = Instant::now();

            let pid = std::process::id();
            let process = Process::new(pid as i32)?;

            loop {
                // Fetch updated process stats
                let stat = process.stat().expect("Failed to read process stat");
                let _mem = process.status().expect("Failed to read memory status");

                println!("Elapsed time: {:.2?}", start.elapsed());
                println!(
                    "CPU time (user + sys): {:.2} sec",
                    stat.utime as f64 / 100.0 + stat.stime as f64 / 100.0
                );
                println!("Threads: {}", stat.num_threads);

                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
