//! OpenSSL 3.5+ implementation with post-quantum cryptography capabilities

use std::path::Path;
use std::sync::OnceLock;
use log::{debug, info, warn};


use openssl::pkey::PKey;
use openssl::ssl::{SslMethod, SslVerifyMode, SslContext as OpenSslContext};
use openssl::x509::X509 as OpenSslX509;

use crate::common::{ProxyError, Result, read_file};
use super::{CryptoCapabilities, CertificateType, SslContext, X509};
use super::capabilities::{is_pqc_available, get_openssl_version, get_supported_pq_algorithms};
use super::capabilities::{get_recommended_cipher_list, get_recommended_tls13_ciphersuites, get_recommended_groups};

/// OpenSSL 3.5+ provider with post-quantum cryptography capabilities
#[derive(Debug, Clone)]
pub struct OpenSSLProvider {
    /// Whether post-quantum cryptography is supported
    supports_pqc: bool,

    /// OpenSSL version
    openssl_version: String,

    /// Supported post-quantum algorithms
    supported_pq_algorithms: Vec<String>,

    /// Recommended cipher list
    recommended_cipher_list: String,

    /// Recommended TLS 1.3 cipher suites
    recommended_tls13_ciphersuites: String,

    /// Recommended groups
    recommended_groups: String,
}

