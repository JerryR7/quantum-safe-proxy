//! Standard OpenSSL provider implementation
//!
//! This module implements the CryptoProvider trait using standard OpenSSL.
//! It does not support post-quantum cryptography but provides a fallback
//! when OQS-OpenSSL is not available.

use std::path::Path;
use openssl::x509::X509;
use openssl::hash::MessageDigest;
use log::debug;

use crate::common::{ProxyError, Result, read_file};
use super::{CryptoProvider, CryptoCapabilities};

/// Standard OpenSSL provider
///
/// This provider uses the standard OpenSSL library without post-quantum support.
/// It serves as a fallback when OQS-OpenSSL is not available.
#[derive(Debug, Default, Clone)]
pub struct StandardProvider;

impl StandardProvider {
    /// Create a new standard OpenSSL provider
    pub fn new() -> Self {
        Self
    }
}

impl CryptoProvider for StandardProvider {
    fn is_hybrid_cert(&self, cert_path: &Path) -> Result<bool> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object();

        // Get signature algorithm name
        let algorithm_name = signature_algorithm.to_string();
        debug!("Certificate signature algorithm: {}", algorithm_name);

        // Check if it's a hybrid certificate
        // Note: This detection logic should be adjusted based on the actual PQC algorithms in use
        // Currently we're simply checking if the algorithm name contains specific strings
        let is_hybrid = algorithm_name.contains("Kyber") ||
                       algorithm_name.contains("Dilithium") ||
                       algorithm_name.contains("oqs") ||
                       algorithm_name.contains("hybrid");

        Ok(is_hybrid)
    }

    fn get_cert_subject(&self, cert_path: &Path) -> Result<String> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        // Get subject
        let subject = cert.subject_name();
        // Convert X509NameRef to string
        let subject_str = format!("{:?}", subject);

        Ok(subject_str)
    }

    fn get_cert_fingerprint(&self, cert_path: &Path) -> Result<String> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        // Calculate SHA-256 fingerprint
        let fingerprint = cert.digest(MessageDigest::sha256())
            .map_err(|e| ProxyError::Certificate(format!("Failed to calculate certificate fingerprint: {}", e)))?;

        // Convert to hexadecimal string
        let fingerprint_hex = fingerprint.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join(":");

        Ok(fingerprint_hex)
    }

    fn load_cert(&self, cert_path: &Path) -> Result<X509> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        Ok(cert)
    }

    fn capabilities(&self) -> CryptoCapabilities {
        CryptoCapabilities {
            supports_pqc: false,
            supported_key_exchange: vec![
                "RSA".to_string(),
                "ECDHE".to_string(),
                "DHE".to_string(),
            ],
            supported_signatures: vec![
                "RSA".to_string(),
                "ECDSA".to_string(),
                "DSA".to_string(),
            ],
        }
    }

    fn name(&self) -> &'static str {
        "Standard OpenSSL"
    }
}
