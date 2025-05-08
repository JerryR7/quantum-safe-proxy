//! Test for configuration priority order
//!
//! This test verifies that the configuration priority order is correctly applied:
//! Command line arguments > Environment variables > Configuration file > Default values

use quantum_safe_proxy::config::{self, ProxyConfig};
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::tempdir;

#[test]
fn test_config_priority() {
    // Create a simple config file with minimal settings
    let config_file = "test_config.json";
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "log_level": "info",
        "buffer_size": 8192,
        "connection_timeout": 30,
        "strategy": "single"
    }"#;
    fs::write(config_file, config_content).expect("Failed to write config file");

    // Set environment variables with correct case
    env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "127.0.0.1:8444");
    env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:6001");
    env::set_var("QUANTUM_SAFE_PROXY_LOG_LEVEL", "debug");
    env::set_var("QUANTUM_SAFE_PROXY_BUFFER_SIZE", "16384");
    env::set_var("QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT", "60");
    env::set_var("QUANTUM_SAFE_PROXY_STRATEGY", "sigalgs");

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
        "--strategy".to_string(),
        "dynamic".to_string(),
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

    assert_eq!(proxy_config.log_level(), "trace");
    assert_eq!(proxy_config.buffer_size(), 32768);
    assert_eq!(proxy_config.connection_timeout(), 120);
    assert_eq!(proxy_config.strategy(), config::CertStrategyType::Dynamic);

    // Now test with only environment variables and config file
    // Use empty args to avoid command line arguments overriding environment variables
    let args: Vec<String> = vec![];

    // Print environment variables again to make sure they're still set
    println!("Environment variables before second test:");
    println!("QUANTUM_SAFE_PROXY_LISTEN={}", env::var("QUANTUM_SAFE_PROXY_LISTEN").unwrap());
    println!("QUANTUM_SAFE_PROXY_TARGET={}", env::var("QUANTUM_SAFE_PROXY_TARGET").unwrap());

    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .with_env("QUANTUM_SAFE_PROXY_")
        .without_validation()  // Disable validation to avoid file not found errors
        .build()
        .expect("Failed to build configuration");

    // Log the configuration for debugging
    println!("Configuration sources:");
    for (name, source) in &config.sources {
        println!("  {}: {:?}", name, source);
    }

    let proxy_config = ProxyConfig::from_config(config);

    // Verify environment variables have priority over config file
    let expected_listen = SocketAddr::from_str("127.0.0.1:8444").unwrap();
    let expected_target = SocketAddr::from_str("127.0.0.1:6001").unwrap();

    // Print the actual values for debugging
    println!("Actual listen: {:?}", proxy_config.as_config().values.listen);
    println!("Actual target: {:?}", proxy_config.as_config().values.target);

    // Check the raw values in config.values
    assert_eq!(proxy_config.as_config().values.listen, Some(expected_listen));
    assert_eq!(proxy_config.as_config().values.target, Some(expected_target));

    assert_eq!(proxy_config.log_level(), "debug");
    assert_eq!(proxy_config.buffer_size(), 16384);
    assert_eq!(proxy_config.connection_timeout(), 60);
    assert_eq!(proxy_config.strategy(), config::CertStrategyType::SigAlgs);

    // Clear environment variables
    env::remove_var("QUANTUM_SAFE_PROXY_LISTEN");
    env::remove_var("QUANTUM_SAFE_PROXY_TARGET");
    env::remove_var("QUANTUM_SAFE_PROXY_LOG_LEVEL");
    env::remove_var("QUANTUM_SAFE_PROXY_BUFFER_SIZE");
    env::remove_var("QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT");
    env::remove_var("QUANTUM_SAFE_PROXY_STRATEGY");

    // Test with only config file
    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
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
    assert_eq!(proxy_config.strategy(), config::CertStrategyType::Single);

    // Test with only defaults
    fs::remove_file(config_file).expect("Failed to remove config file");
    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .without_validation()  // Disable validation to avoid file not found errors
        .build()
        .expect("Failed to build configuration");

    let proxy_config = ProxyConfig::from_config(config);

    // Verify default values are used
    assert_eq!(proxy_config.log_level(), "info");
    assert_eq!(proxy_config.buffer_size(), 8192);
    assert_eq!(proxy_config.connection_timeout(), 30);

    // Print the actual strategy for debugging
    println!("Default strategy: {:?}", proxy_config.strategy());

    // Default strategy should be Dynamic
    assert_eq!(proxy_config.strategy(), config::CertStrategyType::Dynamic);
}
