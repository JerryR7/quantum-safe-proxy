//! Quantum Safe Proxy: PQC-Enabled Sidecar with Hybrid Certificate Support
//!
//! This library uses Arc<ProxyConfig> for efficient configuration sharing and minimal cloning.
//!
//! This library implements a TCP proxy with support for Post-Quantum Cryptography (PQC)
//! and hybrid X.509 certificates. It can be deployed as a sidecar to provide PQC protection
//! for existing services.
//!
//! # Main Features
//!
//! - Support for hybrid PQC + traditional certificates (e.g., Kyber + ECDSA)
//! - Transparent support for both PQC-capable and traditional clients
//! - Efficient TCP proxy forwarding
//! - Complete mTLS support
//!
//! # Example
//!
//! ```no_run
//! use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result};
//! use quantum_safe_proxy::config::{parse_socket_addr, ClientCertMode};
//! use quantum_safe_proxy::tls::strategy::CertStrategy;
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create certificate strategy
//!     let strategy = CertStrategy::Single {
//!         cert: Path::new("certs/hybrid/dilithium3/server.crt").to_path_buf(),
//!         key: Path::new("certs/hybrid/dilithium3/server.key").to_path_buf(),
//!     };
//!
//!     // Create TLS acceptor with system-detected TLS settings
//!     let tls_acceptor = create_tls_acceptor(
//!         Path::new("certs/hybrid/dilithium3/ca.crt"),
//!         &ClientCertMode::Required,
//!         strategy,
//!     )?;
//!
//!     // Parse addresses
//!     let listen_addr = parse_socket_addr("0.0.0.0:8443")?;
//!     let target_addr = parse_socket_addr("127.0.0.1:6000")?;
//!
//!     // Create default config and wrap in Arc
//!     let config = std::sync::Arc::new(quantum_safe_proxy::config::ProxyConfig::default());
//!
//!     // Create and start proxy
//!     let mut proxy = Proxy::new(
//!         listen_addr,
//!         target_addr,
//!         tls_acceptor,
//!         config,  // Use Arc<ProxyConfig>
//!     );
//!
//!     // Run proxy service
//!     proxy.run().await?;
//!
//!     Ok(())
//! }
//! ```

// Public modules
pub mod common;
pub mod config;
pub mod crypto;
pub mod protocol;
pub mod proxy;
pub mod tls;

// Re-export commonly used structures and functions for convenience
pub use proxy::Proxy; // Legacy export
pub use proxy::{ProxyService, StandardProxyService, ProxyHandle, ProxyMessage}; // New message-driven architecture
pub use tls::create_tls_acceptor;
pub use common::{ProxyError, Result};
pub use config::ProxyConfig;
pub use config::builder::auto_load;
pub use config::validator::{ConfigValidator, check_warnings};
pub use tls::build_cert_strategy;
use std::sync::Arc;

