//! OQS-OpenSSL provider implementation
//!
//! This module implements the CryptoProvider trait using OQS-OpenSSL,
//! which supports post-quantum cryptography algorithms.

use std::path::Path;
use openssl::x509::X509;
use openssl::hash::MessageDigest;
use log::debug;

use crate::common::{ProxyError, Result, read_file};
use super::{CryptoProvider, CryptoCapabilities, CertificateType};

/// OQS-OpenSSL provider with post-quantum support
///
/// This provider uses the OQS-OpenSSL library with post-quantum support.
/// It can handle hybrid and pure post-quantum certificates.
#[derive(Debug, Default, Clone)]
pub struct OqsProvider;

impl OqsProvider {
    /// Create a new OQS-OpenSSL provider
    pub fn new() -> Self {
        Self
    }

    /// Extract post-quantum algorithms from certificate
    fn extract_pqc_algorithms(&self, cert: &X509) -> Vec<String> {
        let mut algorithms = Vec::new();

        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object().to_string();

        // Check for known PQC algorithms
        if signature_algorithm.contains("Kyber") {
            algorithms.push("Kyber".to_string());
        }

        if signature_algorithm.contains("Dilithium") {
            algorithms.push("Dilithium".to_string());
        }

        if signature_algorithm.contains("Falcon") {
            algorithms.push("Falcon".to_string());
        }

        if signature_algorithm.contains("SPHINCS") {
            algorithms.push("SPHINCS+".to_string());
        }

        // Try to extract more detailed information from extensions
        // This is a simplified approach; a real implementation would parse X.509 extensions

        algorithms
    }

    /// Determine certificate type
    fn determine_certificate_type(&self, cert: &X509) -> CertificateType {
        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object().to_string();
        debug!("Raw signature algorithm string: {}", signature_algorithm);

        // For p384_dilithium3 style hybrid certificates
        if signature_algorithm.contains("p256_") ||
           signature_algorithm.contains("p384_") ||
           signature_algorithm.contains("p521_") ||
           signature_algorithm.contains("_p256") ||
           signature_algorithm.contains("_p384") ||
           signature_algorithm.contains("_p521") {
            // This is definitely a hybrid certificate
            debug!("Detected hybrid certificate with signature algorithm: {}", signature_algorithm);
            return CertificateType::Hybrid;
        }

        // Check for known PQC algorithm indicators
        if signature_algorithm.contains("Kyber") ||
           signature_algorithm.contains("kyber") ||
           signature_algorithm.contains("Dilithium") ||
           signature_algorithm.contains("dilithium") ||
           signature_algorithm.contains("Falcon") ||
           signature_algorithm.contains("falcon") ||
           signature_algorithm.contains("SPHINCS") ||
           signature_algorithm.contains("sphincs") {
            // If it contains both classical and PQC algorithms, it's hybrid
            if signature_algorithm.contains("RSA") ||
               signature_algorithm.contains("rsa") ||
               signature_algorithm.contains("ECDSA") ||
               signature_algorithm.contains("ecdsa") ||
               signature_algorithm.contains("DSA") ||
               signature_algorithm.contains("dsa") ||
               signature_algorithm.contains("P-256") ||
               signature_algorithm.contains("P-384") ||
               signature_algorithm.contains("P-521") ||
               signature_algorithm.contains("prime256v1") ||
               signature_algorithm.contains("secp384r1") ||
               signature_algorithm.contains("secp521r1") {
                debug!("Detected hybrid certificate with classical and PQC algorithms: {}", signature_algorithm);
                CertificateType::Hybrid
            } else {
                debug!("Detected pure post-quantum certificate: {}", signature_algorithm);
                CertificateType::PurePostQuantum
            }
        } else if signature_algorithm.contains("oqs") ||
                  signature_algorithm.contains("OQS") ||
                  signature_algorithm.contains("hybrid") {
            // Generic indicators for hybrid certificates
            debug!("Detected hybrid certificate with generic indicators: {}", signature_algorithm);
            CertificateType::Hybrid
        } else {
            // No PQC indicators found
            debug!("Detected classical certificate: {}", signature_algorithm);
            CertificateType::Classical
        }
    }
}

impl CryptoProvider for OqsProvider {
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
        let cert_type = self.determine_certificate_type(&cert);

        Ok(cert_type == CertificateType::Hybrid)
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

        // Add PQC algorithm information if available
        let pqc_algorithms = self.extract_pqc_algorithms(&cert);
        if !pqc_algorithms.is_empty() {
            let pqc_info = format!(" (PQC: {})", pqc_algorithms.join(", "));
            return Ok(format!("{}{}", subject_str, pqc_info));
        }

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
            supports_pqc: true,
            supported_key_exchange: vec![
                "RSA".to_string(),
                "ECDHE".to_string(),
                "DHE".to_string(),
                "Kyber".to_string(),
            ],
            supported_signatures: vec![
                "RSA".to_string(),
                "ECDSA".to_string(),
                "DSA".to_string(),
                "Dilithium".to_string(),
                "Falcon".to_string(),
                "SPHINCS+".to_string(),
            ],
        }
    }

    fn name(&self) -> &'static str {
        "OQS-OpenSSL (Post-Quantum)"
    }
}
