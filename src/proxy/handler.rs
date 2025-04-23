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

use crate::config::{self, ProxyConfig};

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
pub async fn handle_connection(
    client_stream: TcpStream,
    target_addr: SocketAddr,
    tls_acceptor: Arc<SslAcceptor>,
    config: &ProxyConfig,
) -> Result<()> {
    // Set up TLS connection with optimized settings
    let ssl = openssl::ssl::Ssl::new(tls_acceptor.context())
        .map_err(ProxyError::Ssl)?;

    // Create SslStream and wrap in Pin<Box<>> for async operations
    // We use a single allocation for the SslStream to reduce memory overhead
    let stream = SslStream::new(ssl, client_stream)
        .map_err(ProxyError::Ssl)?;

    use std::pin::Pin;
    let mut stream = Pin::new(Box::new(stream));

    match stream.as_mut().accept().await {
        Ok(_) => {
            debug!("TLS handshake successful");

            // Only log TLS details when debug logging is enabled (performance optimization)
            if log::log_enabled!(log::Level::Debug) {
                let ssl = stream.as_ref().get_ref().ssl();
                debug!("TLS version: {}", ssl.version_str());

                // Avoid unnecessary string allocations
                if let Some(cipher) = ssl.current_cipher() {
                    debug!("TLS cipher: {}", cipher.name());
                } else {
                    debug!("TLS cipher: None");
                }

                // Log SNI without unnecessary allocations
                match ssl.servername(openssl::ssl::NameType::HOST_NAME) {
                    Some(servername) => debug!("TLS SNI: {}", servername),
                    None => debug!("TLS SNI: None"),
                }
            }
        },
        Err(e) => {
            // Only get detailed error information when error logging is enabled
            if log::log_enabled!(log::Level::Error) {
                // Get SSL verification result without unnecessary allocations
                let ssl_error = stream.as_ref().get_ref().ssl().verify_result();
                error!("TLS handshake failed: {}", e);
                error!("TLS verify result: {}", ssl_error);

                // Extract OpenSSL error code more efficiently
                let err_string = e.to_string();
                if err_string.starts_with("error:") {
                    if let Some(code_end) = err_string.find(':') {
                        if code_end > 6 { // "error:" is 6 chars
                            error!("OpenSSL error code: {}", &err_string[6..code_end]);
                        }
                    }
                }
            }

            // Create error with borrowed string to avoid allocation
            return Err(ProxyError::TlsHandshake(e.to_string()));
        }
    }

    // Get client certificate information only when info logging is enabled
    if log::log_enabled!(log::Level::Info) {
        if let Some(cert) = stream.as_ref().get_ref().ssl().peer_certificate() {
            // Only format the subject name when needed (avoid unnecessary allocations)
            let subject = cert.subject_name();
            info!("Client certificate subject: {:?}", subject);
        }
    }

    // Connect to target service with timeout
    // Use lock-free access method to get connection timeout for better performance
    let connect_timeout = Duration::from_secs(config::get_connection_timeout());

    // Use more specific error type for better diagnostics
    let target_stream = match timeout(connect_timeout, TcpStream::connect(target_addr)).await {
        Ok(result) => result,
        Err(_) => return Err(ProxyError::ConnectionTimeout(config::get_connection_timeout())),
    }
        .map_err(ProxyError::Io)?;

    // Forward data between client and target service
    // Pass config by reference to avoid cloning
    proxy_data(stream, target_stream, config).await
}

#[cfg(test)]
mod tests {
    // Unit tests for connection handling could be added here
    // However, since we need to mock TLS connections, this might require more complex test setup
}
