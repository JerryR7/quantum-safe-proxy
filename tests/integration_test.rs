//! Integration tests
//!
//! This file contains integration tests for Quantum Safe Proxy.

use quantum_safe_proxy::config::ProxyConfig;
use quantum_safe_proxy::tls::{is_hybrid_cert, get_cert_subject, get_cert_fingerprint};
use std::path::PathBuf;

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

    // Test auto-detection of certificate mode
    println!("Has fallback (Dynamic mode): {}", config.has_fallback());
}

#[test]
fn test_config_with_fallback() {
    // Test creating configuration with fallback (Dynamic mode)
    let mut config = ProxyConfig::default();
    config.values.cert = Some(PathBuf::from("certs/hybrid/server.crt"));
    config.values.key = Some(PathBuf::from("certs/hybrid/server.key"));
    config.values.fallback_cert = Some(PathBuf::from("certs/traditional/server.crt"));
    config.values.fallback_key = Some(PathBuf::from("certs/traditional/server.key"));

    // Should be in Dynamic mode
    assert!(config.has_fallback());

    // Test accessors
    assert_eq!(config.cert(), PathBuf::from("certs/hybrid/server.crt"));
    assert_eq!(config.fallback_cert(), Some(PathBuf::from("certs/traditional/server.crt").as_path()));
}

#[test]
fn test_config_without_fallback() {
    // Test creating configuration without fallback (Single mode)
    let mut config = ProxyConfig::default();
    config.values.cert = Some(PathBuf::from("certs/hybrid/server.crt"));
    config.values.key = Some(PathBuf::from("certs/hybrid/server.key"));

    // Should be in Single mode
    assert!(!config.has_fallback());

    // Fallback should be None
    assert!(config.fallback_cert().is_none());
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

#[test]
fn test_backward_compat_accessors() {
    // Test that backward compatibility accessors work
    let mut config = ProxyConfig::default();
    config.values.cert = Some(PathBuf::from("certs/hybrid/server.crt"));
    config.values.key = Some(PathBuf::from("certs/hybrid/server.key"));
    config.values.fallback_cert = Some(PathBuf::from("certs/traditional/server.crt"));
    config.values.fallback_key = Some(PathBuf::from("certs/traditional/server.key"));
    config.values.client_ca_cert = Some(PathBuf::from("certs/ca.crt"));

    // New accessors
    assert_eq!(config.cert(), PathBuf::from("certs/hybrid/server.crt"));
    assert_eq!(config.key(), PathBuf::from("certs/hybrid/server.key"));
    assert_eq!(config.client_ca_cert(), PathBuf::from("certs/ca.crt"));

    // Backward compat accessors
    assert_eq!(config.hybrid_cert(), PathBuf::from("certs/hybrid/server.crt"));
    assert_eq!(config.hybrid_key(), PathBuf::from("certs/hybrid/server.key"));
    assert_eq!(config.client_ca_cert_path(), PathBuf::from("certs/ca.crt"));
}

#[test]
fn test_build_cert_strategy() {
    // Test building certificate strategy from config
    let mut config = ProxyConfig::default();
    config.values.cert = Some(PathBuf::from("certs/hybrid/server.crt"));
    config.values.key = Some(PathBuf::from("certs/hybrid/server.key"));

    // Build strategy (Single mode since no fallback)
    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&config);
    assert!(strategy.is_ok());

    let strategy = strategy.unwrap();
    assert!(strategy.is::<quantum_safe_proxy::tls::strategy::CertStrategy>());
}

#[test]
fn test_build_dynamic_cert_strategy() {
    // Test building dynamic certificate strategy from config
    let mut config = ProxyConfig::default();
    config.values.cert = Some(PathBuf::from("certs/hybrid/server.crt"));
    config.values.key = Some(PathBuf::from("certs/hybrid/server.key"));
    config.values.fallback_cert = Some(PathBuf::from("certs/traditional/server.crt"));
    config.values.fallback_key = Some(PathBuf::from("certs/traditional/server.key"));

    // Build strategy (Dynamic mode since has fallback)
    assert!(config.has_fallback());

    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&config);
    assert!(strategy.is_ok());

    let strategy = strategy.unwrap();
    assert!(strategy.is::<quantum_safe_proxy::tls::strategy::CertStrategy>());
}