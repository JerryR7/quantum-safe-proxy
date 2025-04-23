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
//! use quantum_safe_proxy::{Proxy, create_tls_acceptor, Result, parse_socket_addr};
//! use quantum_safe_proxy::config::ClientCertMode;
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
//!     let proxy = Proxy::new(
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
pub mod proxy;
pub mod tls;

// Re-export commonly used structures and functions for convenience
pub use proxy::Proxy;
pub use tls::create_tls_acceptor;
pub use common::{ProxyError, Result, parse_socket_addr};
use std::sync::Arc;

// Buffer size moved to ProxyConfig

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

/// Reload proxy configuration from file
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
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # use std::sync::Arc;
/// # use quantum_safe_proxy::config::ProxyConfig;
/// # use openssl::ssl::SslAcceptor;
/// # let tls_acceptor = SslAcceptor::mozilla_intermediate_v5(openssl::ssl::SslMethod::tls()).unwrap().build();
/// # let config = Arc::new(ProxyConfig::default());
/// # use std::net::SocketAddr;
/// # let mut proxy = Proxy::new("127.0.0.1:8443".parse::<SocketAddr>()?, "127.0.0.1:6000".parse::<SocketAddr>()?, tls_acceptor, config);
/// // Reload configuration
/// let new_config = reload_config(&mut proxy, Path::new("config.json"))?;
/// # Ok(())
/// # }
/// ```
pub fn reload_config(
    proxy: &mut Proxy,
    config_path: &std::path::Path,
) -> Result<Arc<config::ProxyConfig>> {
    use log::{info};

    info!("Reloading configuration from {}", config_path.display());

    // Reload configuration from file using the singleton manager
    config::reload_config(Some(config_path))?;

    // Get the updated configuration
    let new_config = config::get_config()?;

    // Build certificate strategy
    let strategy = new_config.build_cert_strategy()?;

    // Create new TLS acceptor with system-detected TLS settings
    let tls_acceptor = create_tls_acceptor(
        &new_config.ca_cert_path,
        &new_config.client_cert_mode,
        strategy,
    )?;

    // Update proxy configuration
    // 傳遞 Arc<ProxyConfig> 的引用，完全避免克隆
    proxy.update_config(new_config.target, tls_acceptor, &new_config);

    info!("Proxy configuration reloaded successfully");
    // 返回 Arc<ProxyConfig>，不需要克隆 ProxyConfig
    Ok(new_config)
}
