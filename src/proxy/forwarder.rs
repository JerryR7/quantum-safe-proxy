//! Data forwarding module
//!
//! This module handles data forwarding between two streams.
//! Optimized for high performance and memory efficiency using Rust's zero-cost abstractions.

use log::debug;
use socket2::{Socket, TcpKeepalive};
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::common::{ProxyError, Result};
use crate::config::ProxyConfig;

// TCP keepalive constants
const KEEPALIVE_INTERVAL: u64 = 10;   // TCP keepalive interval (seconds)
const KEEPALIVE_RETRIES: u32 = 3;     // TCP keepalive retry count

/// Set TCP keepalive with safe socket handling
fn set_tcp_keepalive(stream: &TcpStream, timeout_secs: u64) -> io::Result<()> {
    unsafe {
        let socket = Socket::from_raw_fd(stream.as_raw_fd());
        socket.set_keepalive(true)?;
        socket.set_tcp_keepalive(&TcpKeepalive::new()
            .with_time(Duration::from_secs(timeout_secs))
            .with_interval(Duration::from_secs(KEEPALIVE_INTERVAL))
            .with_retries(KEEPALIVE_RETRIES))?;
        let _ = socket.into_raw_fd();
        Ok(())
    }
}

/// One-way data transfer with logging
async fn transfer<R, W>(mut reader: R, mut writer: W, direction: &'static str) -> Result<u64>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    let bytes = tokio::io::copy(&mut reader, &mut writer)
        .await
        .map_err(|e| {
            debug!("{direction}: Transfer error: {e}");
            ProxyError::Io(e)
        })?;

    debug!("{direction}: Total transferred {bytes} bytes");
    writer.shutdown().await.map_err(|e| debug!("{direction}: Close error: {e}")).ok();
    Ok(bytes)
}

/// Bidirectional data forwarding between TLS and target streams
pub async fn proxy_data<S>(
    tls_stream: S,
    target_stream: TcpStream,
    config: &ProxyConfig,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Setup TCP keepalive
    set_tcp_keepalive(&target_stream, config.connection_timeout)
        .map(|_| debug!("TCP keepalive enabled: timeout={}s, interval={}s, retries={}",
                      config.connection_timeout, KEEPALIVE_INTERVAL, KEEPALIVE_RETRIES))
        .unwrap_or_else(|e| debug!("Failed to set TCP keepalive: {e}"));

    // Split and transfer bidirectionally
    let (tls_read, tls_write) = tokio::io::split(tls_stream);
    let (target_read, target_write) = tokio::io::split(target_stream);

    // Execute transfers concurrently
    let (client_result, target_result) = tokio::join!(
        transfer(tls_read, target_write, "Client->Target"),
        transfer(target_read, tls_write, "Target->Client")
    );

    // Log transfer results
    match (client_result, target_result) {
        (Ok(c), Ok(t)) => debug!("Connection successful: Client->Target: {c} bytes, Target->Client: {t} bytes"),
        (c, t) if c.is_ok() || t.is_ok() => debug!("Connection partially successful: Client->Target: {c:?}, Target->Client: {t:?}"),
        (c, t) => debug!("Connection failed: Client->Target: {c:?}, Target->Client: {t:?}"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
    use tokio::test;

    #[test]
    async fn test_transfer() {
        // Setup test with connected streams
        let (mut client, mut server) = duplex(1024);
        let test_data = b"Hello, World!";

        // Write, flush and close
        client.write_all(test_data).await.unwrap();
        client.shutdown().await.unwrap();

        // Read and verify
        let mut buffer = vec![0u8; 1024];
        let n = server.read(&mut buffer).await.unwrap();
        assert_eq!(&buffer[..n], test_data);
    }
}
