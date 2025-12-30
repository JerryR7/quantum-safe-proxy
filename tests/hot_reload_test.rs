//! Test for hot reload functionality
//!
//! This test verifies that the configuration can be hot reloaded.

use quantum_safe_proxy::config::{self};
use std::fs;
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

#[test]
fn test_hot_reload() {
    // Create a simple config file with minimal settings (using new field names)
    let config_file = "test_hot_reload.json";
    let config_content = r#"{
        "listen": "127.0.0.1:8443",
        "target": "127.0.0.1:6000",
        "log_level": "info",
        "buffer_size": 8192,
        "connection_timeout": 30,
        "cert": "certs/server-pqc.crt",
        "key": "certs/server-pqc.key"
    }"#;
    fs::write(config_file, config_content).expect("Failed to write config file");

    // Initialize configuration
    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()  // Disable validation to avoid file not found errors
        .build()
        .expect("Failed to build configuration");

    // Initialize global configuration
    config::initialize(config.clone()).expect("Failed to initialize configuration");

    // Get the current configuration
    let current_config = config::get_config();

    // Verify initial values
    let expected_listen = SocketAddr::from_str("127.0.0.1:8443").unwrap();
    let expected_target = SocketAddr::from_str("127.0.0.1:6000").unwrap();

    assert_eq!(current_config.values.listen, Some(expected_listen));
    assert_eq!(current_config.values.target, Some(expected_target));
    assert_eq!(current_config.values.buffer_size, Some(8192));
    assert_eq!(current_config.values.connection_timeout, Some(30));
    // No fallback = Single mode
    assert!(!current_config.has_fallback());

    // Update the config file with new values (add fallback for Dynamic mode)
    let updated_config_content = r#"{
        "listen": "127.0.0.1:8444",
        "target": "127.0.0.1:6001",
        "log_level": "debug",
        "buffer_size": 16384,
        "connection_timeout": 60,
        "cert": "certs/server-pqc.crt",
        "key": "certs/server-pqc.key",
        "fallback_cert": "certs/server-pqc.crt",
        "fallback_key": "certs/server-pqc.key"
    }"#;
    fs::write(config_file, updated_config_content).expect("Failed to write updated config file");

    // Reload the configuration
    config::reload_config(config_file).expect("Failed to reload configuration");

    // Wait a moment for the reload to take effect
    thread::sleep(Duration::from_millis(100));

    // Get the updated configuration
    let updated_config = config::get_config();

    // Verify updated values
    let expected_listen = SocketAddr::from_str("127.0.0.1:8444").unwrap();
    let expected_target = SocketAddr::from_str("127.0.0.1:6001").unwrap();

    assert_eq!(updated_config.values.listen, Some(expected_listen));
    assert_eq!(updated_config.values.target, Some(expected_target));
    assert_eq!(updated_config.values.buffer_size, Some(16384));
    assert_eq!(updated_config.values.connection_timeout, Some(60));
    // Has fallback = Dynamic mode
    assert!(updated_config.has_fallback());

    // Clean up
    fs::remove_file(config_file).expect("Failed to remove config file");
}

#[test]
fn test_hot_reload_mode_change() {
    // Test that mode changes correctly during hot reload
    let config_file = "test_hot_reload_mode.json";

    // Start with Dynamic mode (has fallback)
    let config_content = r#"{
        "listen": "127.0.0.1:9443",
        "target": "127.0.0.1:7000",
        "cert": "certs/server-pqc-2.crt",
        "key": "certs/server-pqc-2.key",
        "fallback_cert": "certs/server-pqc.crt",
        "fallback_key": "certs/server-pqc.key"
    }"#;
    fs::write(config_file, config_content).expect("Failed to write config file");

    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    // Verify Dynamic mode
    assert!(config.has_fallback());

    // Change to Single mode (remove fallback)
    let updated_config_content = r#"{
        "listen": "127.0.0.1:9443",
        "target": "127.0.0.1:7000",
        "cert": "certs/server-pqc-2.crt",
        "key": "certs/server-pqc-2.key"
    }"#;
    fs::write(config_file, updated_config_content).expect("Failed to write updated config file");

    let updated_config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    // Verify Single mode
    assert!(!updated_config.has_fallback());

    // Clean up
    fs::remove_file(config_file).expect("Failed to remove config file");
}

#[test]
fn test_hot_reload_backward_compat() {
    // Test that old field names work during hot reload
    let config_file = "test_hot_reload_compat.json";

    // Use old field names
    let config_content = r#"{
        "listen": "127.0.0.1:9444",
        "target": "127.0.0.1:7001",
        "hybrid_cert": certs/server-pqc.crt",
        "hybrid_key": "certs/server-pqc.key",
        "traditional_cert": "certs/server-pqc.crt",
        "traditional_key": "certs/server-pqc.key"
    }"#;
    fs::write(config_file, config_content).expect("Failed to write config file");

    let config = config::builder::ConfigBuilder::new()
        .with_defaults()
        .with_file(config_file)
        .without_validation()
        .build()
        .expect("Failed to build configuration");

    // Old names should map to new accessors
    assert_eq!(config.cert().to_string_lossy(), "certs/hybrid/server.crt");
    assert_eq!(config.fallback_cert().unwrap().to_string_lossy(), "certs/traditional/server.crt");

    // Has fallback = Dynamic mode
    assert!(config.has_fallback());

    // Clean up
    fs::remove_file(config_file).expect("Failed to remove config file");
}