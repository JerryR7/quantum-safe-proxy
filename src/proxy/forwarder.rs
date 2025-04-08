//! Data forwarding module
//!
//! This module handles data forwarding between two streams.

use log::debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::common::Result;
use crate::BUFFER_SIZE;

/// Forward data between two streams
///
/// # Parameters
///
/// * `tls_stream` - TLS stream
/// * `target_stream` - Target TCP stream
///
/// # Returns
///
/// Returns `Ok(())` if forwarding is successful, otherwise returns an error.
pub async fn proxy_data<S>(
    tls_stream: S,
    target_stream: TcpStream,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Use two tasks to handle bidirectional data flow simultaneously
    let (mut tls_reader, mut tls_writer) = tokio::io::split(tls_stream);
    let (mut target_reader, mut target_writer) = tokio::io::split(target_stream);

    // Data flow from client to target
    let client_to_target = tokio::spawn(async move {
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_bytes = 0;

        loop {
            match tls_reader.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    total_bytes += n;
                    if target_writer.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        debug!("Client to target transferred {} bytes total", total_bytes);
    });

    // Data flow from target to client
    let target_to_client = tokio::spawn(async move {
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_bytes = 0;

        loop {
            match target_reader.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    total_bytes += n;
                    if tls_writer.write_all(&buffer[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        debug!("Target to client transferred {} bytes total", total_bytes);
    });

    // Wait for either task to complete
    tokio::select! {
        _ = client_to_target => debug!("Client to target connection closed"),
        _ = target_to_client => debug!("Target to client connection closed"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Unit tests for data forwarding could be added here
    // However, since we need to mock async IO, this might require more complex test setup
}
