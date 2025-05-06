//! Configuration file example
//!
//! This example demonstrates how to use a configuration file with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result, CertificateStrategyBuilder};
use quantum_safe_proxy::config::ConfigLoader;
use std::fs;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Starting Quantum Safe Proxy with configuration file...");

    // Create a configuration file
    let config_content = r#"{
        "listen": "0.0.0.0:8443",
        "target": "127.0.0.1:6000",
        "log_level": "debug",
        "client_cert_mode": "optional",
        "buffer_size": 8192,
        "connection_timeout": 30,

        "strategy": "dynamic",

        "traditional_cert": "certs/traditional/rsa/server.crt",
        "traditional_key": "certs/traditional/rsa/server.key",

        "hybrid_cert": "certs/hybrid/dilithium3/server.crt",
        "hybrid_key": "certs/hybrid/dilithium3/server.key",

        "client_ca_cert_path": "certs/hybrid/dilithium3/ca.crt"
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
    println!("  Strategy: {:?}", config.strategy);
    println!("  Traditional Certificate: {:?}", config.traditional_cert);
    println!("  Hybrid Certificate: {:?}", config.hybrid_cert);
    println!("  Buffer size: {} bytes", config.buffer_size);
    println!("  Connection timeout: {} seconds", config.connection_timeout);

    // Build certificate strategy
    let strategy = config.build_cert_strategy()?;

    // Create TLS acceptor with system-detected TLS settings
    let tls_acceptor = create_tls_acceptor(
        &config.client_ca_cert_path,
        &config.client_cert_mode,
        strategy,
    )?;

    // Parse listen and target addresses
    let listen_addr = config.listen;
    let target_addr = config.target;

    // Create and start proxy
    let mut proxy = Proxy::new(
        listen_addr,
        target_addr,
        tls_acceptor,
        std::sync::Arc::new(config),  // Wrap ProxyConfig in Arc
    );

    println!("Proxy service is ready, press Ctrl+C to stop");

    // Run proxy service
    proxy.run().await?;

    // Clean up
    fs::remove_file(config_path)?;

    Ok(())
}
