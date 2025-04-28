//! Simple proxy example
//!
//! This example demonstrates how to create a simple proxy using Quantum Safe Proxy.

use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result, parse_socket_addr};
use quantum_safe_proxy::tls::strategy::CertStrategy;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Starting simple proxy example...");

    // Create certificate strategy
    let strategy = CertStrategy::Single {
        cert: Path::new("certs/hybrid/dilithium3/server.crt").to_path_buf(),
        key: Path::new("certs/hybrid/dilithium3/server.key").to_path_buf(),
    };

    // Create TLS acceptor with system-detected TLS settings
    let tls_acceptor = create_tls_acceptor(
        Path::new("certs/hybrid/dilithium3/ca.crt"),
        &quantum_safe_proxy::config::ClientCertMode::Optional,
        strategy,
    )?;

    // Create and start proxy
    let listen_addr = parse_socket_addr("0.0.0.0:8443")?;
    let target_addr = parse_socket_addr("127.0.0.1:6000")?;

    // Create default config and wrap in Arc
    let config = std::sync::Arc::new(quantum_safe_proxy::config::ProxyConfig::default());

    let proxy = Proxy::new(
        listen_addr,
        target_addr,
        tls_acceptor,
        config,  // Use Arc<ProxyConfig>
    );

    println!("Proxy service started, press Ctrl+C to stop");

    // Run proxy service
    proxy.run().await?;

    Ok(())
}
