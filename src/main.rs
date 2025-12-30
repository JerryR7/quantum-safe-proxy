//! Quantum Safe Proxy Command Line Interface
//!
//! This is the main entry point for the quantum-safe-proxy application.
//! It handles command line argument parsing, configuration loading, and
//! starting the proxy service.

use log::{info};
use tokio::signal;
use tokio::signal::unix::{signal, SignalKind};

use quantum_safe_proxy::{
    StandardProxyService, ProxyService,
    create_tls_acceptor
};
use quantum_safe_proxy::common::{Result, init_logger};
use quantum_safe_proxy::config::{self};
use quantum_safe_proxy::crypto::initialize_openssl;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration with proper priority
    // This handles: defaults -> config file -> env vars -> CLI args
    let args = std::env::args().collect::<Vec<String>>();
    let initial_config = config::builder::auto_load(args)?;

    // 2. Initialize logger
    init_logger(initial_config.log_level());

    // 3. Initialize global configuration
    config::initialize(initial_config)?;
    info!("Configuration loaded successfully");

    // 4. Get the global configuration
    let config = config::get_config();

    // 5. Set OpenSSL directory if specified
    if let Some(openssl_dir) = config.openssl_dir() {
        info!("Setting OpenSSL directory to: {}", openssl_dir.display());
        std::env::set_var("OPENSSL_DIR", openssl_dir.to_string_lossy().to_string());
        initialize_openssl(openssl_dir);
    }

    // 6. Build certificate strategy and TLS acceptor
    let cert_strategy = quantum_safe_proxy::tls::build_cert_strategy(&config)
        .and_then(|strategy| {
            strategy.downcast::<quantum_safe_proxy::tls::strategy::CertStrategy>()
                .map_err(|_| {
                    let err_msg = "Failed to downcast strategy to CertStrategy";
                    log::error!("{}", err_msg);
                    quantum_safe_proxy::common::ProxyError::Config(err_msg.to_string())
                })
                .map(|boxed| *boxed)
        })?;

    let tls_acceptor = create_tls_acceptor(
        config.client_ca_cert(),
        &config.client_cert_mode(),
        cert_strategy,
    )?;

    // 7. Start proxy service
    let listen_addr = config.listen();
    info!("Starting proxy service on {}", listen_addr);
    info!("Certificate mode: {}", if config.has_fallback() { "Dynamic" } else { "Single" });

    let proxy_service = StandardProxyService::new(
        listen_addr,
        config.target(),
        tls_acceptor,
        config.clone(),
    );
    let proxy_handle = proxy_service.start()?;

    // 8. Wait for shutdown or reload signal
    let mut sighup = signal(SignalKind::hangup())?;
    tokio::spawn(async move {
        while let Some(_) = sighup.recv().await {
            info!("Received SIGHUP signal, reloading configuration...");
            // Configuration reload logic would go here
        }
    });

    // Wait for Ctrl+C
    signal::ctrl_c().await?;
    info!("Received shutdown signal");

    // Shutdown gracefully
    proxy_handle.shutdown().await?;
    info!("Proxy service stopped");

    Ok(())
}
