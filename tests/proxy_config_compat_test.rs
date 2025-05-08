//! Test for ProxyConfig compatibility
//!
//! This test verifies that ProxyConfig is compatible with the old API.

use quantum_safe_proxy::config::{self, ProxyConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

#[test]
fn test_proxy_config_compat() {
    // Create a ProxyConfig with default values
    let proxy_config = ProxyConfig::default();

    // Verify default values
    assert_eq!(proxy_config.log_level(), "info");
    assert_eq!(proxy_config.buffer_size(), 8192);
    assert_eq!(proxy_config.connection_timeout(), 30);
    assert_eq!(proxy_config.strategy(), config::CertStrategyType::Dynamic);

    // Create a ProxyConfig with custom values
    let mut config = ProxyConfig::default();
    config.values.listen = Some(SocketAddr::from_str("127.0.0.1:8443").unwrap());
    config.values.target = Some(SocketAddr::from_str("127.0.0.1:6000").unwrap());
    config.values.log_level = Some("debug".to_string());
    config.values.buffer_size = Some(16384);
    config.values.connection_timeout = Some(60);
    config.values.strategy = Some(config::CertStrategyType::SigAlgs);
    config.values.traditional_cert = Some(PathBuf::from("certs/traditional/rsa/server.crt"));
    config.values.traditional_key = Some(PathBuf::from("certs/traditional/rsa/server.key"));
    config.values.hybrid_cert = Some(PathBuf::from("certs/hybrid/dilithium3/server.crt"));
    config.values.hybrid_key = Some(PathBuf::from("certs/hybrid/dilithium3/server.key"));
    config.values.client_ca_cert_path = Some(PathBuf::from("certs/hybrid/dilithium3/ca.crt"));

    // Use the config directly
    let proxy_config = config;

    // Verify custom values
    assert_eq!(proxy_config.listen(), SocketAddr::from_str("127.0.0.1:8443").unwrap());
    assert_eq!(proxy_config.target(), SocketAddr::from_str("127.0.0.1:6000").unwrap());
    assert_eq!(proxy_config.log_level(), "debug");
    assert_eq!(proxy_config.buffer_size(), 16384);
    assert_eq!(proxy_config.connection_timeout(), 60);
    assert_eq!(proxy_config.strategy(), config::CertStrategyType::SigAlgs);
    assert_eq!(proxy_config.traditional_cert(), PathBuf::from("certs/traditional/rsa/server.crt"));
    assert_eq!(proxy_config.traditional_key(), PathBuf::from("certs/traditional/rsa/server.key"));
    assert_eq!(proxy_config.hybrid_cert(), PathBuf::from("certs/hybrid/dilithium3/server.crt"));
    assert_eq!(proxy_config.hybrid_key(), PathBuf::from("certs/hybrid/dilithium3/server.key"));
    assert_eq!(proxy_config.client_ca_cert_path(), PathBuf::from("certs/hybrid/dilithium3/ca.crt"));

    // Test direct access to values
    assert_eq!(proxy_config.values.listen, Some(SocketAddr::from_str("127.0.0.1:8443").unwrap()));
    assert_eq!(proxy_config.values.target, Some(SocketAddr::from_str("127.0.0.1:6000").unwrap()));
    assert_eq!(proxy_config.values.log_level, Some("debug".to_string()));
    assert_eq!(proxy_config.values.buffer_size, Some(16384));
    assert_eq!(proxy_config.values.connection_timeout, Some(60));
    assert_eq!(proxy_config.values.strategy, Some(config::CertStrategyType::SigAlgs));
    assert_eq!(proxy_config.values.traditional_cert, Some(PathBuf::from("certs/traditional/rsa/server.crt")));
    assert_eq!(proxy_config.values.traditional_key, Some(PathBuf::from("certs/traditional/rsa/server.key")));
    assert_eq!(proxy_config.values.hybrid_cert, Some(PathBuf::from("certs/hybrid/dilithium3/server.crt")));
    assert_eq!(proxy_config.values.hybrid_key, Some(PathBuf::from("certs/hybrid/dilithium3/server.key")));
    assert_eq!(proxy_config.values.client_ca_cert_path, Some(PathBuf::from("certs/hybrid/dilithium3/ca.crt")));

    // Test the check method
    let warnings = quantum_safe_proxy::check_warnings(&proxy_config);
    println!("Warnings: {:?}", warnings);

    // Test the build_cert_strategy method
    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&proxy_config).expect("Failed to build certificate strategy");
    assert!(strategy.is::<quantum_safe_proxy::tls::strategy::CertStrategy>());
}
