//! Proxy configuration compatibility tests
//!
//! This module tests backward compatibility of the configuration system.

use std::path::PathBuf;
use std::net::SocketAddr;
use std::str::FromStr;

use quantum_safe_proxy::config::{ProxyConfig, ConfigBuilder};

/// Test backward compatibility with old field names
#[test]
fn test_backward_compat_json() {
    // Create a config file with OLD field names (backward compat)
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "log_level": "debug",
        "buffer_size": 16384,
        "connection_timeout": 60,
        "hybrid_cert": "certs/hybrid/dilithium3/server.crt",
        "hybrid_key": "certs/hybrid/dilithium3/server.key",
        "traditional_cert": "certs/traditional/rsa/server.crt",
        "traditional_key": "certs/traditional/rsa/server.key",
        "client_ca_cert_path": "certs/hybrid/dilithium3/ca.crt"
    }"#;

    let config_path = "test_compat_config.json";
    std::fs::write(config_path, config_content).expect("Failed to write test config file");

    // Load configuration
    let proxy_config = ConfigBuilder::new()
        .with_defaults()
        .with_file(config_path)
        .without_validation()
        .build()
        .expect("Failed to load config");

    // Test new accessor methods
    assert_eq!(proxy_config.listen(), SocketAddr::from_str("127.0.0.1:8443").unwrap());
    assert_eq!(proxy_config.target(), SocketAddr::from_str("127.0.0.1:6000").unwrap());
    assert_eq!(proxy_config.log_level(), "debug");
    assert_eq!(proxy_config.buffer_size(), 16384);
    assert_eq!(proxy_config.connection_timeout(), 60);

    // New names should work (mapped from aliases)
    assert_eq!(proxy_config.cert(), PathBuf::from("certs/hybrid/dilithium3/server.crt"));
    assert_eq!(proxy_config.key(), PathBuf::from("certs/hybrid/dilithium3/server.key"));
    assert_eq!(proxy_config.fallback_cert(), Some(PathBuf::from("certs/traditional/rsa/server.crt").as_path()));
    assert_eq!(proxy_config.fallback_key(), Some(PathBuf::from("certs/traditional/rsa/server.key").as_path()));
    assert_eq!(proxy_config.client_ca_cert(), PathBuf::from("certs/hybrid/dilithium3/ca.crt"));

    // Backward compat aliases should also work
    assert_eq!(proxy_config.hybrid_cert(), PathBuf::from("certs/hybrid/dilithium3/server.crt"));
    assert_eq!(proxy_config.hybrid_key(), PathBuf::from("certs/hybrid/dilithium3/server.key"));
    assert_eq!(proxy_config.client_ca_cert_path(), PathBuf::from("certs/hybrid/dilithium3/ca.crt"));

    // Has fallback = Dynamic mode
    assert!(proxy_config.has_fallback());

    // Clean up
    std::fs::remove_file(config_path).expect("Failed to remove test config file");
}

/// Test new field names
#[test]
fn test_new_field_names() {
    // Create a config file with NEW field names
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "log_level": "debug",
        "cert": "certs/hybrid/server.crt",
        "key": "certs/hybrid/server.key",
        "fallback_cert": "certs/traditional/server.crt",
        "fallback_key": "certs/traditional/server.key",
        "client_ca_cert": "certs/ca.crt"
    }"#;

    let config_path = "test_new_names_config.json";
    std::fs::write(config_path, config_content).expect("Failed to write test config file");

    // Load configuration
    let proxy_config = ConfigBuilder::new()
        .with_defaults()
        .with_file(config_path)
        .without_validation()
        .build()
        .expect("Failed to load config");

    // Check new field names work
    assert_eq!(proxy_config.cert(), PathBuf::from("certs/hybrid/server.crt"));
    assert_eq!(proxy_config.key(), PathBuf::from("certs/hybrid/server.key"));
    assert_eq!(proxy_config.fallback_cert(), Some(PathBuf::from("certs/traditional/server.crt").as_path()));
    assert_eq!(proxy_config.fallback_key(), Some(PathBuf::from("certs/traditional/server.key").as_path()));
    assert_eq!(proxy_config.client_ca_cert(), PathBuf::from("certs/ca.crt"));

    // Has fallback = Dynamic mode
    assert!(proxy_config.has_fallback());

    // Clean up
    std::fs::remove_file(config_path).expect("Failed to remove test config file");
}

/// Test the build_cert_strategy method
#[test]
fn test_build_cert_strategy() {
    // Create a config with fallback (Dynamic mode)
    let mut config = ProxyConfig::default();
    config.values.cert = Some("certs/hybrid/server.crt".into());
    config.values.key = Some("certs/hybrid/server.key".into());
    config.values.fallback_cert = Some("certs/traditional/server.crt".into());
    config.values.fallback_key = Some("certs/traditional/server.key".into());

    // Build strategy should succeed (returns Box<dyn Any>)
    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&config)
        .expect("Failed to build certificate strategy");
    
    // Should be a CertStrategy
    assert!(strategy.is::<quantum_safe_proxy::tls::strategy::CertStrategy>());
}

/// Test check_warnings function
#[test]
fn test_check_warnings() {
    let config = ProxyConfig::default();
    
    // check_warnings should return warnings for missing cert files
    let warnings = quantum_safe_proxy::check_warnings(&config);
    
    // Should have warnings since default cert paths don't exist
    println!("Warnings: {:?}", warnings);
    assert!(!warnings.is_empty());
}
