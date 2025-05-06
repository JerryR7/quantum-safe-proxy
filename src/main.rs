//! Quantum Safe Proxy Command Line Interface


use quantum_safe_proxy::{
    StandardProxyService, ProxyService,
    create_tls_acceptor
};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config::ProxyConfig;
use quantum_safe_proxy::crypto::initialize_openssl;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration using a unified approach
    let config = ProxyConfig::load()?;

    // Initialize logger
    init_logger(&config.log_level);

    // Set OpenSSL directory if specified
    if let Some(openssl_dir) = &config.openssl_dir {
        std::env::set_var("OPENSSL_DIR", openssl_dir);
    }
    if let Some(openssl_dir) = config.openssl_dir.as_deref() {
        initialize_openssl(openssl_dir);
    }

    // Build certificate strategy and TLS acceptor
    let strategy = config.build_cert_strategy()?;
    let tls_acceptor = create_tls_acceptor(
        &config.client_ca_cert_path,
        &config.client_cert_mode,
        strategy,
    )?;

    // Start proxy service
    let proxy_service = StandardProxyService::new(
        config.listen,
        config.target,
        tls_acceptor,
        Arc::new(config),
    );
    let proxy_handle = proxy_service.start()?;

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    proxy_handle.shutdown().await?;

    Ok(())
}
