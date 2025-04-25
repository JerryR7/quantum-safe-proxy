//! Connection handler module
//!
//! This module handles individual client connections.

use log::{info, error, debug};
use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_openssl::SslStream;

use crate::config::{self, ProxyConfig, ClientCertMode};

use crate::common::{ProxyError, Result};
use super::forwarder::proxy_data;

/// Handle a single client connection
///
/// # Parameters
///
/// * `client_stream` - Client TCP stream
/// * `target_addr` - Target service address
/// * `tls_acceptor` - TLS acceptor
/// * `config` - Proxy configuration
///
/// # Returns
///
/// Returns `Ok(())` if handling is successful, otherwise returns an error.
/// Check if a connection is using TLS protocol
///
/// This function checks if the connection is using TLS by peeking at the first few bytes.
/// If it's not a TLS connection, it sends a TCP RST to immediately close the connection.
///
/// # Returns
/// - `Ok(stream)` if the connection is using TLS
/// - `Err(ProxyError)` if the connection is not using TLS or an error occurred
async fn ensure_tls_connection(stream: TcpStream) -> Result<TcpStream> {
    // Enable TCP_NODELAY for faster response
    stream.set_nodelay(true).map_err(ProxyError::Io)?;

    // Peek at the first few bytes to check if it looks like a TLS ClientHello
    let mut peek_buf = [0u8; 5];

    // Use timeout to avoid waiting indefinitely
    match tokio::time::timeout(Duration::from_millis(500), stream.peek(&mut peek_buf)).await {
        // Successfully peeked at data
        Ok(Ok(size)) if size >= 3 => {
            // TLS handshake starts with content type 0x16 (22 decimal)
            if peek_buf[0] != 0x16 {
                debug!("Not a TLS connection: first byte is {:#04x}, expected 0x16", peek_buf[0]);
                send_tcp_rst(&stream)?;
                return Err(ProxyError::NonTlsConnection(format!("Invalid protocol: first byte {:#04x}", peek_buf[0])));
            }

            debug!("Detected TLS connection, proceeding with handshake");
            Ok(stream)
        },
        // Not enough data to determine protocol
        Ok(Ok(size)) => {
            debug!("Insufficient data ({} bytes) to determine protocol", size);
            send_tcp_rst(&stream)?;
            Err(ProxyError::NonTlsConnection(format!("Insufficient data: only {} bytes received", size)))
        },
        // Error reading from socket
        Ok(Err(e)) => {
            debug!("Error reading from socket: {}", e);
            send_tcp_rst(&stream)?;
            Err(ProxyError::Io(e))
        },
        // Timeout waiting for data
        Err(_) => {
            debug!("Timeout waiting for initial data");
            send_tcp_rst(&stream)?;
            Err(ProxyError::NonTlsConnection("Timeout waiting for initial data".to_string()))
        }
    }
}

/// Send a TCP RST packet to immediately close the connection
fn send_tcp_rst(stream: &TcpStream) -> Result<()> {
    // Setting SO_LINGER with a timeout of 0 causes a TCP RST to be sent on close
    stream.set_linger(Some(Duration::from_secs(0)))
        .map_err(|e| {
            debug!("Failed to set TCP RST option: {}", e);
            ProxyError::Io(e)
        })
}

pub async fn handle_connection(
    client_stream: TcpStream,
    target_addr: SocketAddr,
    tls_acceptor: Arc<SslAcceptor>,
    config: &ProxyConfig,
) -> Result<()> {
    // First, ensure this is a TLS connection
    let client_stream = ensure_tls_connection(client_stream).await?;

    // Setup TLS with client verification mode
    let mut ssl = openssl::ssl::Ssl::new(tls_acceptor.context()).map_err(ProxyError::Ssl)?;
    ssl.set_verify(match config.client_cert_mode {
        ClientCertMode::Required => openssl::ssl::SslVerifyMode::PEER | openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT,
        ClientCertMode::Optional => openssl::ssl::SslVerifyMode::PEER,
        ClientCertMode::None => openssl::ssl::SslVerifyMode::NONE,
    });

    // Create and accept TLS stream
    let mut stream = Box::pin(SslStream::new(ssl, client_stream).map_err(ProxyError::Ssl)?);

    // Perform TLS handshake with error handling
    if let Err(e) = stream.as_mut().accept().await {
        // Log error details if error logging is enabled
        if log::log_enabled!(log::Level::Error) {
            let ssl_error = stream.as_ref().get_ref().ssl().verify_result();
            error!("TLS handshake failed: {e}, verify result: {ssl_error}");

            // Extract OpenSSL error code if present
            e.to_string().strip_prefix("error:").and_then(|s| s.find(':'))
                .map(|code_end| error!("OpenSSL error code: {}", &e.to_string()[6..6+code_end]));
        }
        return Err(ProxyError::TlsHandshake(e.to_string()));
    }

    debug!("TLS handshake successful");

    // Log TLS details and client certificate when appropriate
    if let (true, ssl) = (log::log_enabled!(log::Level::Debug), stream.as_ref().get_ref().ssl()) {
        debug!("TLS version: {}", ssl.version_str());
        debug!("TLS cipher: {}", ssl.current_cipher().map_or("None", |c| c.name()));
        debug!("TLS SNI: {}", ssl.servername(openssl::ssl::NameType::HOST_NAME).unwrap_or("None"));

        // Log client certificate if present and info logging is enabled
        if log::log_enabled!(log::Level::Info) {
            ssl.peer_certificate()
                .map(|cert| info!("Client certificate subject: {:?}", cert.subject_name()));
        }
    }

    // Connect to target with timeout
    let target_stream = timeout(
        Duration::from_secs(config::get_connection_timeout()),
        TcpStream::connect(target_addr)
    )
    .await
    .map_err(|_| ProxyError::ConnectionTimeout(config::get_connection_timeout()))?
    .map_err(ProxyError::Io)?;

    // Forward data between client and target
    proxy_data(stream, target_stream, config).await
}

#[cfg(test)]
mod tests {
    // Unit tests for connection handling could be added here
    // However, since we need to mock TLS connections, this might require more complex test setup
}
