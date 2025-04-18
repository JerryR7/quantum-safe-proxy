//! OpenSSL cryptographic provider
//!
//! This module provides an implementation of the CryptoProvider trait
//! using OpenSSL. It supports both standard OpenSSL and OpenSSL 3.5+
//! with post-quantum cryptography capabilities.
//!
//! This implementation is only available when the "openssl" feature is enabled.

#![cfg(feature = "openssl")]

use std::path::Path;
use std::sync::Arc;
use log::{debug, info, warn};

// Import OpenSSL types
use openssl::pkey::PKey;
use openssl::ssl::{SslMethod, SslVerifyMode, SslContext as OpenSslContext};
use openssl::x509::X509 as OpenSslX509;

use crate::common::{ProxyError, Result, read_file};
use super::{CryptoProvider, CryptoCapabilities, CertificateType, SslContext, X509};
use super::capabilities::OpenSSLCapabilities;

/// OpenSSL cryptographic provider
///
/// This provider uses OpenSSL for cryptographic operations.
/// It automatically detects if post-quantum cryptography is available
/// (OpenSSL 3.5+) and adjusts its capabilities accordingly.
#[derive(Debug, Clone)]
pub struct OpenSSLProvider {
    /// Detected capabilities of the OpenSSL installation
    capabilities: Arc<OpenSSLCapabilities>,
}

impl OpenSSLProvider {
    /// Create a new OpenSSL provider
    pub fn new() -> Self {
        // Detect OpenSSL capabilities
        let capabilities = Arc::new(OpenSSLCapabilities::detect());

        Self { capabilities }
    }

    /// Determine the type of certificate (traditional or hybrid)
    fn determine_certificate_type(&self, cert: &OpenSslX509) -> CertificateType {
        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object().to_string();
        debug!("Raw signature algorithm string: {}", signature_algorithm);

        // Check for hybrid certificate indicators in OpenSSL 3.5 format
        if signature_algorithm.contains("P256_ML-DSA") ||
           signature_algorithm.contains("P384_ML-DSA") ||
           signature_algorithm.contains("P521_ML-DSA") ||
           signature_algorithm.contains("X25519_ML-KEM") ||
           // Check for hybrid certificate indicators in OQS format
           signature_algorithm.contains("p256_") ||
           signature_algorithm.contains("p384_") ||
           signature_algorithm.contains("p521_") ||
           signature_algorithm.contains("_p256") ||
           signature_algorithm.contains("_p384") ||
           signature_algorithm.contains("_p521") {
            debug!("Detected hybrid certificate with signature algorithm: {}", signature_algorithm);
            return CertificateType::Hybrid;
        }

        // Check for pure post-quantum certificate
        if signature_algorithm.contains("ML-KEM") ||
           signature_algorithm.contains("ML-DSA") ||
           signature_algorithm.contains("SLH-DSA") ||
           // Check for OQS post-quantum algorithms
           signature_algorithm.contains("dilithium") ||
           signature_algorithm.contains("falcon") ||
           signature_algorithm.contains("sphincs") ||
           signature_algorithm.contains("kyber") {
            debug!("Detected post-quantum certificate with signature algorithm: {}", signature_algorithm);
            return CertificateType::PostQuantum;
        }

        // Default to traditional
        debug!("Detected traditional certificate with signature algorithm: {}", signature_algorithm);
        CertificateType::Traditional
    }

    #[cfg(not(feature = "openssl"))]
    fn determine_certificate_type(&self, _cert: &X509) -> CertificateType {
        // Fallback implementation when OpenSSL is not available
        CertificateType::Traditional
    }
}

