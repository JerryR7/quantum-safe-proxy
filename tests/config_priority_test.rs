//! Test for configuration priority order
//!
//! This test verifies that the configuration priority order is correctly applied:
//! Command line arguments > Environment variables > Configuration file > Default values

use quantum_safe_proxy::config::{self, ProxyConfig};
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::str::FromStr;

#[test]
fn test_config_priority() {
    // Create a simple config file with minimal settings (using new field names)
    let config_file = "test_priority_config.json";
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "log_level": "info",
        "buffer_size": 8192,
        "connection_timeout": 30
    }"#;
    fs::write(config_file, config_content).expect("Failed to write config file");

    // Set environment variables with correct case
    env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "127.0.0.1:8444");
    env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:6001");
    env::set_var("QUANTUM_SAFE_PROXY_LOG_LEVEL", "debug");
    env::set_var("QUANTUM_SAFE_PROXY_BUFFER_SIZE", "16384");
    env::set_var("QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT", "60");

    // Print environment variables for debugging
    println!("Environment variables set:");
    println!("QUANTUM_SAFE_PROXY_LISTEN={}", env::var("QUANTUM_SAFE_PROXY_LISTEN").unwrap());
    println!("QUANTUM_SAFE_PROXY_TARGET={}", env::var("QUANTUM_SAFE_PROXY_TARGET").unwrap());

    // Create command line arguments
    let args = vec![
        "quantum-safe-proxy".to_string(),
        "--listen".to_string(),
        "127.0.0.1:8445".to_string(),
        "--target".to_string(),
        "127.0.0.1:6002".to_string(),
        "--log-level".to_string(),
        "trace".to_string(),
        "--buffer-size".to_string(),
        "32768".to_string(),
        "--connection-timeout".to_string(),
        "120".to_string(),
        "--config-file".to_string(),
        config_file.to_string(),
    ];

    // Load configuration with all sources
    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .with_env("QUANTUM_SAFE_PROXY_")
        .with_cli(args)
        .without_validation()  // Disable validation to avoid file not found errors
        .build()
        .expect("Failed to build configuration");

    let proxy_config = ProxyConfig::from_config(config);

    // Verify command line arguments have highest priority
    let expected_listen = SocketAddr::from_str("127.0.0.1:8445").unwrap();
    let expected_target = SocketAddr::from_str("127.0.0.1:6002").unwrap();

    // Print the actual values for debugging
    println!("Actual listen: {:?}", proxy_config.as_config().values.listen);
    println!("Actual target: {:?}", proxy_config.as_config().values.target);

    // Check the raw values in config.values
    assert_eq!(proxy_config.as_config().values.listen, Some(expected_listen));
    assert_eq!(proxy_config.as_config().values.target, Some(expected_target));

    // CLI should override env
    assert_eq!(proxy_config.log_level(), "trace");
    assert_eq!(proxy_config.buffer_size(), 32768);
    assert_eq!(proxy_config.connection_timeout(), 120);

    // Clear environment variables
    env::remove_var("QUANTUM_SAFE_PROXY_LISTEN");
    env::remove_var("QUANTUM_SAFE_PROXY_TARGET");
    env::remove_var("QUANTUM_SAFE_PROXY_LOG_LEVEL");
    env::remove_var("QUANTUM_SAFE_PROXY_BUFFER_SIZE");
    env::remove_var("QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT");

    // Test with only config file
    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    let proxy_config = ProxyConfig::from_config(config);

    // Verify config file values are used
    let expected_listen = SocketAddr::from_str("127.0.0.1:8443").unwrap();
    let expected_target = SocketAddr::from_str("127.0.0.1:6000").unwrap();

    // Print the actual values for debugging
    println!("Actual listen: {:?}", proxy_config.as_config().values.listen);
    println!("Actual target: {:?}", proxy_config.as_config().values.target);

    // Check the raw values in config.values
    assert_eq!(proxy_config.as_config().values.listen, Some(expected_listen));
    assert_eq!(proxy_config.as_config().values.target, Some(expected_target));

    assert_eq!(proxy_config.log_level(), "info");
    assert_eq!(proxy_config.buffer_size(), 8192);
    assert_eq!(proxy_config.connection_timeout(), 30);

    // Test with only defaults
    fs::remove_file(config_file).expect("Failed to remove config file");
    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    let proxy_config = ProxyConfig::from_config(config);

    // Verify default values are used
    assert_eq!(proxy_config.log_level(), "info");
    assert_eq!(proxy_config.buffer_size(), 8192);
    assert_eq!(proxy_config.connection_timeout(), 30);

    // Print the actual certificate mode for debugging
    println!("Has fallback: {}", proxy_config.has_fallback());

    // Default should not have fallback (Single mode)
    assert!(!proxy_config.has_fallback());
}

#[test]
fn test_dynamic_mode_detection() {
    // Test that dynamic mode is auto-detected when fallback certs are configured
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "cert": "certs/hybrid/server.crt",
        "key": "certs/hybrid/server.key",
        "fallback_cert": "certs/traditional/server.crt",
        "fallback_key": "certs/traditional/server.key"
    }"#;

    let config_file = "test_dynamic_mode.json";
    fs::write(config_file, config_content).expect("Failed to write config file");

    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    // Should detect Dynamic mode
    assert!(config.has_fallback());

    fs::remove_file(config_file).expect("Failed to remove config file");
}

#[test]
fn test_single_mode_detection() {
    // Test that single mode is auto-detected when only primary cert is configured
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "cert": "certs/hybrid/server.crt",
        "key": "certs/hybrid/server.key"
    }"#;

    let config_file = "test_single_mode.json";
    fs::write(config_file, config_content).expect("Failed to write config file");

    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    // Should detect Single mode (no fallback)
    assert!(!config.has_fallback());

    fs::remove_file(config_file).expect("Failed to remove config file");
}
