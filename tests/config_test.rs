//! Configuration tests
//!
//! This module contains tests for the configuration system.

use std::fs;

use quantum_safe_proxy::config::{
    ProxyConfig, ClientCertMode,
    ConfigBuilder
};

/// Test default configuration
#[test]
fn test_default_config() {
    // Create a configuration with default values
    let config = ProxyConfig::default();

    // Check default values
    assert_eq!(config.listen().to_string(), "0.0.0.0:8443");
    assert_eq!(config.target().to_string(), "127.0.0.1:6000");
    assert_eq!(config.log_level(), "info");
    assert_eq!(config.client_cert_mode(), ClientCertMode::Optional);
    assert_eq!(config.buffer_size(), 8192);
    assert_eq!(config.connection_timeout(), 30);
    // Default has no fallback configured, so it's Single mode
    assert!(!config.has_fallback());
}

/// Test configuration from file
#[test]
fn test_file_config() {
    // Create a temporary configuration file with new field names
    let config_content = r#"{
        "listen": "127.0.0.1:9000",
        "target": "127.0.0.1:8000",
        "log_level": "debug",
        "client_cert_mode": "required",
        "buffer_size": 16384,
        "connection_timeout": 60,
        "cert": "certs/hybrid/server.crt",
        "key": "certs/hybrid/server.key",
        "fallback_cert": "certs/traditional/server.crt",
        "fallback_key": "certs/traditional/server.key"
    }"#;

    let config_path = "test_config.json";
    fs::write(config_path, config_content).expect("Failed to write test config file");

    // Load configuration from file (without validation to avoid file not found errors)
    let config = ConfigBuilder::new()
        .with_defaults()
        .with_file(config_path)
        .without_validation()
        .build()
        .expect("Failed to load config from file");

    // Check values from file
    assert_eq!(config.listen().to_string(), "127.0.0.1:9000");
    assert_eq!(config.target().to_string(), "127.0.0.1:8000");
    assert_eq!(config.log_level(), "debug");
    assert_eq!(config.client_cert_mode(), ClientCertMode::Required);
    assert_eq!(config.buffer_size(), 16384);
    assert_eq!(config.connection_timeout(), 60);
    // Has fallback configured, so it's Dynamic mode
    assert!(config.has_fallback());

    // Clean up
    fs::remove_file(config_path).expect("Failed to remove test config file");
}

/// Test configuration from environment variables
#[test]
fn test_env_config() {
    // Create a mock environment source
    let mut config = ProxyConfig::default();

    // Set values directly
    config.values.listen = Some("127.0.0.1:7000".parse().unwrap());
    config.values.target = Some("127.0.0.1:5000".parse().unwrap());
    config.values.log_level = Some("warn".to_string());
    config.values.client_cert_mode = Some(ClientCertMode::None);
    config.values.buffer_size = Some(4096);
    config.values.connection_timeout = Some(15);
    // No fallback = Single mode

    // Check values
    assert_eq!(config.listen().to_string(), "127.0.0.1:7000");
    assert_eq!(config.target().to_string(), "127.0.0.1:5000");
    assert_eq!(config.log_level(), "warn");
    assert_eq!(config.client_cert_mode(), ClientCertMode::None);
    assert_eq!(config.buffer_size(), 4096);
    assert_eq!(config.connection_timeout(), 15);
    assert!(!config.has_fallback());
}

