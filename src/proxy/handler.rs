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
    // Set up TLS connection
    let ssl = openssl::ssl::Ssl::new(tls_acceptor.context())
        .map_err(ProxyError::Ssl)?;
    let stream = SslStream::new(ssl, client_stream)
        .map_err(ProxyError::Ssl)?;

    // Perform TLS handshake
    use std::pin::Pin;
    let mut stream = Pin::new(Box::new(stream));

    match stream.as_mut().accept().await {
        Ok(_) => {
            debug!("TLS handshake successful");

            // Log TLS connection details
            let ssl = stream.as_ref().get_ref().ssl();
            debug!("TLS version: {}", ssl.version_str());
            debug!("TLS cipher: {}", ssl.current_cipher().map_or("None".to_string(), |c| c.name().to_string()));
            let protocol = ssl.version_str();
            debug!("TLS protocol: {}", protocol);

            if let Some(servername) = ssl.servername(openssl::ssl::NameType::HOST_NAME) {
                debug!("TLS SNI: {}", servername);
            } else {
                debug!("TLS SNI: None");
            }

            // Log TLS connection details
            debug!("Client signature algorithms not available in this OpenSSL version");
        },
        Err(e) => {
            // Get detailed error information
            let ssl_error = stream.as_ref().get_ref().ssl().verify_result();
            error!("TLS handshake failed: {}", e);
            error!("TLS verify result: {}", ssl_error);

            // Try to get more detailed OpenSSL error information
            if let Some(err_str) = e.to_string().strip_prefix("error:") {
                if let Some(code_str) = err_str.split(':').next() {
                    error!("OpenSSL error code: {}", code_str);
                }
            }

            return Err(ProxyError::TlsHandshake(e.to_string()));
        }
    }

    // Get client certificate information (if available)
    if let Some(cert) = stream.as_ref().get_ref().ssl().peer_certificate() {
        let subject = cert.subject_name();
        // Convert X509NameRef to string
        let subject_str = format!("{:?}", subject);
        info!("Client certificate subject: {}", subject_str);
    }

    // Connect to target service with timeout
    // Use lock-free access method to get connection timeout for better performance
    let connect_timeout = Duration::from_secs(config::get_connection_timeout());
    let target_stream = timeout(connect_timeout, TcpStream::connect(target_addr))
        .await
        .map_err(|_| ProxyError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "Connection timed out")))?
        .map_err(ProxyError::Io)?;

    // Forward data between client and target service
    // Use lock-free access method to get buffer size for better performance
    let buffer_size = config::get_buffer_size();
    let mut config_clone = config.clone();
    config_clone.buffer_size = buffer_size; // Ensure we use the cached value
    proxy_data(stream, target_stream, config_clone).await
}

#[cfg(test)]
mod tests {
    // Unit tests for connection handling could be added here
    // However, since we need to mock TLS connections, this might require more complex test setup
}
