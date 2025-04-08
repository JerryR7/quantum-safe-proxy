//! TLS acceptor module
//!
//! This module provides functionality for creating TLS acceptors.

use log::{info, warn};
use openssl::ssl::{SslMethod, SslAcceptor, SslFiletype, SslVerifyMode};
use std::path::Path;

use crate::common::{ProxyError, Result};
use super::cert::is_hybrid_cert;

/// Create a TLS acceptor with support for hybrid certificates
///
/// # Parameters
///
/// * `cert_path` - Server certificate path
/// * `key_path` - Server private key path
/// * `ca_cert_path` - CA certificate path, used for client certificate validation
///
/// # Returns
///
/// Returns a configured SSL acceptor
///
/// # Errors
///
/// Returns an error if the acceptor cannot be created or certificates cannot be set.
///
/// # Example
///
/// ```no_run
/// # use std::path::Path;
/// # use quantum_proxy::tls::create_tls_acceptor;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let acceptor = create_tls_acceptor(
///     Path::new("certs/server.crt"),
///     Path::new("certs/server.key"),
///     Path::new("certs/ca.crt")
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_tls_acceptor(
    cert_path: &Path,
    key_path: &Path,
    ca_cert_path: &Path,
) -> Result<SslAcceptor> {
    // Create TLS acceptor
    let mut acceptor = SslAcceptor::mozilla_modern(SslMethod::tls())
        .map_err(ProxyError::Ssl)?;

    // Set server certificate and private key
    acceptor.set_certificate_file(cert_path, SslFiletype::PEM)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set server certificate: {}", e)))?;
    acceptor.set_private_key_file(key_path, SslFiletype::PEM)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set server private key: {}", e)))?;

    // Enable client certificate validation
    acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);

    // Set CA certificate for client certificate validation
    acceptor.set_ca_file(ca_cert_path)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set CA certificate: {}", e)))?;

    // Check if certificate is a hybrid certificate
    if let Ok(is_hybrid) = is_hybrid_cert(cert_path) {
        if is_hybrid {
            info!("Hybrid certificate mode enabled");
        } else {
            warn!("Using traditional certificate, not hybrid");
        }
    }

    // Build the acceptor
    Ok(acceptor.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_create_tls_acceptor() {
        // This test requires valid certificate files
        let cert_path = PathBuf::from("certs/server.crt");
        let key_path = PathBuf::from("certs/server.key");
        let ca_cert_path = PathBuf::from("certs/ca.crt");

        if !cert_path.exists() || !key_path.exists() || !ca_cert_path.exists() {
            println!("Skipping test: Certificate files do not exist");
            return;
        }

        // Test creating TLS acceptor
        let result = create_tls_acceptor(&cert_path, &key_path, &ca_cert_path);
        assert!(result.is_ok(), "Should be able to create TLS acceptor");
    }
}
