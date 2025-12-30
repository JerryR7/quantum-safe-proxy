//! Configuration file example
//!
//! This example demonstrates how to use a configuration file with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Starting Quantum Safe Proxy with configuration file...");

    // Create a configuration file with new field names
    let config_content = r#"{
        "listen": "0.0.0.0:8443",
        "target": "127.0.0.1:6000",
        "log_level": "debug",
        "client_cert_mode": "optional",
        "buffer_size": 8192,
        "connection_timeout": 30,

        "cert": "certs/hybrid/dilithium3/server.crt",
        "key": "certs/hybrid/dilithium3/server.key",

        "fallback_cert": "certs/traditional/rsa/server.crt",
        "fallback_key": "certs/traditional/rsa/server.key",

        "client_ca_cert": "certs/hybrid/dilithium3/ca.crt"
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
    println!("  Certificate mode: {}", if config.has_fallback() { "Dynamic" } else { "Single" });
    println!("  Primary Certificate: {:?}", config.cert());
    println!("  Fallback Certificate: {:?}", config.fallback_cert());
    println!("  Buffer size: {:?} bytes", config.buffer_size());
    println!("  Connection timeout: {:?} seconds", config.connection_timeout());

    // Build certificate strategy (auto-detected from config)
    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&config)?;

    // Create TLS acceptor
    let cert_strategy = match strategy.downcast::<quantum_safe_proxy::tls::strategy::CertStrategy>() {
        Ok(cs) => *cs,
        Err(_) => {
            let err_msg = "Failed to downcast strategy to CertStrategy";
            eprintln!("{}", err_msg);
            return Err(quantum_safe_proxy::common::ProxyError::Config(err_msg.to_string()));
        }
    };

    let tls_acceptor = create_tls_acceptor(
        config.client_ca_cert(),
        &config.client_cert_mode(),
        cert_strategy,
    )?;

    // Create and start the proxy
    let mut proxy = Proxy::new(
        config.listen(),
        config.target(),
        tls_acceptor,
        Arc::new(config),
    );

    println!("Proxy started successfully");
    println!("Press Ctrl+C to stop");

    // Run the proxy
    proxy.run().await?;

    // Clean up
    fs::remove_file(config_path)?;

    Ok(())
}
