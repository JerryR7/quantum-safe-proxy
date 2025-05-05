//! Environment variables example
//!
//! This example demonstrates how to use environment variables with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result, CertificateStrategyBuilder};
use quantum_safe_proxy::config::ConfigLoader;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Starting Quantum Safe Proxy with environment variables...");

    // Set environment variables
    env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "0.0.0.0:9443");
    env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:7000");
    env::set_var("QUANTUM_SAFE_PROXY_LOG_LEVEL", "debug");
    env::set_var("QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE", "optional");

    // Set certificate strategy
    env::set_var("QUANTUM_SAFE_PROXY_STRATEGY", "dynamic");

    // Set traditional certificate
    env::set_var("QUANTUM_SAFE_PROXY_TRADITIONAL_CERT", "certs/traditional/rsa/server.crt");
    env::set_var("QUANTUM_SAFE_PROXY_TRADITIONAL_KEY", "certs/traditional/rsa/server.key");

    // Set hybrid certificate
    env::set_var("QUANTUM_SAFE_PROXY_HYBRID_CERT", "certs/hybrid/dilithium3/server.crt");
    env::set_var("QUANTUM_SAFE_PROXY_HYBRID_KEY", "certs/hybrid/dilithium3/server.key");

    // Set client CA certificate
    env::set_var("QUANTUM_SAFE_PROXY_CLIENT_CA_CERT", "certs/hybrid/dilithium3/ca.crt");

    println!("Set environment variables:");
    println!("  QUANTUM_SAFE_PROXY_LISTEN: {}", env::var("QUANTUM_SAFE_PROXY_LISTEN").unwrap());
    println!("  QUANTUM_SAFE_PROXY_TARGET: {}", env::var("QUANTUM_SAFE_PROXY_TARGET").unwrap());
    println!("  QUANTUM_SAFE_PROXY_STRATEGY: {}", env::var("QUANTUM_SAFE_PROXY_STRATEGY").unwrap());
    println!("  QUANTUM_SAFE_PROXY_TRADITIONAL_CERT: {}", env::var("QUANTUM_SAFE_PROXY_TRADITIONAL_CERT").unwrap());
    println!("  QUANTUM_SAFE_PROXY_HYBRID_CERT: {}", env::var("QUANTUM_SAFE_PROXY_HYBRID_CERT").unwrap());

    // Load configuration from environment variables
    let config = quantum_safe_proxy::config::ProxyConfig::from_env()?;
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

    // Create and start proxy
    let mut proxy = Proxy::new(
        config.listen,
        config.target,
        tls_acceptor,
        std::sync::Arc::new(config),
    );

    println!("Proxy service is ready, press Ctrl+C to stop");

    // Run proxy service
    proxy.run().await?;

    Ok(())
}
