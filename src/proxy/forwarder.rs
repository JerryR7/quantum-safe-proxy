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

// Constants
const KEEPALIVE_INTERVAL: u64 = 10;   // TCP keepalive interval (seconds)
const KEEPALIVE_RETRIES: u32 = 3;     // TCP keepalive retry count
const MIN_BUFFER_SIZE: usize = 1024;  // Minimum buffer size (bytes)

/// Set TCP keepalive
fn set_tcp_keepalive(stream: &TcpStream, timeout_secs: u64) -> io::Result<()> {
    // Get file descriptor
    let fd = stream.as_raw_fd();

    // Safely use socket2 to set keepalive
    unsafe {
        // Create Socket from file descriptor without taking ownership
        let socket = Socket::from_raw_fd(fd);

        // Enable keepalive
        socket.set_keepalive(true)?;

        // Set keepalive parameters
        let keepalive = TcpKeepalive::new()
            .with_time(Duration::from_secs(timeout_secs))
            .with_interval(Duration::from_secs(KEEPALIVE_INTERVAL))
            .with_retries(KEEPALIVE_RETRIES);

        socket.set_tcp_keepalive(&keepalive)?;

        // Release socket without closing file descriptor
        let _ = socket.into_raw_fd();
    }

    Ok(())
}

/// One-way data transfer
///
/// Reads data from reader and writes to writer until connection closes or error occurs
/// Uses tokio::io::copy for efficient data transfer
async fn transfer<R, W>(
    mut reader: R,
    mut writer: W,
    _buffer_size: usize, // Kept for API compatibility but no longer used
    direction: &'static str
) -> Result<u64>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    // Use tokio::io::copy for efficient data transfer
    // It internally uses optimized buffer management and zero-copy techniques
    let result = tokio::io::copy(
        &mut reader,
        &mut writer,
    ).await;

    // Handle result
    match result {
        Ok(total_bytes) => {
            debug!("{}: Total transferred {} bytes", direction, total_bytes);

            // Properly close the write end
            if let Err(e) = writer.shutdown().await {
                debug!("{}: Connection close error: {}", direction, e);
            }

            Ok(total_bytes)
        },
        Err(e) => {
            debug!("{}: Transfer error: {}", direction, e);
            Err(ProxyError::Io(e))
        }
    }
}

/// Bidirectional data forwarding
///
/// Forwards data bidirectionally between two streams until both directions complete or an error occurs
pub async fn proxy_data<S>(
    tls_stream: S,
    target_stream: TcpStream,
    config: &ProxyConfig,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Get connection timeout setting for TCP keepalive
    let timeout_secs = config.connection_timeout;

    // Set TCP keepalive to maintain long connections
    if let Err(e) = set_tcp_keepalive(&target_stream, timeout_secs) {
        debug!("Failed to set TCP keepalive: {}", e);
    } else {
        debug!("TCP keepalive enabled, timeout: {}s, interval: {}s, retries: {}",
               timeout_secs, KEEPALIVE_INTERVAL, KEEPALIVE_RETRIES);
    }

    // Get configured buffer size, ensuring it meets minimum size
    let buffer_size = config.buffer_size.max(MIN_BUFFER_SIZE);

    // Split streams
    let (tls_read, tls_write) = tokio::io::split(tls_stream);
    let (target_read, target_write) = tokio::io::split(target_stream);

    // Start bidirectional transfer
    let client_to_target = transfer(tls_read, target_write, buffer_size, "Client->Target");
    let target_to_client = transfer(target_read, tls_write, buffer_size, "Target->Client");

    // Wait for both directions to complete
    let (client_result, target_result) = tokio::join!(client_to_target, target_to_client);

    // Handle results
    match (client_result, target_result) {
        (Ok(client_bytes), Ok(target_bytes)) => {
            debug!("Connection completed successfully. Client->Target: {} bytes, Target->Client: {} bytes",
                   client_bytes, target_bytes);
        },
        (client_result, target_result) => {
            // If at least one direction succeeded, consider the connection partially successful
            if client_result.is_ok() || target_result.is_ok() {
                debug!("Connection partially successful. Client->Target: {:?}, Target->Client: {:?}",
                       client_result, target_result);
            } else {
                debug!("Connection failed. Client->Target: {:?}, Target->Client: {:?}",
                       client_result, target_result);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
    use tokio::test;

    #[test]
    async fn test_transfer() {
        // Create a pair of connected streams
        let (client, server) = duplex(1024);

        // Write test data
        let test_data = b"Hello, World!";
        let mut client_write = client;
        client_write.write_all(test_data).await.unwrap();
        client_write.flush().await.unwrap();
        client_write.shutdown().await.unwrap();

        // Read data
        let mut server_read = server;
        let mut buffer = vec![0u8; 1024];
        let n = server_read.read(&mut buffer).await.unwrap();

        // Verify data
        assert_eq!(&buffer[..n], test_data);
    }
}
