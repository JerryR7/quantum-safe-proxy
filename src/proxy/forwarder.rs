//! Data forwarding module
//!
//! This module handles data forwarding between two streams.
//! Optimized with buffer pool for efficient memory management.

use log::debug;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::common::{BufferPool, Result};
use crate::config::ProxyConfig;

/// Forward data between two streams using a buffer pool for efficient memory management
///
/// # Parameters
///
/// * `tls_stream` - TLS stream
/// * `target_stream` - Target TCP stream
/// * `config` - Proxy configuration reference (optimized to avoid cloning)
///
/// # Returns
///
/// Returns `Ok(())` if forwarding is successful, otherwise returns an error.
pub async fn proxy_data<S>(
    tls_stream: S,
    target_stream: TcpStream,
    _config: &ProxyConfig, // Unused but kept for API compatibility
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Use two tasks to handle bidirectional data flow simultaneously
    let (mut tls_reader, mut tls_writer) = tokio::io::split(tls_stream);
    let (mut target_reader, mut target_writer) = tokio::io::split(target_stream);

    // Create a shared buffer pool with max 32 buffers of the configured size
    // This allows efficient buffer reuse across all connections
    static BUFFER_POOL: Lazy<Arc<BufferPool>> = Lazy::new(|| {
        Arc::new(BufferPool::new(32, 8192))
    });

    // Data flow from client to target
    let client_to_target = {
        // Get a separate reference to the buffer pool for this task
        let buffer_pool = Arc::clone(&BUFFER_POOL);

        tokio::spawn(async move {
            let mut total_bytes = 0;

            // Get a buffer from the pool
            let mut pooled_buffer = buffer_pool.get_buffer().await;

            loop {
                // Use the buffer from the pool
                match tls_reader.read(&mut pooled_buffer.buffer[..]).await {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        total_bytes += n;
                        if target_writer.write_all(&pooled_buffer.buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            debug!("Client to target transferred {} bytes total", total_bytes);
            // Buffer is automatically returned to the pool when dropped
        })
    };

    // Data flow from target to client
    let target_to_client = {
        // Get a separate reference to the buffer pool for this task
        let buffer_pool = Arc::clone(&BUFFER_POOL);

        tokio::spawn(async move {
            let mut total_bytes = 0;

            // Get a buffer from the pool
            let mut pooled_buffer = buffer_pool.get_buffer().await;

            loop {
                // Use the buffer from the pool
                match target_reader.read(&mut pooled_buffer.buffer[..]).await {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        total_bytes += n;
                        if tls_writer.write_all(&pooled_buffer.buffer[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            debug!("Target to client transferred {} bytes total", total_bytes);
            // Buffer is automatically returned to the pool when dropped
        })
    };

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
