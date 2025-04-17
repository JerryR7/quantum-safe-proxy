//! Hybrid certificates example
//!
//! This example demonstrates how to work with hybrid certificates in Quantum Safe Proxy.
//! It shows how to detect hybrid certificates and display their properties.

use quantum_safe_proxy::{Result};
use quantum_safe_proxy::tls::{is_hybrid_cert, get_cert_subject, get_cert_fingerprint, CertProviderType};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Hybrid Certificates Example");
    println!("==========================");

    // Path to certificate files
    let cert_paths = [
        "certs/hybrid/dilithium3/server.crt",
        "certs/hybrid/dilithium5/server.crt",
        "certs/traditional/rsa/server.crt",
        // Add more certificate paths as needed
    ];

    // Check each certificate
    for cert_path in cert_paths.iter() {
        let path = Path::new(cert_path);

        // Skip if the certificate doesn't exist
        if !path.exists() {
            println!("Certificate not found: {}", cert_path);
            continue;
        }

        println!("\nAnalyzing certificate: {}", cert_path);
        println!("---------------------------");

        // Check if it's a hybrid certificate
        match is_hybrid_cert(path, None) {
            Ok(is_hybrid) => {
                if is_hybrid {
                    println!("✅ This is a hybrid PQC certificate");
                } else {
                    println!("❌ This is a traditional certificate (not hybrid)");
                }
            },
            Err(e) => println!("Error checking certificate type: {}", e),
        }

        // Get certificate subject
        match get_cert_subject(path, None) {
            Ok(subject) => println!("Subject: {}", subject),
            Err(e) => println!("Error getting certificate subject: {}", e),
        }

        // Get certificate fingerprint
        match get_cert_fingerprint(path, None) {
            Ok(fingerprint) => println!("Fingerprint: {}", fingerprint),
            Err(e) => println!("Error getting certificate fingerprint: {}", e),
        }
    }

    println!("\nHybrid Certificate Detection Logic");
    println!("=================================");
    println!("Quantum Safe Proxy detects hybrid certificates by examining the signature algorithm.");
    println!("It looks for algorithms containing any of these strings:");
    println!("  - \"Kyber\" (CRYSTALS-Kyber key encapsulation mechanism)");
    println!("  - \"Dilithium\" (CRYSTALS-Dilithium signature scheme)");
    println!("  - \"oqs\" (Open Quantum Safe project identifier)");
    println!("  - \"hybrid\" (Generic hybrid indicator)");

    Ok(())
}
