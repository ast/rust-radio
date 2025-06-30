use anyhow::Result;
use clap::Parser;

use bytemuck::cast_slice;
use num_complex::Complex32;

use signals::{FileSource, NoiseSource};
use spectrum::Analyzer;

use std::io::BufWriter;

use std::{
    fs,
    io::Write,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
};

use zerocopy::{
    Immutable, IntoBytes, byteorder::big_endian::U16 as U16be, byteorder::big_endian::U32 as U32be,
    byteorder::big_endian::U64 as U64be,
};

/// Message kind
#[repr(u8)]
#[derive(Debug, Copy, Clone, IntoBytes, Immutable)]
pub enum MsgType {
    Spectrum = 0x01,
}

// Header for the spectrum server messages.
#[repr(C)]
#[derive(Debug, Copy, Clone, IntoBytes, Immutable)]
pub struct MsgHeader {
    pub prefix: U32be,
    pub version: u8,
    pub msg_type: MsgType,
    pub sequence_number: U16be,
    pub ntp_time: U64be,
    pub length: U32be,
}

impl MsgHeader {
    pub const PREFIX: u32 = 0xC0DE_0073;
    pub const VERSION: u8 = 1;

    pub fn new(msg_type: MsgType, length: usize) -> Self {
        MsgHeader {
            prefix: U32be::new(0xC0DE_0073),
            version: Self::VERSION,
            msg_type,
            sequence_number: U16be::new(0),
            ntp_time: U64be::new(0),
            length: U32be::new(length.try_into().expect("Length must fit in u32")),
        }
    }
}

const SOCKET_PATH: &str = "/tmp/echo.sock";

#[derive(Parser, Debug)]
#[command(name = "server", version, about = "Unix socket spectrum server test.")]
struct Cli {
    #[arg(short, long, default_value = SOCKET_PATH)]
    socket_path: PathBuf,
    #[arg(short, long)]
    file: Option<PathBuf>,
}

fn handle_client(stream: UnixStream) -> Result<()> {
    // Size of FFT
    let size = 32768;
    // Create our analyzer
    let mut analzyer = Analyzer::new(size, 768000.0);
    // Use a buffered writer
    let mut buf_writer = BufWriter::new(stream);

    // Noise generator
    let mut noise = NoiseSource::new(-60.0);
    // let mut signal0 = ComplexOscillator::new(50_000.0, 768_000.0);
    // let mut signal1 = ComplexOscillator::new(-50_000.0, 768_000.0);
    // Recorded from airspy
    let mut signal2 = FileSource::new(PathBuf::from("/home/albin/src/rust-radio/89.3mhz.bin"))?;

    let mut input_buffer = vec![Complex32::default(); size];
    let mut output_buffer = vec![0.0f32; size];

    loop {
        // Collect noise samples into input_buffer
        input_buffer.clear();
        input_buffer.extend(noise.by_ref().take(size));

        // Add some signal
        for i in 0..size {
            //input_buffer[i] += signal0.next().unwrap() + 0.5 * signal1.next().unwrap();
            input_buffer[i] += signal2.next().unwrap_or(Complex32::default());
        }

        // Process
        analzyer.process(&input_buffer, &mut output_buffer);

        let bytes: &[u8] = cast_slice(&output_buffer);
        let header = MsgHeader::new(MsgType::Spectrum, bytes.len());
        // Write header and bytes
        header.write_to_io(&mut buf_writer)?;
        buf_writer.write_all(bytes)?;
        println!("Sending {} bytes", bytes.len());
        // flush
        buf_writer.flush()?;

        std::thread::sleep(std::time::Duration::from_millis(1000 / 30));
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let socket_path = args.socket_path;

    // Clean up any pre-existing socket file
    if socket_path.exists() {
        fs::remove_file(socket_path.clone())?;
    }

    // Bind and listen
    let listener = UnixListener::bind(socket_path.clone())?;
    println!("🦀  Echo server listening on {}", socket_path.display());

    // Accept-loop (sequential; wrap in threads/tokio for concurrency)
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("Client connected");
                if let Err(e) = handle_client(stream) {
                    eprintln!("Client error: {e}");
                }
                println!("Client disconnected");
            }
            Err(err) => eprintln!("Accept error: {err}"),
        }
    }

    Ok(())
}