// Buffer size moved to ProxyConfig

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Reload proxy configuration from file (legacy version)
///
/// This function reloads the proxy configuration from the specified file and
/// updates the proxy instance with the new configuration.
///
/// # Parameters
///
/// * `proxy` - Mutable reference to the proxy instance
/// * `config_path` - Path to the configuration file
///
/// # Returns
///
/// Returns the updated configuration if successful, otherwise returns an error.
///
/// # Example
///
/// ```no_run
/// # use quantum_safe_proxy::{Proxy, reload_config};
/// # use std::path::Path;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::sync::Arc;
/// # use quantum_safe_proxy::config::ProxyConfig;
/// # use openssl::ssl::SslAcceptor;
/// # let tls_acceptor = SslAcceptor::mozilla_intermediate_v5(openssl::ssl::SslMethod::tls()).unwrap().build();
/// # let config = Arc::new(ProxyConfig::default());
/// # use std::net::SocketAddr;
/// # let mut proxy = Proxy::new("127.0.0.1:8443".parse::<SocketAddr>()?, "127.0.0.1:6000".parse::<SocketAddr>()?, tls_acceptor, config);
/// // Reload configuration
/// let new_config = reload_config(&mut proxy, Path::new("config.json")).await?;
/// # Ok(())
/// # }
/// ```
pub async fn reload_config(
    proxy: &mut Proxy,
    config_path: &std::path::Path,
) -> Result<Arc<config::ProxyConfig>> {
    use log::info;

    info!("Reloading configuration from {}", config_path.display());

    // Reload configuration from file using the singleton manager
    let loaded_config = match config::ProxyConfig::from_file(config_path.to_str().unwrap_or("config.json")) {
        Ok(config) => {
            info!("Configuration reloaded successfully from file");
            Arc::new(config)
        },
        Err(e) => {
            let err_msg = format!("Failed to reload configuration from file: {}", e);
            log::error!("{}", err_msg);
            return Err(e.into());
        }
    };

    // Build certificate strategy
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

    // Create new TLS acceptor with system-detected TLS settings
    let tls_acceptor = match {
        // Extract the CertStrategy from the Box<dyn Any>
        let cert_strategy = match strategy.downcast::<crate::tls::strategy::CertStrategy>() {
            Ok(cs) => *cs,  // Unbox it
            Err(_) => {
                let err_msg = "Failed to downcast strategy to CertStrategy";
                log::error!("{}", err_msg);
                return Err(ProxyError::Config(err_msg.to_string()));
            }
        };

        // Now call create_tls_acceptor with the correct types
        create_tls_acceptor(
            &loaded_config.client_ca_cert_path(),
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

    // Update proxy configuration
    proxy.update_config(tls_acceptor, &loaded_config).await?;

    info!("Proxy configuration reloaded successfully");
    // Return Arc<ProxyConfig> without cloning ProxyConfig
    Ok(loaded_config)
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
///
/// # Example
///
/// ```no_run
/// # use quantum_safe_proxy::{StandardProxyService, ProxyService, reload_config_async};
/// # use std::path::Path;
/// # use std::sync::Arc;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # use quantum_safe_proxy::config::ProxyConfig;
/// # use openssl::ssl::SslAcceptor;
/// # let tls_acceptor = SslAcceptor::mozilla_intermediate_v5(openssl::ssl::SslMethod::tls()).unwrap().build();
/// # let config = Arc::new(ProxyConfig::default());
/// # use std::net::SocketAddr;
/// # let service = StandardProxyService::new(
/// #    "127.0.0.1:8443".parse::<SocketAddr>()?,
/// #    "127.0.0.1:6000".parse::<SocketAddr>()?,
/// #    tls_acceptor,
/// #    config
/// # );
/// # let proxy_handle = service.start()?;
/// // Reload configuration
/// let new_config = reload_config_async(&proxy_handle, Path::new("config.json")).await?;
/// # Ok(())
/// # }
/// ```
pub async fn reload_config_async(
    proxy_handle: &ProxyHandle,
    config_path: &std::path::Path,
) -> Result<Arc<config::ProxyConfig>> {
    use log::info;

    info!("Reloading configuration from {}", config_path.display());

    // Reload configuration from file using the singleton manager
    let loaded_config = match config::ProxyConfig::from_file(config_path.to_str().unwrap_or("config.json")) {
        Ok(config) => {
            info!("Configuration reloaded successfully from file");
            Arc::new(config)
        },
        Err(e) => {
            let err_msg = format!("Failed to reload configuration from file: {}", e);
            log::error!("{}", err_msg);
            return Err(e.into());
        }
    };

    // Build certificate strategy
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

    // Create new TLS acceptor with system-detected TLS settings
    let tls_acceptor = match {
        // Extract the CertStrategy from the Box<dyn Any>
        let cert_strategy = match strategy.downcast::<crate::tls::strategy::CertStrategy>() {
            Ok(cs) => *cs,  // Unbox it
            Err(_) => {
                let err_msg = "Failed to downcast strategy to CertStrategy";
                log::error!("{}", err_msg);
                return Err(ProxyError::Config(err_msg.to_string()));
            }
        };

        // Now call create_tls_acceptor with the correct types
        create_tls_acceptor(
            &loaded_config.client_ca_cert_path(),
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
    // Return Arc<ProxyConfig> without cloning ProxyConfig
    Ok(loaded_config)
}
