//! Certificate handling module
//!
//! This module provides functionality for working with X.509 certificates,
//! including hybrid post-quantum certificates.

use log::{debug, info};
// Use X509 from crypto provider module
use crate::crypto::provider::X509;
use std::path::Path;

use crate::common::Result;
use crate::crypto::provider::{ProviderType, create_provider, is_openssl35_available};

/// Certificate provider type
///
/// This enum represents the type of certificate provider to use.
pub enum CertProviderType {
    /// OpenSSL 3.5 provider
    OpenSSL35,

    /// OQS-OpenSSL provider
    OQS,

    /// Auto-detect provider
    Auto,
}

/// Check if a certificate is a hybrid certificate
///
/// Hybrid certificates combine traditional algorithms (like ECDSA or RSA) with
/// post-quantum algorithms (like Kyber, Dilithium, etc.) to provide security
/// against both classical and quantum computer attacks.
///
/// This function uses the best available crypto provider to detect hybrid certificates.
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
/// * `provider_type` - Optional provider type to use (default: Auto)
///
/// # Returns
///
/// Returns whether the certificate is a hybrid certificate
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn is_hybrid_cert(cert_path: &Path, provider_type: Option<CertProviderType>) -> Result<bool> {
    // Determine provider type to use
    let provider_type = match provider_type {
        Some(CertProviderType::OpenSSL35) => ProviderType::Standard, // OpenSSL 3.5 is detected as Standard
        Some(CertProviderType::OQS) => ProviderType::Oqs,
        Some(CertProviderType::Auto) | None => ProviderType::Auto,
    };

    // Create a crypto provider
    let provider = create_provider(provider_type)?;

    // Use the provider to check if the certificate is hybrid
    let is_hybrid = provider.is_hybrid_cert(cert_path)?;

    // Log the provider used
    debug!("Checked certificate using {} provider", provider.name());

    // Log OpenSSL 3.5 availability
    if is_openssl35_available() {
        info!("Using OpenSSL 3.5+ for certificate operations");
    }

    Ok(is_hybrid)
}

/// Get certificate subject information
///
/// This function uses the best available crypto provider to get certificate subject information.
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
/// * `provider_type` - Optional provider type to use (default: Auto)
///
/// # Returns
///
/// Returns the certificate subject information as a string
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn get_cert_subject(cert_path: &Path, provider_type: Option<CertProviderType>) -> Result<String> {
    // Determine provider type to use
    let provider_type = match provider_type {
        Some(CertProviderType::OpenSSL35) => ProviderType::Standard, // OpenSSL 3.5 is detected as Standard
        Some(CertProviderType::OQS) => ProviderType::Oqs,
        Some(CertProviderType::Auto) | None => ProviderType::Auto,
    };

    // Create a crypto provider
    let provider = create_provider(provider_type)?;

    // Use the provider to get certificate subject
    let subject = provider.get_cert_subject(cert_path)?;

    Ok(subject)
}

/// Get certificate fingerprint
///
/// This function uses the best available crypto provider to get certificate fingerprint.
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
/// * `provider_type` - Optional provider type to use (default: Auto)
///
/// # Returns
///
/// Returns the SHA-256 fingerprint of the certificate
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn get_cert_fingerprint(cert_path: &Path, provider_type: Option<CertProviderType>) -> Result<String> {
    // Determine provider type to use
    let provider_type = match provider_type {
        Some(CertProviderType::OpenSSL35) => ProviderType::Standard, // OpenSSL 3.5 is detected as Standard
        Some(CertProviderType::OQS) => ProviderType::Oqs,
        Some(CertProviderType::Auto) | None => ProviderType::Auto,
    };

    // Create a crypto provider
    let provider = create_provider(provider_type)?;

    // Use the provider to get certificate fingerprint
    let fingerprint = provider.get_cert_fingerprint(cert_path)?;

    Ok(fingerprint)
}

/// Load certificate from PEM file
///
/// This function uses the best available crypto provider to load a certificate.
///
/// # Parameters
///
/// * `cert_path` - Path to the certificate file
/// * `provider_type` - Optional provider type to use (default: Auto)
///
/// # Returns
///
/// Returns the loaded X509 certificate
///
/// # Errors
///
/// Returns an error if the certificate cannot be read or parsed.
pub fn load_cert(cert_path: &Path, provider_type: Option<CertProviderType>) -> Result<X509> {
    // Determine provider type to use
    let provider_type = match provider_type {
        Some(CertProviderType::OpenSSL35) => ProviderType::Standard, // OpenSSL 3.5 is detected as Standard
        Some(CertProviderType::OQS) => ProviderType::Oqs,
        Some(CertProviderType::Auto) | None => ProviderType::Auto,
    };

    // Create a crypto provider
    let provider = create_provider(provider_type)?;

    // Use the provider to load certificate
    let cert = provider.load_cert(cert_path)?;

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
        let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can check the certificate type
        let result = is_hybrid_cert(&cert_path, None);
        assert!(result.is_ok(), "Should be able to check certificate type");

        // Test with explicit provider type
        if is_openssl35_available() {
            let result = is_hybrid_cert(&cert_path, Some(CertProviderType::OpenSSL35));
            assert!(result.is_ok(), "Should be able to check certificate type with OpenSSL 3.5");
        }
    }

    #[test]
    fn test_get_cert_subject() {
        // This test needs a valid certificate file
        let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can get the certificate subject
        let result = get_cert_subject(&cert_path, None);
        assert!(result.is_ok(), "Should be able to get certificate subject");

        // If successful, check that the subject is not empty
        if let Ok(subject) = result {
            assert!(!subject.is_empty(), "Certificate subject should not be empty");
            println!("Certificate subject: {}", subject);
        }

        // Test with explicit provider type
        if is_openssl35_available() {
            let result = get_cert_subject(&cert_path, Some(CertProviderType::OpenSSL35));
            assert!(result.is_ok(), "Should be able to get certificate subject with OpenSSL 3.5");
        }
    }

    #[test]
    fn test_get_cert_fingerprint() {
        // This test needs a valid certificate file
        let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can get the certificate fingerprint
        let result = get_cert_fingerprint(&cert_path, None);
        assert!(result.is_ok(), "Should be able to get certificate fingerprint");

        // If successful, check that the fingerprint matches expected format
        if let Ok(fingerprint) = result {
            assert!(!fingerprint.is_empty(), "Certificate fingerprint should not be empty");
            assert!(fingerprint.contains(':'), "Certificate fingerprint should contain colon separators");
            println!("Certificate fingerprint: {}", fingerprint);
        }

        // Test with explicit provider type
        if is_openssl35_available() {
            let result = get_cert_fingerprint(&cert_path, Some(CertProviderType::OpenSSL35));
            assert!(result.is_ok(), "Should be able to get certificate fingerprint with OpenSSL 3.5");
        }
    }

    #[test]
    fn test_load_cert() {
        // This test needs a valid certificate file
        let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can load the certificate
        let result = load_cert(&cert_path, None);
        assert!(result.is_ok(), "Should be able to load certificate");

        // Test with explicit provider type
        if is_openssl35_available() {
            let result = load_cert(&cert_path, Some(CertProviderType::OpenSSL35));
            assert!(result.is_ok(), "Should be able to load certificate with OpenSSL 3.5");
        }
    }
}
