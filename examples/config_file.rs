//! Configuration file example
//!
//! This example demonstrates how to use a configuration file with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use quantum_safe_proxy::config::ProxyConfig;
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
    println!("  Listen: {:?}", config.listen());
    println!("  Target: {:?}", config.target());
    println!("  Strategy: {:?}", config.strategy());
    println!("  Traditional Certificate: {:?}", config.traditional_cert());
    println!("  Hybrid Certificate: {:?}", config.hybrid_cert());
    println!("  Buffer size: {:?} bytes", config.buffer_size());
    println!("  Connection timeout: {:?} seconds", config.connection_timeout());

    // Build certificate strategy
    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&config)?;

    // Create TLS acceptor with system-detected TLS settings
    // Extract the CertStrategy from the Box<dyn Any>
    let cert_strategy = match strategy.downcast::<quantum_safe_proxy::tls::strategy::CertStrategy>() {
        Ok(cs) => *cs,  // Unbox it
        Err(_) => {
            let err_msg = "Failed to downcast strategy to CertStrategy";
            eprintln!("{}", err_msg);
            return Err(quantum_safe_proxy::common::ProxyError::Config(err_msg.to_string()));
        }
    };

    let tls_acceptor = create_tls_acceptor(
        config.client_ca_cert_path(),
        &config.client_cert_mode(),
        cert_strategy,
    )?;

    // Create and start proxy
    let listen_addr = if let Some(addr) = config.values.listen {
        addr
    } else {
        return Err(quantum_safe_proxy::common::ProxyError::Config("Listen address not set".to_string()));
    };

    let target_addr = if let Some(addr) = config.values.target {
        addr
    } else {
        return Err(quantum_safe_proxy::common::ProxyError::Config("Target address not set".to_string()));
    };

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
