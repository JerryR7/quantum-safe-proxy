//! TLS acceptor module
//!
//! This module provides functionality for creating TLS acceptors.

use log::{debug, info, warn};
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
    // 創建提供者以檢測環境能力
    // 自動選擇最適合的提供者，優先選擇 OQS-OpenSSL（如果可用）
    let provider = create_provider(ProviderType::Auto)?;
    let capabilities = provider.capabilities();

    // 記錄所使用的提供者和其能力
    log::info!("Using crypto provider: {} (PQC support: {})",
              provider.name(),
              if capabilities.supports_pqc { "available" } else { "not available" });
    log::debug!("Provider supports PQC: {}", capabilities.supports_pqc);

    // Create TLS acceptor
    let mut acceptor = SslAcceptor::mozilla_modern(SslMethod::tls())
        .map_err(|e| ProxyError::Certificate(format!("Failed to create SSL acceptor: {}", e)))?;

    // Use a cipher list based on system capabilities
    // This includes high and medium strength ciphers but excludes anonymous and weak ciphers
    let cipher_list = &capabilities.recommended_cipher_list;
    acceptor.set_cipher_list(cipher_list)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set cipher list: {}", e)))?;

    // Log TLS settings at trace level for detailed debugging
    log::trace!("Using TLS cipher list: {}", cipher_list);

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

    // Log provider capabilities only if they haven't been logged already
    LOGGED_PQC_STATUS.with(|logged| {
        if !logged.get() {
            if capabilities.supports_pqc && !capabilities.supported_key_exchange.is_empty() {
                let pqc_algos = capabilities.supported_key_exchange.iter()
                    .filter(|alg| alg.contains("Kyber") || alg.contains("NTRU"))
                    .cloned().collect::<Vec<_>>();

                if !pqc_algos.is_empty() {
                    debug!("Supported PQC key exchange algorithms: {}", pqc_algos.join(", "));
                }
            }
            logged.set(true);
        }
    });

    // Set TLS 1.3 ciphersuites including post-quantum ones if available
    // This is a critical step for enabling post-quantum key exchange
    // Use ciphersuites based on system capabilities
    let tls13_ciphersuites = &capabilities.recommended_tls13_ciphersuites;
    acceptor.set_ciphersuites(tls13_ciphersuites)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set TLS 1.3 ciphersuites: {}", e)))?;

    log::trace!("Using TLS 1.3 ciphersuites: {}", tls13_ciphersuites);

    // Enable post-quantum groups if available
    // The actual groups will be determined by the OQS provider
    // Traditional groups are listed first for better compatibility
    let groups = &capabilities.recommended_groups;
    acceptor.set_groups_list(groups)
        .map_err(|e| ProxyError::Certificate(format!("Failed to set groups: {}", e)))?;

    log::trace!("Using TLS groups: {}", groups);

    // Log a summary of TLS settings at debug level
    log::debug!("TLS settings configured with {} cipher suites and {} groups",
              tls13_ciphersuites.split(':').count(),
              groups.split(':').count());

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
        let result = create_tls_acceptor(&cert_path, &key_path, &ca_cert_path, &ClientCertMode::Optional);
        assert!(result.is_ok(), "Should be able to create TLS acceptor");
    }
}
