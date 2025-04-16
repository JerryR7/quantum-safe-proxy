//! Configuration file example
//!
//! This example demonstrates how to use a configuration file with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Starting Quantum Safe Proxy with configuration file...");

    // Create a configuration file
    let config_content = r#"{
        "listen": "0.0.0.0:8443",
        "target": "127.0.0.1:6000",
        "cert_path": "certs/hybrid/dilithium3/server.crt",
        "key_path": "certs/hybrid/dilithium3/server.key",
        "ca_cert_path": "certs/hybrid/dilithium3/ca.crt",
        "hybrid_mode": true,
        "log_level": "debug"
    }"#;

    // Write the configuration to a temporary file
    let config_path = "config.json";
    fs::write(config_path, config_content)?;
    println!("Created configuration file: {}", config_path);

    // Load the configuration
    let config = quantum_safe_proxy::config::ProxyConfig::from_file(config_path)?;
    println!("Loaded configuration:");
    println!("  Listen: {}", config.listen);
    println!("  Target: {}", config.target);
    println!("  Certificate: {:?}", config.cert_path);
    println!("  Hybrid mode: {}", config.hybrid_mode);

    // Create TLS acceptor with system-detected TLS settings
    let tls_acceptor = create_tls_acceptor(
        &config.cert_path,
        &config.key_path,
        &config.ca_cert_path,
        &config.client_cert_mode,
    )?;

    // Create and start proxy
    let proxy = Proxy::new(
        config.listen,
        config.target,
        tls_acceptor,
    );

    println!("Proxy service is ready, press Ctrl+C to stop");

    // Run proxy service
    proxy.run().await?;

    // Clean up
    fs::remove_file(config_path)?;

    Ok(())
}
