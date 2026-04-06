pub mod message;

use crossbeam::channel::Receiver;
use crossbeam::channel::bounded;
use doublemap::{Producer, ring_buffer_pair};
use num_complex::Complex32;
use pool::BufferGuard;
use pool::BufferPool;

use message::{MsgHeader, MsgType};

use std::{
    fs,
    io::Write,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
};

use zerocopy::IntoBytes;

// TODO: rename analyzer to something more correct.
use spectrum::Analyzer;

pub struct SpectrumServer {
    producer: Producer<Complex32>,
}

impl SpectrumServer {
    pub fn new(fft_len: usize) -> Self {
        // Buffer pool for output
        let pool = BufferPool::<f32>::new(10, fft_len);
        // Spectrum analyzer
        let mut analyzer = Analyzer::new(fft_len, 768_000.0);

        let (producer, consumer) = ring_buffer_pair::<Complex32>(2048, fft_len);

        // Bounded channel for power density spectrum
        let (pds_tx, pds_rx) = bounded(4);

        // FFT worker thread
        std::thread::spawn(move || {
            println!("Starting consumer thread with FFT length: {}", fft_len);

            loop {
                let result = consumer.consume(|input| {
                    println!("Got {} samples", input.len());

                    let mut pds = pool.get();
                    // Process the input samples
                    analyzer.process(input, &mut pds);

                    // Try send power density spectrum, none blocking
                    match pds_tx.try_send(pds) {
                        Ok(_) => {
                            println!("Sent power density spectrum");
                        }
                        Err(err) => {
                            eprintln!("Failed to send power density spectrum: {err}");
                        }
                    }

                    // Consume the input buffer, which is at least fft_len long
                    fft_len
                });
                if result.is_err() {
                    eprintln!("Producer disconnected, stopping FFT worker");
                    break;
                }
            }
        });

        // Socket server thread
        std::thread::spawn(move || {
            let socket_path = PathBuf::from("/tmp/echo.sock");

            Self::listen_socket(socket_path, pds_rx).unwrap_or_else(|err| {
                eprintln!("Failed to start socket server: {err}");
            });
        });

        // Return the SpectrumServer instance
        SpectrumServer { producer }
    }

    pub fn process(&self, samples: &[Complex32]) {
        let _ = self.producer.produce(|slice| {
            slice[..samples.len()].copy_from_slice(samples);
            samples.len()
        });
    }

    fn listen_socket(socket_path: PathBuf, rx: Receiver<BufferGuard<f32>>) -> std::io::Result<()> {
        // Clean up any pre-existing socket file
        if socket_path.exists() {
            fs::remove_file(socket_path.clone())?;
        }

        // Bind and listen
        let listener = UnixListener::bind(socket_path.clone()).expect("Failed to bind socket");

        println!("🦀  Echo server listening on {}", socket_path.display());

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("Client connected");
                    if let Err(e) = Self::handle_client(stream, rx.clone()) {
                        eprintln!("Client error: {e}");
                    }
                    println!("Client disconnected");
                }
                Err(err) => eprintln!("Accept error: {err}"),
            }
        }

        Ok(())
    }

    fn handle_client(stream: UnixStream, rx: Receiver<BufferGuard<f32>>) -> std::io::Result<()> {
        for pds in rx.iter() {
            // Convert the power density spectrum to bytes
            let bytes: &[u8] = bytemuck::cast_slice(&pds);
            let header = MsgHeader::new(MsgType::Spectrum, bytes.len());

            // Write header and bytes to the stream
            let mut buf_writer = std::io::BufWriter::new(stream.try_clone()?);
            header.write_to_io(&mut buf_writer)?;
            buf_writer.write_all(bytes)?;

            println!("Sent {} bytes", bytes.len());

            // Flush the buffer
            buf_writer.flush()?;
        }

        Ok(())
    }
}
