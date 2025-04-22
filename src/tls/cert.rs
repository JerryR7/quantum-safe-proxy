//! X.509 certificate handling with hybrid post-quantum support

use log::{debug, info};

use crate::crypto::X509;
use std::path::Path;

use crate::common::Result;
use crate::crypto::{get_provider, is_openssl35_available};



/// Check if a certificate combines traditional and post-quantum algorithms
pub fn is_hybrid_cert(cert_path: &Path) -> Result<bool> {
    // Get the global crypto provider
    let provider = get_provider();

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
pub fn get_cert_subject(cert_path: &Path) -> Result<String> {
    // Get the global crypto provider
    let provider = get_provider();

    // Use the provider to get a certificate subject
    let subject = provider.get_cert_subject(cert_path)?;

    Ok(subject)
}

/// Get certificate SHA-256 fingerprint
pub fn get_cert_fingerprint(cert_path: &Path) -> Result<String> {
    // Get the global crypto provider
    let provider = get_provider();

    // Use the provider to get certificate fingerprint
    let fingerprint = provider.get_cert_fingerprint(cert_path)?;

    Ok(fingerprint)
}

/// Load certificate from a PEM file
pub fn load_cert(cert_path: &Path) -> Result<X509> {
    // Get the global crypto provider
    let provider = get_provider();

    // Use the provider to load certificate
    let cert = provider.load_cert(cert_path)?;

    Ok(cert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Tests require valid certificate files

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
        let result = is_hybrid_cert(&cert_path);
        assert!(result.is_ok(), "Should be able to check certificate type");
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
        let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
        if !cert_path.exists() {
            println!("Skipping test: Certificate file does not exist");
            return;
        }

        // Test if we can get the certificate fingerprint
        let result = get_cert_fingerprint(&cert_path);
        assert!(result.is_ok(), "Should be able to get certificate fingerprint");

        // If successful, check that the fingerprint matches an expected format
        if let Ok(fingerprint) = result {
            assert!(!fingerprint.is_empty(), "Certificate fingerprint should not be empty");
            assert!(fingerprint.contains(':'), "Certificate fingerprint should contain colon separators");
            println!("Certificate fingerprint: {}", fingerprint);
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
        let result = load_cert(&cert_path);
        assert!(result.is_ok(), "Should be able to load certificate");
    }
}
