//! Quantum Safe Proxy Library
//!
//! This library provides a TLS proxy with post-quantum cryptography support.
//! It automatically detects client capabilities and selects appropriate certificates.
//!
//! # Certificate Strategy
//!
//! The certificate strategy is automatically determined based on configuration:
//! - **Single mode**: Only primary certificate (`cert`/`key`) configured
//! - **Dynamic mode**: Both primary and fallback certificates configured
//!
//! # Example
//!
//! ```no_run
//! use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
//! use quantum_safe_proxy::config::ProxyConfig;
//! use quantum_safe_proxy::tls::strategy::CertStrategy;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let config = ProxyConfig::auto_load()?;
//!     
//!     // Strategy is auto-detected based on config
//!     let strategy = CertStrategy::from(&config);
//!     
//!     let tls_acceptor = create_tls_acceptor(
//!         config.client_ca_cert(),
//!         &config.client_cert_mode(),
//!         strategy,
//!     )?;
//!
//!     let mut proxy = Proxy::new(
//!         config.listen(),
//!         config.target(),
//!         tls_acceptor,
//!         Arc::new(config),
//!     );
//!
//!     proxy.run().await
//! }
//! ```

// Public modules
pub mod common;
pub mod config;
pub mod crypto;
pub mod tls;
pub mod protocol;
pub mod proxy;

// Re-exports for convenience
pub use common::{Result, ProxyError};
pub use config::{ProxyConfig, ClientCertMode};
pub use proxy::{Proxy, StandardProxyService, ProxyService, ProxyHandle};
pub use tls::create_tls_acceptor;

// Re-export validator trait
pub use config::validator::ConfigValidator;

/// Check configuration for warnings
///
/// This is a convenience function that checks configuration for potential issues.
pub fn check_warnings(config: &ProxyConfig) -> Vec<String> {
    ConfigValidator::check_warnings(config)
}

/// Reload proxy configuration from file (async version)
///
/// This function reloads the proxy configuration from the specified file and
/// sends an update message to the proxy service.
///
/// # Parameters
///
/// * `proxy_handle` - Proxy handle for controlling the proxy service
/// * `config_path` - Path to the configuration file
///
/// # Returns
///
/// Returns the updated configuration if successful, otherwise returns an error.
pub async fn reload_config_async(
    proxy_handle: &ProxyHandle,
    config_path: &std::path::Path,
) -> Result<std::sync::Arc<config::ProxyConfig>> {
    use log::info;
    use std::sync::Arc;

    info!("Reloading configuration from {}", config_path.display());

    // Reload configuration from file
    let loaded_config = match config::ProxyConfig::from_file(config_path.to_str().unwrap_or("config.json")) {
        Ok(config) => {
            info!("Configuration reloaded successfully from file");
            info!("Certificate mode: {}", if config.has_fallback() { "Dynamic" } else { "Single" });
            Arc::new(config)
        },
        Err(e) => {
            let err_msg = format!("Failed to reload configuration from file: {}", e);
            log::error!("{}", err_msg);
            return Err(e.into());
        }
    };

    // Build certificate strategy (auto-detected)
    let strategy = match tls::build_cert_strategy(&loaded_config) {
        Ok(s) => {
            info!("Built certificate strategy successfully");
            s
        },
        Err(e) => {
            let err_msg = format!("Failed to build certificate strategy: {}", e);
            log::error!("{}", err_msg);
            return Err(e.into());
        }
    };

    // Create new TLS acceptor
    let tls_acceptor = match {
        // Extract the CertStrategy from the Box<dyn Any>
        let cert_strategy = match strategy.downcast::<crate::tls::strategy::CertStrategy>() {
            Ok(cs) => *cs,
            Err(_) => {
                let err_msg = "Failed to downcast strategy to CertStrategy";
                log::error!("{}", err_msg);
                return Err(ProxyError::Config(err_msg.to_string()));
            }
        };

        create_tls_acceptor(
            loaded_config.client_ca_cert(),
            &loaded_config.client_cert_mode(),
            cert_strategy,
        )
    } {
        Ok(acceptor) => {
            info!("Created TLS acceptor successfully");
            acceptor
        },
        Err(e) => {
            let err_msg = format!("Failed to create TLS acceptor: {}", e);
            log::error!("{}", err_msg);
            return Err(e);
        }
    };

    // Send update message to proxy service
    proxy_handle.update_config(tls_acceptor, Arc::clone(&loaded_config)).await?;

    info!("Proxy configuration reloaded successfully");
    Ok(loaded_config)
}