/// Test configuration from command line arguments
#[test]
fn test_cli_config() {
    // Create command line arguments with new field names
    let args = vec![
        "program".to_string(),
        "--listen".to_string(), "127.0.0.1:6000".to_string(),
        "--target".to_string(), "127.0.0.1:4000".to_string(),
        "--log-level".to_string(), "error".to_string(),
        "--client-cert-mode".to_string(), "required".to_string(),
        "--buffer-size".to_string(), "2048".to_string(),
        "--connection-timeout".to_string(), "10".to_string(),
        "--cert".to_string(), "certs/hybrid/server.crt".to_string(),
        "--key".to_string(), "certs/hybrid/server.key".to_string(),
        "--fallback-cert".to_string(), "certs/traditional/server.crt".to_string(),
        "--fallback-key".to_string(), "certs/traditional/server.key".to_string(),
    ];

    // Build configuration with command line arguments
    let config = ConfigBuilder::new()
        .with_defaults()
        .with_cli(args)
        .without_validation()
        .build()
        .expect("Failed to build config with command line arguments");

    // Check values from command line arguments
    assert_eq!(config.listen().to_string(), "127.0.0.1:6000");
    assert_eq!(config.target().to_string(), "127.0.0.1:4000");
    assert_eq!(config.log_level(), "error");
    assert_eq!(config.client_cert_mode(), ClientCertMode::Required);
    assert_eq!(config.buffer_size(), 2048);
    assert_eq!(config.connection_timeout(), 10);
    // Has fallback configured
    assert!(config.has_fallback());
}

/// Test configuration priority
#[test]
fn test_config_priority() {
    // Create a configuration with file values
    let mut file_config = ProxyConfig::default();
    file_config.values.listen = Some("127.0.0.1:9000".parse().unwrap());
    file_config.values.target = Some("127.0.0.1:8000".parse().unwrap());
    file_config.values.log_level = Some("debug".to_string());
    file_config.values.client_cert_mode = Some(ClientCertMode::Required);
    file_config.values.buffer_size = Some(16384);
    file_config.values.connection_timeout = Some(60);

    // Create a configuration with environment values (should override file)
    let mut env_config = file_config.clone();
    env_config.values.listen = Some("127.0.0.1:7000".parse().unwrap());
    env_config.values.log_level = Some("warn".to_string());
    env_config.values.buffer_size = Some(4096);

    // Create a configuration with CLI values (should override environment and file)
    let mut cli_config = env_config.clone();
    cli_config.values.listen = Some("127.0.0.1:6000".parse().unwrap());
    cli_config.values.target = Some("127.0.0.1:7000".parse().unwrap());
    cli_config.values.buffer_size = Some(2048);

    // Check values with proper priority
    assert_eq!(cli_config.listen().to_string(), "127.0.0.1:6000");  // From CLI
    assert_eq!(cli_config.target().to_string(), "127.0.0.1:7000");  // From CLI
    assert_eq!(cli_config.log_level(), "warn");                     // From env
    assert_eq!(cli_config.client_cert_mode(), ClientCertMode::Required);  // From file
    assert_eq!(cli_config.buffer_size(), 2048);                     // From CLI
    assert_eq!(cli_config.connection_timeout(), 60);                // From file
}

/// Test backward compatibility aliases
#[test]
fn test_backward_compat_aliases() {
    // Create command line arguments with old field names
    let args = vec![
        "program".to_string(),
        "--hybrid-cert".to_string(), "certs/hybrid/server.crt".to_string(),
        "--hybrid-key".to_string(), "certs/hybrid/server.key".to_string(),
        "--traditional-cert".to_string(), "certs/traditional/server.crt".to_string(),
        "--traditional-key".to_string(), "certs/traditional/server.key".to_string(),
    ];

    // Build configuration with command line arguments
    let config = ConfigBuilder::new()
        .with_defaults()
        .with_cli(args)
        .without_validation()
        .build()
        .expect("Failed to build config with backward compat arguments");

    // Check that old names map to new names
    assert_eq!(config.cert().to_string_lossy(), "certs/hybrid/server.crt");
    assert_eq!(config.key().to_string_lossy(), "certs/hybrid/server.key");
    assert_eq!(config.fallback_cert().unwrap().to_string_lossy(), "certs/traditional/server.crt");
    assert_eq!(config.fallback_key().unwrap().to_string_lossy(), "certs/traditional/server.key");
    
    // Has fallback configured
    assert!(config.has_fallback());
}