impl OpenSSLProvider {
    /// Get the global singleton instance, initialized on first access
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<OpenSSLProvider> = OnceLock::new();
        INSTANCE.get_or_init(|| OpenSSLProvider::new())
    }

    /// Create a new OpenSSL provider
    pub fn new() -> Self {
        // Check if post-quantum cryptography is supported
        let supports_pqc = is_pqc_available();

        // Get OpenSSL version
        let openssl_version = get_openssl_version();

        // Get supported post-quantum algorithms
        let supported_pq_algorithms = get_supported_pq_algorithms();

        // Get recommended cipher list, ciphersuites, and groups
        let recommended_cipher_list = get_recommended_cipher_list(supports_pqc);
        let recommended_tls13_ciphersuites = get_recommended_tls13_ciphersuites(supports_pqc);
        let recommended_groups = get_recommended_groups(supports_pqc);

        // Log provider information
        if supports_pqc {
            info!("Using OpenSSL {} with post-quantum support", openssl_version);
            debug!("Supported PQ algorithms: {:?}", supported_pq_algorithms);
        } else {
            warn!("Using OpenSSL {} without post-quantum support", openssl_version);
        }

        Self {
            supports_pqc,
            openssl_version,
            supported_pq_algorithms,
            recommended_cipher_list,
            recommended_tls13_ciphersuites,
            recommended_groups,
        }
    }

    /// Get the provider's name
    pub fn name(&self) -> &'static str {
        "OpenSSL 3.5+ Provider"
    }

    /// Get the provider's capabilities
    pub fn capabilities(&self) -> CryptoCapabilities {
        CryptoCapabilities {
            supports_pqc: self.supports_pqc,
            openssl_version: self.openssl_version.clone(),
            supported_pq_algorithms: self.supported_pq_algorithms.clone(),
            recommended_cipher_list: self.recommended_cipher_list.clone(),
            recommended_tls13_ciphersuites: self.recommended_tls13_ciphersuites.clone(),
            recommended_groups: self.recommended_groups.clone(),
        }
    }

    /// Create a TLS server context
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the server certificate
    /// * `key_path` - Path to the server private key
    /// * `ca_path` - Optional path to the CA certificate
    /// * `verify_client` - Whether to verify client certificates
    ///
    /// # Returns
    ///
    /// A TLS server context
    pub fn create_server_context(&self, cert_path: &Path, key_path: &Path, ca_path: Option<&Path>, verify_client: bool) -> Result<SslContext> {
        // Create a new SSL context for server
        let mut ctx = OpenSslContext::builder(SslMethod::tls_server())?;

        // Load certificate and private key
        let cert_data = read_file(cert_path)?;
        let key_data = read_file(key_path)?;

        let cert = OpenSslX509::from_pem(&cert_data)?;
        let key = PKey::private_key_from_pem(&key_data)?;

        // Set certificate and private key
        ctx.set_certificate(&cert)?;
        ctx.set_private_key(&key)?;
        ctx.check_private_key()?;

        // Determine a certificate type
        let cert_type = self.get_certificate_type(cert_path)?;

        // Set CA certificate if provided
        if let Some(ca_path) = ca_path {
            ctx.set_ca_file(ca_path)?;

            // Require client certificate verification if requested
            if verify_client {
                ctx.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
            }
        }

        // Set cipher list
        ctx.set_cipher_list(&self.recommended_cipher_list)?;

        // Set TLS 1.3 ciphersuites
        ctx.set_ciphersuites(&self.recommended_tls13_ciphersuites)?;

        // Set groups (curves)
        ctx.set_groups_list(&self.recommended_groups)?;

        // Set options
        ctx.set_options(
            openssl::ssl::SslOptions::NO_SSLV2 |
            openssl::ssl::SslOptions::NO_SSLV3 |
            openssl::ssl::SslOptions::NO_TLSV1 |
            openssl::ssl::SslOptions::NO_TLSV1_1 |
            openssl::ssl::SslOptions::NO_COMPRESSION
        );

        // Log certificate type
        match cert_type {
            CertificateType::Traditional => info!("Using traditional certificate"),
            CertificateType::Hybrid => info!("Using hybrid certificate (traditional + post-quantum)"),
            CertificateType::PostQuantum => info!("Using pure post-quantum certificate"),
        }

        Ok(ctx.build())
    }

    /// Create a TLS client context
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Optional path to the client certificate
    /// * `key_path` - Optional path to the client private key
    /// * `ca_path` - Optional path to the CA certificate
    ///
    /// # Returns
    ///
    /// A TLS client context
    pub fn create_client_context(&self, cert_path: Option<&Path>, key_path: Option<&Path>, ca_path: Option<&Path>) -> Result<SslContext> {
        // Create a new SSL context for the client
        let mut ctx = OpenSslContext::builder(SslMethod::tls_client())?;

        // Load certificate and private key if provided
        if let (Some(cert_path), Some(key_path)) = (cert_path, key_path) {
            let cert_data = read_file(cert_path)?;
            let key_data = read_file(key_path)?;

            let cert = OpenSslX509::from_pem(&cert_data)?;
            let key = PKey::private_key_from_pem(&key_data)?;

            ctx.set_certificate(&cert)?;
            ctx.set_private_key(&key)?;
            ctx.check_private_key()?;
        }

        // Set CA certificate if provided
        if let Some(ca_path) = ca_path {
            ctx.set_ca_file(ca_path)?;
        }

        // Set cipher list
        ctx.set_cipher_list(&self.recommended_cipher_list)?;

        // Set TLS 1.3 ciphersuites
        ctx.set_ciphersuites(&self.recommended_tls13_ciphersuites)?;

        // Set groups (curves)
        ctx.set_groups_list(&self.recommended_groups)?;

        // Set options
        ctx.set_options(
            openssl::ssl::SslOptions::NO_SSLV2 |
            openssl::ssl::SslOptions::NO_SSLV3 |
            openssl::ssl::SslOptions::NO_TLSV1 |
            openssl::ssl::SslOptions::NO_TLSV1_1 |
            openssl::ssl::SslOptions::NO_COMPRESSION
        );

        Ok(ctx.build())
    }

    /// Check if a certificate is a hybrid certificate
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// `true` if the certificate is a hybrid certificate, `false` otherwise
    pub fn is_hybrid_cert(&self, cert_path: &Path) -> Result<bool> {
        // Get a certificate type
        let cert_type = self.get_certificate_type(cert_path)?;

        // Return true if certificate is hybrid
        Ok(cert_type == CertificateType::Hybrid)
    }

    /// Get the certificate type
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The certificate type
    pub fn get_certificate_type(&self, cert_path: &Path) -> Result<CertificateType> {
        // Load certificate
        let cert = self.load_cert(cert_path)?;

        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object().to_string();
        debug!("Raw signature algorithm string: {}", signature_algorithm);

        // Check for hybrid certificate indicators in OpenSSL 3.5 format
        if signature_algorithm.contains("P256_ML-DSA") ||
           signature_algorithm.contains("P384_ML-DSA") ||
           signature_algorithm.contains("P521_ML-DSA") ||
           signature_algorithm.contains("RSA_ML-DSA") ||
           signature_algorithm.contains("P256_SLH-DSA") ||
           signature_algorithm.contains("P384_SLH-DSA") ||
           signature_algorithm.contains("P521_SLH-DSA") ||
           signature_algorithm.contains("RSA_SLH-DSA") {
            return Ok(CertificateType::Hybrid);
        }

        // Check for pure post-quantum certificate indicators
        if signature_algorithm.contains("ML-DSA") ||
           signature_algorithm.contains("SLH-DSA") {
            return Ok(CertificateType::PostQuantum);
        }

        // Default to traditional certificate
        Ok(CertificateType::Traditional)
    }

    /// Get certificate subject information
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The certificate subject information as a string
    pub fn get_cert_subject(&self, cert_path: &Path) -> Result<String> {
        // Load certificate
        let cert = self.load_cert(cert_path)?;

        // Get subject name
        let subject = cert.subject_name();

        // Convert subject to string
        let mut subject_string = String::new();
        for entry in subject.entries() {
            if !subject_string.is_empty() {
                subject_string.push_str(", ");
            }
            subject_string.push_str(&format!("{}={}", entry.object().nid().short_name()?, entry.data().as_utf8()?));
        }

        Ok(subject_string)
    }

    /// Get certificate fingerprint
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The certificate fingerprint as a string
    pub fn get_cert_fingerprint(&self, cert_path: &Path) -> Result<String> {
        // Load certificate
        let cert = self.load_cert(cert_path)?;

        // Get fingerprint
        let fingerprint = cert.digest(openssl::hash::MessageDigest::sha256())?;

        // Convert fingerprint to hex string
        let fingerprint_hex = fingerprint.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(":");

        Ok(fingerprint_hex)
    }

    /// Load certificate from the PEM file
    ///
    /// # Arguments
    ///
    /// * `cert_path` - Path to the certificate file
    ///
    /// # Returns
    ///
    /// The loaded X509 certificate
    pub fn load_cert(&self, cert_path: &Path) -> Result<X509> {
        // Read the certificate file
        let cert_data = read_file(cert_path)?;

        // Parse certificate
        let cert = OpenSslX509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        Ok(cert)
    }
}