impl CryptoProvider for OpenSSLProvider {
    fn create_server_context(&self, cert_path: &Path, key_path: &Path, ca_path: Option<&Path>) -> Result<SslContext> {
        // Implementation returns OpenSslContext which is converted to SslContext
        #[cfg(feature = "openssl")]
        let mut ctx = OpenSslContext::builder(SslMethod::tls_server())?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        // Set certificate and private key
        let cert_data = read_file(cert_path)?;

        #[cfg(feature = "openssl")]
        let cert = OpenSslX509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        ctx.set_certificate(&cert)?;

        let key_data = read_file(key_path)?;

        #[cfg(feature = "openssl")]
        let key = PKey::private_key_from_pem(&key_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse private key: {}", e)))?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        ctx.set_private_key(&key)?;

        // Check key and certificate compatibility
        ctx.check_private_key()?;

        // Determine certificate type
        let cert_type = self.determine_certificate_type(&cert);

        // Set CA certificate if provided
        if let Some(ca_path) = ca_path {
            ctx.set_ca_file(ca_path)?;

            // Enable client certificate verification
            ctx.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        }

        // Set cipher list
        ctx.set_cipher_list(&self.capabilities.recommended_cipher_list)?;

        // Set TLS 1.3 ciphersuites
        ctx.set_ciphersuites(&self.capabilities.recommended_tls13_ciphersuites)?;

        // Enable post-quantum groups if available
        // The actual groups will be determined by the OpenSSL capabilities
        // Traditional groups are listed first for better compatibility
        let groups = &self.capabilities.recommended_groups;
        ctx.set_groups_list(groups)?;

        // Disable old protocols
        ctx.set_options(
            openssl::ssl::SslOptions::NO_SSLV2 |
            openssl::ssl::SslOptions::NO_SSLV3 |
            openssl::ssl::SslOptions::NO_TLSV1 |
            openssl::ssl::SslOptions::NO_TLSV1_1
        );

        // Log certificate information
        let subject = cert.subject_name();
        // Convert subject to string manually
        let subject_str = format!("{:?}", subject);
        info!("Certificate subject: {}", subject_str);
        let fingerprint = cert.digest(openssl::hash::MessageDigest::sha256())?;
        let fingerprint_hex = fingerprint.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join(":");
        info!("Certificate fingerprint: {}", fingerprint_hex);

        // Log certificate type
        match cert_type {
            CertificateType::Traditional => {
                warn!("Using traditional certificate, not hybrid (using {})", self.name());
                if self.capabilities.supports_pqc {
                    warn!("Post-quantum cryptography is available but not used in certificate");
                }
            },
            CertificateType::Hybrid => {
                info!("Using hybrid certificate (traditional + post-quantum)");
            },
            CertificateType::PostQuantum => {
                info!("Using pure post-quantum certificate");
                if !self.capabilities.supports_pqc {
                    return Err(ProxyError::Certificate(
                        "Post-quantum certificate used but OpenSSL does not support PQC".to_string()
                    ));
                }
            },
        }

        Ok(ctx.build())
    }

    fn create_client_context(&self, cert_path: Option<&Path>, key_path: Option<&Path>, ca_path: Option<&Path>) -> Result<SslContext> {
        // Implementation returns OpenSslContext which is converted to SslContext
        #[cfg(feature = "openssl")]
        let mut ctx = OpenSslContext::builder(SslMethod::tls_client())?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        // Set certificate and private key if provided
        if let (Some(cert_path), Some(key_path)) = (cert_path, key_path) {
            let cert_data = read_file(cert_path)?;
            #[cfg(feature = "openssl")]
            let cert = OpenSslX509::from_pem(&cert_data)?;

            #[cfg(not(feature = "openssl"))]
            return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));
            ctx.set_certificate(&cert)?;

            let key_data = read_file(key_path)?;
            let key = PKey::private_key_from_pem(&key_data)?;
            ctx.set_private_key(&key)?;

            // Check key and certificate compatibility
            ctx.check_private_key()?;
        }

        // Set CA certificate if provided
        if let Some(ca_path) = ca_path {
            ctx.set_ca_file(ca_path)?;
        }

        // Set cipher list
        ctx.set_cipher_list(&self.capabilities.recommended_cipher_list)?;

        // Set TLS 1.3 ciphersuites
        ctx.set_ciphersuites(&self.capabilities.recommended_tls13_ciphersuites)?;

        // Enable post-quantum groups if available
        let groups = &self.capabilities.recommended_groups;
        ctx.set_groups_list(groups)?;

        // Disable old protocols
        ctx.set_options(
            openssl::ssl::SslOptions::NO_SSLV2 |
            openssl::ssl::SslOptions::NO_SSLV3 |
            openssl::ssl::SslOptions::NO_TLSV1 |
            openssl::ssl::SslOptions::NO_TLSV1_1
        );

        Ok(ctx.build())
    }

    fn capabilities(&self) -> CryptoCapabilities {
        CryptoCapabilities {
            supports_pqc: self.capabilities.supports_pqc,
            supported_key_exchange: self.capabilities.supported_key_exchange.clone(),
            supported_signatures: self.capabilities.supported_signatures.clone(),
            recommended_cipher_list: self.capabilities.recommended_cipher_list.clone(),
            recommended_tls13_ciphersuites: self.capabilities.recommended_tls13_ciphersuites.clone(),
            recommended_groups: self.capabilities.recommended_groups.clone(),
        }
    }

    fn name(&self) -> &'static str {
        if self.capabilities.supports_pqc {
            "OpenSSL 3.5+ (with post-quantum support)"
        } else {
            "OpenSSL (standard)"
        }
    }

    fn is_hybrid_cert(&self, cert_path: &Path) -> Result<bool> {
        // Read certificate file
        let cert_data = crate::common::read_file(cert_path)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        #[cfg(feature = "openssl")]
        let cert = OpenSslX509::from_pem(&cert_data)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        // Determine certificate type
        let cert_type = self.determine_certificate_type(&cert);

        // Return whether the certificate is hybrid
        Ok(cert_type == CertificateType::Hybrid)
    }

    fn get_cert_subject(&self, cert_path: &Path) -> Result<String> {
        // Read certificate file
        let cert_data = crate::common::read_file(cert_path)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        #[cfg(feature = "openssl")]
        let cert = OpenSslX509::from_pem(&cert_data)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        // Get subject
        let subject = cert.subject_name();
        // Convert subject to string manually
        let subject_str = format!("{:?}", subject);

        Ok(subject_str)
    }

    fn get_cert_fingerprint(&self, cert_path: &Path) -> Result<String> {
        // Read certificate file
        let cert_data = crate::common::read_file(cert_path)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        #[cfg(feature = "openssl")]
        let cert = OpenSslX509::from_pem(&cert_data)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        // Calculate fingerprint
        let fingerprint = cert.digest(openssl::hash::MessageDigest::sha256())?;
        let fingerprint_hex = fingerprint.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join(":");

        Ok(fingerprint_hex)
    }

    fn load_cert(&self, cert_path: &Path) -> Result<X509> {
        // Implementation returns OpenSslX509 which is converted to X509
        // Read certificate file
        let cert_data = crate::common::read_file(cert_path)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        #[cfg(feature = "openssl")]
        let cert = OpenSslX509::from_pem(&cert_data)
            .map_err(|e| crate::common::ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        #[cfg(not(feature = "openssl"))]
        return Err(ProxyError::Certificate("OpenSSL support is not enabled".to_string()));

        Ok(cert)
    }
}
