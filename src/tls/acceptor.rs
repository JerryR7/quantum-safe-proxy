//! TLS acceptor module
//!
//! This module provides functionality for creating TLS acceptors.

use log::{info, warn};
use openssl::ssl::{SslMethod, SslAcceptor, SslFiletype, SslVerifyMode};
use std::path::Path;

use crate::common::{ProxyError, Result};
use crate::crypto::provider::{ProviderType, create_provider};

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
/// # use quantum_safe_proxy::tls::create_tls_acceptor;
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
    // Create a crypto provider (auto-select the best available)
    let provider = create_provider(ProviderType::Auto)?;

    // Create TLS acceptor
    let mut acceptor = SslAcceptor::mozilla_modern(SslMethod::tls())
        .map_err(|e| ProxyError::Certificate(format!("Failed to create SSL acceptor: {}", e)))?;

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

    // Use thread-local storage for logging state to avoid static variables
    thread_local! {
        static LOGGED_CERT_TYPE: std::cell::Cell<bool> = std::cell::Cell::new(false);
        static LOGGED_PQC_STATUS: std::cell::Cell<bool> = std::cell::Cell::new(false);
    }

    // Check if certificate is a hybrid certificate using the provider
    LOGGED_CERT_TYPE.with(|logged| {
        if !logged.get() {
            if let Ok(is_hybrid) = provider.is_hybrid_cert(cert_path) {
                if is_hybrid {
                    info!("Hybrid certificate mode enabled (using {})", provider.name());
                } else {
                    warn!("Using traditional certificate, not hybrid (using {})", provider.name());
                }
                logged.set(true);
            }
        }
    });

    // Log provider capabilities
    let capabilities = provider.capabilities();
    LOGGED_PQC_STATUS.with(|logged| {
        if !logged.get() {
            if capabilities.supports_pqc {
                info!("Post-quantum cryptography support is available");
                if !capabilities.supported_key_exchange.is_empty() {
                    let pqc_algos = capabilities.supported_key_exchange.iter()
                        .filter(|alg| alg.contains("Kyber") || alg.contains("NTRU"))
                        .cloned().collect::<Vec<_>>();

                    if !pqc_algos.is_empty() {
                        info!("Supported PQC key exchange algorithms: {}", pqc_algos.join(", "));
                    }
                }
            } else {
                warn!("Post-quantum cryptography support is NOT available");
            }
            logged.set(true);
        }
    });

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
