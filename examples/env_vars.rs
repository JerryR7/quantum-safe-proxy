//! Environment variables example
//!
//! This example demonstrates how to use environment variables with Quantum Proxy.

use quantum_proxy::{Proxy, create_tls_acceptor, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    println!("Starting Quantum Proxy with environment variables...");
    
    // Set environment variables
    env::set_var("QUANTUM_PROXY_LISTEN", "0.0.0.0:9443");
    env::set_var("QUANTUM_PROXY_TARGET", "127.0.0.1:7000");
    env::set_var("QUANTUM_PROXY_CERT", "certs/server.crt");
    env::set_var("QUANTUM_PROXY_KEY", "certs/server.key");
    env::set_var("QUANTUM_PROXY_CA_CERT", "certs/ca.crt");
    env::set_var("QUANTUM_PROXY_LOG_LEVEL", "debug");
    env::set_var("QUANTUM_PROXY_HYBRID_MODE", "true");
    
    println!("Set environment variables:");
    println!("  QUANTUM_PROXY_LISTEN: {}", env::var("QUANTUM_PROXY_LISTEN").unwrap());
    println!("  QUANTUM_PROXY_TARGET: {}", env::var("QUANTUM_PROXY_TARGET").unwrap());
    println!("  QUANTUM_PROXY_CERT: {}", env::var("QUANTUM_PROXY_CERT").unwrap());
    
    // Load configuration from environment variables
    let config = quantum_proxy::config::ProxyConfig::from_env()?;
    println!("Loaded configuration:");
    println!("  Listen: {}", config.listen);
    println!("  Target: {}", config.target);
    println!("  Certificate: {:?}", config.cert_path);
    println!("  Hybrid mode: {}", config.hybrid_mode);
    
    // Create TLS acceptor
    let tls_acceptor = create_tls_acceptor(
        &config.cert_path,
        &config.key_path,
        &config.ca_cert_path,
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
    
    Ok(())
}
