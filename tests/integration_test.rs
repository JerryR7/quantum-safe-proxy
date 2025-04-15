//! Integration tests
//!
//! This file contains integration tests for Quantum Safe Proxy.

use quantum_safe_proxy::config::ProxyConfig;
use quantum_safe_proxy::tls::{is_hybrid_cert, get_cert_subject, get_cert_fingerprint};
use quantum_safe_proxy::common::types::{ConnectionInfo, CertificateInfo};
use quantum_safe_proxy::common::{check_file_exists, read_file};
use std::path::PathBuf;
use std::time::SystemTime;

#[test]
fn test_config_creation() {
    // Test creating configuration
    let config = ProxyConfig::from_args(
        "127.0.0.1:8443",
        "127.0.0.1:6000",
        "certs/hybrid/dilithium3/server.crt",
        "certs/hybrid/dilithium3/server.key",
        "certs/hybrid/dilithium3/ca.crt",
        "info",
        "optional",
    );

    assert!(config.is_ok(), "Should be able to create configuration");
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
        timestamp: SystemTime::now(),
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

#[test]
fn test_utils() {
    // Test checking if file exists
    let path = PathBuf::from("Cargo.toml");
    let result = check_file_exists(&path);
    assert!(result.is_ok(), "Should be able to check existing file");

    // Test reading file
    let result = read_file(&path);
    assert!(result.is_ok(), "Should be able to read existing file");
}
