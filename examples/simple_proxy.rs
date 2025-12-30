//! Simple proxy example
//!
//! This example demonstrates how to create a simple proxy using Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
use quantum_safe_proxy::config::parse_socket_addr;
use quantum_safe_proxy::tls::strategy::CertStrategy;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Starting simple proxy example...");

    // Create certificate strategy
    // Dynamic mode: auto-selects based on client PQC support
    let strategy = CertStrategy::Dynamic {
        primary: (
            Path::new("certs/hybrid/dilithium3/server.crt").to_path_buf(),
            Path::new("certs/hybrid/dilithium3/server.key").to_path_buf(),
        ),
        fallback: (
            Path::new("certs/traditional/rsa/server.crt").to_path_buf(),
            Path::new("certs/traditional/rsa/server.key").to_path_buf(),
        ),
    };

    // Create TLS acceptor
    let tls_acceptor = create_tls_acceptor(
        Path::new("certs/hybrid/dilithium3/ca.crt"),
        &quantum_safe_proxy::config::ClientCertMode::Optional,
        strategy,
    )?;

    // Create and start proxy
    let listen_addr = parse_socket_addr("0.0.0.0:8443")?;
    let target_addr = parse_socket_addr("127.0.0.1:6000")?;

    let config = std::sync::Arc::new(quantum_safe_proxy::config::ProxyConfig::default());

    let mut proxy = Proxy::new(
        listen_addr,
        target_addr,
        tls_acceptor,
        config,
    );

    println!("Proxy service started, press Ctrl+C to stop");

    proxy.run().await?;

    Ok(())
}
