//! Quantum Safe Proxy Command Line Interface
//!
//! This is the main entry point for the quantum-safe-proxy application.
//! It handles command line argument parsing, configuration loading, and
//! starting the proxy service.

use std::sync::Arc;
use log::{info, warn};
use tokio::signal;

use quantum_safe_proxy::{
    StandardProxyService, ProxyService,
    create_tls_acceptor, CertificateStrategyBuilder
};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config::{ProxyConfig, ConfigLoader, ConfigValidator};
use quantum_safe_proxy::crypto::initialize_openssl;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration with proper priority using auto_load
    // This handles: defaults -> config file -> env vars -> CLI args
    let config = ProxyConfig::auto_load()?;

    // 2. Initialize logger
    init_logger(&config.log_level);

    // 3. Log warnings and configuration
    let warnings = config.check();
    for warning in warnings {
        warn!("Configuration warning: {}", warning);
    }
    info!("Configuration loaded successfully");
    quantum_safe_proxy::config::log_config(&config);

    // 4. Set OpenSSL directory if specified
    if let Some(openssl_dir) = &config.openssl_dir {
        std::env::set_var("OPENSSL_DIR", openssl_dir);
        initialize_openssl(openssl_dir);
    }

    // 5. Build certificate strategy and TLS acceptor
    let strategy = config.build_cert_strategy()?;
    let tls_acceptor = create_tls_acceptor(
        &config.client_ca_cert_path,
        &config.client_cert_mode,
        strategy,
    )?;

    // 6. Start proxy service
    info!("Starting proxy service on {}", config.listen);
    let proxy_service = StandardProxyService::new(
        config.listen,
        config.target,
        tls_acceptor,
        Arc::new(config),
    );
    let proxy_handle = proxy_service.start()?;

    // 7. Wait for shutdown signal
    info!("Proxy service started. Press Ctrl+C to stop.");
    signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping proxy service...");
    proxy_handle.shutdown().await?;
    info!("Proxy service stopped.");

    Ok(())
}
