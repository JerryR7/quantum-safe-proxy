//! Environment variables example
//!
//! This example demonstrates how to use environment variables with Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use quantum_safe_proxy::config::ProxyConfig;
use std::env;

/// Helper function to set environment variables
fn set_env_vars() {
    let env_vars = [
        ("QUANTUM_SAFE_PROXY_LISTEN", "0.0.0.0:9443"),
        ("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:7000"),
        ("QUANTUM_SAFE_PROXY_LOG_LEVEL", "debug"),
        ("QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE", "optional"),
        // Primary certificate (hybrid/PQC)
        ("QUANTUM_SAFE_PROXY_CERT", "certs/hybrid/dilithium3/server.crt"),
        ("QUANTUM_SAFE_PROXY_KEY", "certs/hybrid/dilithium3/server.key"),
        // Fallback certificate (traditional) - enables dynamic mode
        ("QUANTUM_SAFE_PROXY_FALLBACK_CERT", "certs/traditional/rsa/server.crt"),
        ("QUANTUM_SAFE_PROXY_FALLBACK_KEY", "certs/traditional/rsa/server.key"),
        // Client CA certificate
        ("QUANTUM_SAFE_PROXY_CLIENT_CA_CERT", "certs/hybrid/dilithium3/ca.crt"),
    ];

    for (key, value) in env_vars {
        env::set_var(key, value);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Starting Quantum Safe Proxy with environment variables...");

    // Set environment variables
    set_env_vars();

    // Load configuration from environment variables and defaults
    let config = ProxyConfig::auto_load()?;
    
    println!("Loaded configuration:");
    println!("  Listen: {:?}", config.listen());
    println!("  Target: {:?}", config.target());
    println!("  Mode: {}", if config.has_fallback() { "Dynamic" } else { "Single" });
    println!("  Primary Certificate: {:?}", config.cert());
    println!("  Fallback Certificate: {:?}", config.fallback_cert());
    println!("  Buffer size: {:?} bytes", config.buffer_size());
    println!("  Connection timeout: {:?} seconds", config.connection_timeout());

    // Build certificate strategy (auto-detected based on config)
    let strategy = quantum_safe_proxy::tls::build_cert_strategy(&config)?;

    // Extract the CertStrategy from the Box<dyn Any>
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

    // Get addresses
    let listen_addr = config.listen();
    let target_addr = config.target();

    // Create and start proxy
    let mut proxy = Proxy::new(
        listen_addr,
        target_addr,
        tls_acceptor,
        std::sync::Arc::new(config),
    );

    println!("Proxy service is ready, press Ctrl+C to stop");

    proxy.run().await?;

    Ok(())
}
