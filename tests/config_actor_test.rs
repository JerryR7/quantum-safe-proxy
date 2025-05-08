//! Configuration actor tests
//!
//! This module contains tests for the configuration actor system.

use std::fs;
use std::sync::Arc;

use quantum_safe_proxy::config::{
    ProxyConfig, ClientCertMode, CertStrategyType, ConfigActor
};

/// Test configuration actor
#[tokio::test]
async fn test_config_actor() {
    // Create a default configuration
    let initial_config = ProxyConfig::default();

    // Create a configuration actor
    let actor = ConfigActor::new(initial_config);

    // Get the current configuration
    let config = actor.get_config().await;

    // Check default values
    assert_eq!(config.listen().to_string(), "0.0.0.0:8443");
    assert_eq!(config.target().to_string(), "127.0.0.1:6000");
    assert_eq!(config.log_level(), "info");
    assert_eq!(config.client_cert_mode(), ClientCertMode::Optional);
    assert_eq!(config.buffer_size(), 8192);
    assert_eq!(config.connection_timeout(), 30);
    assert_eq!(config.strategy(), CertStrategyType::Dynamic);

    // Create a new configuration
    let mut new_config = ProxyConfig::default();
    new_config.values.listen = Some("127.0.0.1:9000".parse().unwrap());
    new_config.values.log_level = Some("debug".to_string());
    new_config.values.buffer_size = Some(16384);

    // Update the configuration
    actor.update_config(new_config).await.expect("Failed to update configuration");

    // Get the updated configuration
    let updated_config = actor.get_config().await;

    // Check updated values
    assert_eq!(updated_config.listen().to_string(), "127.0.0.1:9000");
    assert_eq!(updated_config.log_level(), "debug");
    assert_eq!(updated_config.buffer_size(), 16384);

    // Other values should remain unchanged
    assert_eq!(updated_config.target().to_string(), "127.0.0.1:6000");
    assert_eq!(updated_config.client_cert_mode(), ClientCertMode::Optional);
    assert_eq!(updated_config.connection_timeout(), 30);
    assert_eq!(updated_config.strategy(), CertStrategyType::Dynamic);

    // Shutdown the actor
    actor.shutdown().await;
}

/// Test configuration actor reload
#[tokio::test]
async fn test_config_actor_reload() {
    // Create a temporary configuration file
    let config_content = r#"{
        "listen": "127.0.0.1:9000",
        "target": "127.0.0.1:8000",
        "log_level": "debug",
        "client_cert_mode": "required",
        "buffer_size": 16384,
        "connection_timeout": 60,
        "strategy": "sigalgs"
    }"#;

    let config_path = "test_actor_reload.json";
    fs::write(config_path, config_content).expect("Failed to write test config file");

    // Create a default configuration
    let initial_config = ProxyConfig::default();

    // Create a configuration actor
    let actor = ConfigActor::new(initial_config);

    // Reload configuration from file
    let reloaded_config = actor.reload_config(config_path).await.expect("Failed to reload configuration");

    // Check reloaded values
    assert_eq!(reloaded_config.listen().to_string(), "127.0.0.1:9000");
    assert_eq!(reloaded_config.target().to_string(), "127.0.0.1:8000");
    assert_eq!(reloaded_config.log_level(), "debug");
    assert_eq!(reloaded_config.client_cert_mode(), ClientCertMode::Required);
    assert_eq!(reloaded_config.buffer_size(), 16384);
    assert_eq!(reloaded_config.connection_timeout(), 60);
    assert_eq!(reloaded_config.strategy(), CertStrategyType::SigAlgs);

    // Clean up
    fs::remove_file(config_path).expect("Failed to remove test config file");

    // Shutdown the actor
    actor.shutdown().await;
}

/// Test configuration actor with invalid configuration
#[tokio::test]
async fn test_config_actor_invalid() {
    // Create a default configuration
    let initial_config = ProxyConfig::default();

    // Create a configuration actor
    let actor = ConfigActor::new(initial_config);

    // Create an invalid configuration (listen and target are the same)
    let mut invalid_config = ProxyConfig::default();
    invalid_config.values.listen = Some("127.0.0.1:8000".parse().unwrap());
    invalid_config.values.target = Some("127.0.0.1:8000".parse().unwrap()); // This will cause validation to fail

    // Update should fail with validation error
    let result = actor.update_config(invalid_config).await;
    assert!(result.is_err());

    // Get the current configuration (should still be the initial configuration)
    let config = actor.get_config().await;

    // Check values (should be unchanged)
    assert_eq!(config.listen().to_string(), "0.0.0.0:8443");

    // Shutdown the actor
    actor.shutdown().await;
}

/// Test configuration actor with non-existent file
#[tokio::test]
async fn test_config_actor_missing_file() {
    // Create a default configuration
    let initial_config = ProxyConfig::default();

    // Create a configuration actor
    let actor = ConfigActor::new(initial_config);

    // Reload from non-existent file
    let result = actor.reload_config("non-existent-file.json").await;

    // Should not fail, but should log a warning and use default values
    assert!(result.is_ok());

    // Get the current configuration
    let config = actor.get_config().await;

    // Check values (should be unchanged)
    assert_eq!(config.listen().to_string(), "0.0.0.0:8443");

    // Shutdown the actor
    actor.shutdown().await;
}
