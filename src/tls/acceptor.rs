//! TLS acceptor module
//!
//! This module provides functionality for creating TLS acceptors.

use log::{info, warn};
use openssl::ssl::{SslMethod, SslAcceptor, SslFiletype, SslVerifyMode};
use std::path::Path;

use crate::common::{ProxyError, Result};
use crate::config::ClientCertMode;
use crate::crypto::provider::{ProviderType, create_provider};

/// Create a TLS acceptor with support for hybrid certificates
///
/// # Parameters
///
/// * `cert_path` - Server certificate path
/// * `key_path` - Server private key path
/// * `ca_cert_path` - CA certificate path, used for client certificate validation
/// * `client_cert_mode` - Client certificate verification mode
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
/// # use quantum_safe_proxy::config::ClientCertMode;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let acceptor = create_tls_acceptor(
///     Path::new("certs/hybrid/dilithium3/server.crt"),
///     Path::new("certs/hybrid/dilithium3/server.key"),
///     Path::new("certs/hybrid/dilithium3/ca.crt"),
///     &ClientCertMode::Required
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_tls_acceptor(
    cert_path: &Path,
    key_path: &Path,
    ca_cert_path: &Path,
    client_cert_mode: &ClientCertMode,
) -> Result<SslAcceptor> {
    // Create a crypto provider (auto-select the best available)
    let provider = create_provider(ProviderType::Auto)?;

    // Create TLS acceptor
    let mut acceptor = SslAcceptor::mozilla_modern(SslMethod::tls())
        .map_err(|e| ProxyError::Certificate(format!("Failed to create SSL acceptor: {}", e)))?;

    // Enable all available cipher suites for maximum compatibility
    // This is important for post-quantum TLS testing
    acceptor.set_cipher_list("ALL:COMPLEMENTOFALL")
        .map_err(|e| ProxyError::Certificate(format!("Failed to set cipher list: {}", e)))?;

    // Enable TLSv1.2 and TLSv1.3 for better compatibility
    // Disable older, insecure protocols
    acceptor.set_options(openssl::ssl::SslOptions::NO_TLSV1_1);
    acceptor.set_options(openssl::ssl::SslOptions::NO_TLSV1);
    acceptor.set_options(openssl::ssl::SslOptions::NO_SSLV3);
    acceptor.set_options(openssl::ssl::SslOptions::NO_SSLV2);

    // Set server certificate and private key
    acceptor.set_certificate_file(cert_path, SslFiletype::PEM)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set server certificate: {}", e)))?;
    acceptor.set_private_key_file(key_path, SslFiletype::PEM)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set server private key: {}", e)))?;

    // Configure client certificate validation based on mode
    match client_cert_mode {
        ClientCertMode::Required => {
            info!("Client certificates required for connections");
            acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);

            // Set CA certificate for client certificate validation
            acceptor.set_ca_file(ca_cert_path)
                .map_err(|e| ProxyError::Certificate(format!("Failed to set CA certificate: {}", e)))?;
        },
        ClientCertMode::Optional => {
            info!("Client certificates optional (will be verified if provided)");
            acceptor.set_verify(SslVerifyMode::PEER);

            // Set CA certificate for client certificate validation
            acceptor.set_ca_file(ca_cert_path)
                .map_err(|e| ProxyError::Certificate(format!("Failed to set CA certificate: {}", e)))?;
        },
        ClientCertMode::None => {
            info!("Client certificate verification disabled");
            acceptor.set_verify(SslVerifyMode::NONE);
        },
    };

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

    // Set TLS 1.3 ciphersuites including post-quantum ones if available
    // This is a critical step for enabling post-quantum key exchange
    let tls13_ciphersuites = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256";
    acceptor.set_ciphersuites(tls13_ciphersuites)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set TLS 1.3 ciphersuites: {}", e)))?;

    // Enable post-quantum groups if available
    // The actual groups will be determined by the OQS provider
    let groups = "kyber768:p384_kyber768:kyber512:p256_kyber512:kyber1024:p521_kyber1024:X25519:P-256:P-384:P-521";
    acceptor.set_groups_list(groups)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set groups: {}", e)))?;

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
        let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
        let key_path = PathBuf::from("certs/hybrid/dilithium3/server.key");
        let ca_cert_path = PathBuf::from("certs/hybrid/dilithium3/ca.crt");

        if !cert_path.exists() || !key_path.exists() || !ca_cert_path.exists() {
            println!("Skipping test: Certificate files do not exist");
            return;
        }

        // Test creating TLS acceptor
        let result = create_tls_acceptor(&cert_path, &key_path, &ca_cert_path);
        assert!(result.is_ok(), "Should be able to create TLS acceptor");
    }
}
