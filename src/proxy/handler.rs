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
/// Check if connection uses TLS protocol
///
/// Determines if connection uses TLS by examining the first few bytes.
/// If not a TLS connection, sends TCP RST to immediately close the connection.
async fn ensure_tls_connection(stream: TcpStream) -> Result<TcpStream> {
    // Enable TCP_NODELAY for faster response
    stream.set_nodelay(true).map_err(ProxyError::Io)?;

    // Check first few bytes to determine if it's a TLS ClientHello
    let mut peek_buf = [0u8; 5];

    // Use timeout to avoid infinite waiting, but with shorter timeout
    match tokio::time::timeout(Duration::from_millis(100), stream.peek(&mut peek_buf)).await {
        // Successfully peeked data
        Ok(Ok(size)) if size >= 3 => {
            // TLS handshake starts with content type 0x16 (22 decimal)
            if peek_buf[0] != 0x16 {
                info!("Non-TLS connection: first byte is {:#04x}, expected 0x16", peek_buf[0]);
                send_tcp_rst(&stream)?;
                return Err(ProxyError::NonTlsConnection(format!("Invalid protocol: first byte {:#04x}", peek_buf[0])));
            }

            debug!("TLS connection detected, continuing handshake");
            Ok(stream)
        },
        // Not enough data to determine protocol or timeout waiting for data
        _ => {
            // For non-TLS connections, clients typically don't send data immediately
            // So we assume this is a non-TLS connection
            debug!("No TLS handshake data detected, assuming non-TLS connection");
            send_tcp_rst(&stream)?;
            Err(ProxyError::NonTlsConnection("No TLS handshake data detected".to_string()))
        }
    }
}

/// Send TCP RST packet to immediately close connection
fn send_tcp_rst(stream: &TcpStream) -> Result<()> {
    // Setting SO_LINGER to 0 will send TCP RST when closing
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
    // First ensure this is a TLS connection
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
