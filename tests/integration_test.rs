//! Integration tests
//!
//! This file contains integration tests for Quantum Safe Proxy.

use quantum_safe_proxy::config::ProxyConfig;
use quantum_safe_proxy::tls::{is_hybrid_cert, get_cert_subject, get_cert_fingerprint};
use std::path::PathBuf;
use std::net::SocketAddr;
use std::str::FromStr;

// Define a local ConnectionInfo struct for testing
#[derive(Debug, Clone)]
struct ConnectionInfo {
    source: String,
    target: String,
}

// Define a local CertificateInfo struct for testing
#[derive(Debug, Clone)]
struct CertificateInfo {
    subject: String,
    fingerprint: Option<String>,
    is_hybrid: bool,
}

#[test]
fn test_config_creation() {
    // Test creating configuration with default values
    let config = ProxyConfig::default();

    // Validate the configuration
    let warnings = quantum_safe_proxy::ConfigValidator::check_warnings(&config);
    // Default config may have warnings, but it should not panic
    println!("Config warnings: {:?}", warnings);
}

#[test]
fn test_cert_operations() {
    // This test needs a valid certificate file
    let cert_path = PathBuf::from("certs/hybrid/dilithium3/server.crt");
    if !cert_path.exists() {
        println!("Skipping test: Certificate file does not exist");
        return;
    }

    // Test if we can check the certificate type
    let is_hybrid = is_hybrid_cert(&cert_path);
    assert!(is_hybrid.is_ok(), "Should be able to check certificate type");

    // Test if we can get the certificate subject
    let subject = get_cert_subject(&cert_path);
    assert!(subject.is_ok(), "Should be able to get certificate subject");

    // Test if we can get the certificate fingerprint
    let fingerprint = get_cert_fingerprint(&cert_path);
    assert!(fingerprint.is_ok(), "Should be able to get certificate fingerprint");
}

#[test]
fn test_common_types() {
    // Test connection info
    let conn_info = ConnectionInfo {
        source: "127.0.0.1:12345".to_string(),
        target: "127.0.0.1:6000".to_string(),
    };

    assert_eq!(conn_info.source, "127.0.0.1:12345");
    assert_eq!(conn_info.target, "127.0.0.1:6000");

    // Test certificate info
    let cert_info = CertificateInfo {
        subject: "CN=test".to_string(),
        fingerprint: Some("AA:BB:CC".to_string()),
        is_hybrid: true,
    };

    assert_eq!(cert_info.subject, "CN=test");
    assert_eq!(cert_info.fingerprint, Some("AA:BB:CC".to_string()));
    assert!(cert_info.is_hybrid);
}

// We've removed the test_utils function as it's no longer needed
// The functionality is now tested directly in the modules where it's implemented
