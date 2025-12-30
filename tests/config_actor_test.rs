//! Configuration actor tests
//!
//! This module contains tests for the configuration actor system.

use std::fs;

use quantum_safe_proxy::config::{
    ProxyConfig, ClientCertMode, ConfigActor
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
    // Default has no fallback
    assert!(!config.has_fallback());

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
    assert!(!updated_config.has_fallback());

    // Shutdown the actor
    actor.shutdown().await;
}

/// Test configuration actor reload
#[tokio::test]
async fn test_config_actor_reload() {
    // Create a temporary configuration file with new field names
    let config_content = r#"{
        "listen": "127.0.0.1:9000",
        "target": "127.0.0.1:8000",
        "log_level": "debug",
        "client_cert_mode": "required",
        "buffer_size": 16384,
        "connection_timeout": 60,
        "cert": "certs/server-pqc-2.crt",
        "key": "certs/server-pqc-2.key",
        "fallback_cert": "certs/server-pqc.crt",
        "fallback_key": "certs/server-pqc.key"
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
    // Has fallback configured
    assert!(reloaded_config.has_fallback());

    // Shutdown the actor
    actor.shutdown().await;

    // Clean up
    fs::remove_file(config_path).expect("Failed to remove test config file");
}

/// Test configuration actor with dynamic mode
#[tokio::test]
async fn test_config_actor_dynamic_mode() {
    // Create a configuration with fallback certs (Dynamic mode)
    let mut config = ProxyConfig::default();
    config.values.cert = Some("certs/server-pqc-2.crt".into());
    config.values.key = Some("certs/server-pqc-2.key".into());
    config.values.fallback_cert = Some("certs/server-pqc.crt".into());
    config.values.fallback_key = Some("certs/server-pqc.key".into());

    // Create a configuration actor
    let actor = ConfigActor::new(config);

    // Get the current configuration
    let config = actor.get_config().await;

    // Should be in dynamic mode
    assert!(config.has_fallback());

    // Shutdown the actor
    actor.shutdown().await;
}