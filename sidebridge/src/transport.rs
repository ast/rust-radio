// Copyright (c) SM6WJM 2026

use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{bail, Result};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_serial::SerialPortBuilderExt;
use url::Url;

const DEFAULT_BAUD: u32 = 115200;

pub enum Transport {
    Serial(tokio_serial::SerialStream),
    Tcp(TcpStream),
}

impl Transport {
    pub async fn connect(url: &Url) -> Result<Self> {
        match url.scheme() {
            "serial" => {
                let path = url.path();
                let baud: u32 = url
                    .query_pairs()
                    .find(|(k, _)| k == "baud")
                    .map(|(_, v)| v.parse())
                    .transpose()?
                    .unwrap_or(DEFAULT_BAUD);
                let serial = tokio_serial::new(path, baud).open_native_async()?;
                Ok(Self::Serial(serial))
            }
            "tcp" => {
                let host = url.host_str().unwrap_or("localhost");
                let port = url.port().unwrap_or(9000);
                let stream = TcpStream::connect((host, port)).await?;
                Ok(Self::Tcp(stream))
            }
            scheme => bail!("Unsupported scheme: {scheme}"),
        }
    }
}

impl AsyncRead for Transport {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Transport::Serial(s) => Pin::new(s).poll_read(cx, buf),
            Transport::Tcp(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Transport {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Transport::Serial(s) => Pin::new(s).poll_write(cx, buf),
            Transport::Tcp(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Transport::Serial(s) => Pin::new(s).poll_flush(cx),
            Transport::Tcp(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Transport::Serial(s) => Pin::new(s).poll_shutdown(cx),
            Transport::Tcp(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}
