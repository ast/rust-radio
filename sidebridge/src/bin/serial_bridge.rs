// Copyright (c) SM6WJM 2026

use tokio::io::copy;
use tokio::net::TcpListener;
use tokio_serial::SerialPortBuilderExt;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Transparent TCP to serial bridge")]
struct Args {
    /// Serial port path (e.g. /dev/ic-705a)
    #[arg(short, long, default_value = "/dev/ic-705a")]
    serial: String,

    /// Baud rate for the serial port
    #[arg(short, long, default_value_t = 115200)]
    baud: u32,

    /// TCP listen address and port
    #[arg(short, long, default_value = "0.0.0.0:9000")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut serial_stream = tokio_serial::new(&args.serial, args.baud).open_native_async()?;

    let listener = TcpListener::bind(&args.listen).await?;
    println!("Bridge: {} → {}", args.serial, args.listen);

    loop {
        let (mut tcp_stream, addr) = listener.accept().await?;
        println!("Client connected from {}", addr);

        let (mut tcp_read, mut tcp_write) = tcp_stream.split();
        let (mut ser_read, mut ser_write) = tokio::io::split(&mut serial_stream);

        let client_to_radio = copy(&mut tcp_read, &mut ser_write);
        let radio_to_client = copy(&mut ser_read, &mut tcp_write);

        tokio::select! {
            _ = client_to_radio => println!("Client disconnected."),
            _ = radio_to_client => println!("Radio disconnected."),
        }
    }
}
