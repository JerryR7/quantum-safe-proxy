//! Connection handler module
//!
//! This module handles individual client connections.

use log::{info, error, debug};
use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_openssl::SslStream;

use crate::common::{ProxyError, Result};
use super::forwarder::proxy_data;

/// Handle a single client connection
///
/// # Parameters
///
/// * `client_stream` - Client TCP stream
/// * `target_addr` - Target service address
/// * `tls_acceptor` - TLS acceptor
///
/// # Returns
///
/// Returns `Ok(())` if handling is successful, otherwise returns an error.
pub async fn handle_connection(
    client_stream: TcpStream,
    target_addr: SocketAddr,
    tls_acceptor: Arc<SslAcceptor>,
) -> Result<()> {
    // Set up TLS connection
    let ssl = openssl::ssl::Ssl::new(tls_acceptor.context())
        .map_err(ProxyError::Ssl)?;
    let stream = SslStream::new(ssl, client_stream)
        .map_err(ProxyError::Ssl)?;

    // Perform TLS handshake
    use std::pin::Pin;
    let mut stream = Pin::new(Box::new(stream));

    if let Err(e) = stream.as_mut().accept().await {
        error!("TLS handshake failed: {}", e);
        return Err(ProxyError::TlsHandshake(e.to_string()));
    }

    debug!("TLS handshake successful");

    // Get client certificate information (if available)
    if let Some(cert) = stream.as_ref().get_ref().ssl().peer_certificate() {
        let subject = cert.subject_name();
        // Convert X509NameRef to string
        let subject_str = format!("{:?}", subject);
        info!("Client certificate subject: {}", subject_str);
    }

    // Connect to target service
    let target_stream = TcpStream::connect(target_addr).await
        .map_err(ProxyError::Io)?;

    // Forward data between client and target service
    proxy_data(stream, target_stream).await
}

#[cfg(test)]
mod tests {
    // Unit tests for connection handling could be added here
    // However, since we need to mock TLS connections, this might require more complex test setup
}
