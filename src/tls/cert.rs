//! Certificate handling module
//!
//! This module provides functionality for working with X.509 certificates,
//! including hybrid post-quantum certificates.

use log::debug;
use openssl::x509::X509;
use openssl::hash::MessageDigest;
use std::path::Path;

use crate::common::{ProxyError, Result, read_file};

/// Check if a certificate is a hybrid certificate
///
/// Hybrid certificates combine traditional algorithms (like ECDSA or RSA) with
/// post-quantum algorithms (like Kyber, Dilithium, etc.) to provide security
/// against both classical and quantum computer attacks.
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
///
/// # Returns
///
/// Returns whether the certificate is a hybrid certificate
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn is_hybrid_cert(cert_path: &Path) -> Result<bool> {
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

/// Get certificate subject information
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
///
/// # Returns
///
/// Returns the certificate subject information as a string
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn get_cert_subject(cert_path: &Path) -> Result<String> {
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

/// Get certificate fingerprint
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
///
/// # Returns
///
/// Returns the SHA-256 fingerprint of the certificate
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn get_cert_fingerprint(cert_path: &Path) -> Result<String> {
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

/// Load certificate from PEM file
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
///
/// # Returns
///
/// Returns the loaded X509 certificate
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn load_cert(cert_path: &Path) -> Result<X509> {
    // Read certificate file
    let cert_data = read_file(cert_path)
        .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

    // Parse certificate
    let cert = X509::from_pem(&cert_data)
        .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

    Ok(cert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Note: These tests require valid certificate files to run
    // Before running the tests, make sure these files exist

    #[test]
    fn test_is_hybrid_cert() {
        // This test needs a valid certificate file
        // If there isn't one, we can skip this test
        let cert_path = PathBuf::from("certs/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can check the certificate type
        let result = is_hybrid_cert(&cert_path);
        assert!(result.is_ok(), "Should be able to check certificate type");
    }

    #[test]
    fn test_get_cert_subject() {
        // This test needs a valid certificate file
        let cert_path = PathBuf::from("certs/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can get the certificate subject
        let result = get_cert_subject(&cert_path);
        assert!(result.is_ok(), "Should be able to get certificate subject");

        // If successful, check that the subject is not empty
        if let Ok(subject) = result {
            assert!(!subject.is_empty(), "Certificate subject should not be empty");
            println!("Certificate subject: {}", subject);
        }
    }

    #[test]
    fn test_get_cert_fingerprint() {
        // This test needs a valid certificate file
        let cert_path = PathBuf::from("certs/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can get the certificate fingerprint
        let result = get_cert_fingerprint(&cert_path);
        assert!(result.is_ok(), "Should be able to get certificate fingerprint");

        // If successful, check that the fingerprint matches expected format
        if let Ok(fingerprint) = result {
            assert!(!fingerprint.is_empty(), "Certificate fingerprint should not be empty");
            assert!(fingerprint.contains(':'), "Certificate fingerprint should contain colon separators");
            println!("Certificate fingerprint: {}", fingerprint);
        }
    }

    #[test]
    fn test_load_cert() {
        // This test needs a valid certificate file
        let cert_path = PathBuf::from("certs/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can load the certificate
        let result = load_cert(&cert_path);
        assert!(result.is_ok(), "Should be able to load certificate");
    }
}
