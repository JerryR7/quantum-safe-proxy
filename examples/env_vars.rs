//! Environment variables example
//!
//! This example demonstrates how to use environment variables with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Starting Quantum Safe Proxy with environment variables...");

    // Set environment variables
    env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "0.0.0.0:9443");
    env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:7000");
    env::set_var("QUANTUM_SAFE_PROXY_CERT", "certs/hybrid/dilithium3/server.crt");
    env::set_var("QUANTUM_SAFE_PROXY_KEY", "certs/hybrid/dilithium3/server.key");
    env::set_var("QUANTUM_SAFE_PROXY_CA_CERT", "certs/hybrid/dilithium3/ca.crt");
    env::set_var("QUANTUM_SAFE_PROXY_LOG_LEVEL", "debug");
    env::set_var("QUANTUM_SAFE_PROXY_HYBRID_MODE", "true");

    println!("Set environment variables:");
    println!("  QUANTUM_SAFE_PROXY_LISTEN: {}", env::var("QUANTUM_SAFE_PROXY_LISTEN").unwrap());
    println!("  QUANTUM_SAFE_PROXY_TARGET: {}", env::var("QUANTUM_SAFE_PROXY_TARGET").unwrap());
    println!("  QUANTUM_SAFE_PROXY_CERT: {}", env::var("QUANTUM_SAFE_PROXY_CERT").unwrap());

    // Load configuration from environment variables
    let config = quantum_safe_proxy::config::ProxyConfig::from_env()?;
    println!("Loaded configuration:");
    println!("  Listen: {}", config.listen);
    println!("  Target: {}", config.target);
    println!("  Certificate: {:?}", config.cert_path);
    println!("  Buffer size: {} bytes", config.buffer_size);
    println!("  Connection timeout: {} seconds", config.connection_timeout);

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
        std::sync::Arc::new(config),  // 將 ProxyConfig 包裝在 Arc 中
    );

    println!("Proxy service is ready, press Ctrl+C to stop");

    // Run proxy service
    proxy.run().await?;

    Ok(())
}
