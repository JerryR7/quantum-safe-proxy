//! Quantum Safe Proxy: PQC-Enabled Sidecar with Hybrid Certificate Support
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
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create TLS acceptor with system-detected TLS settings
//!     let tls_acceptor = create_tls_acceptor(
//!         Path::new("certs/hybrid/dilithium3/server.crt"),
//!         Path::new("certs/hybrid/dilithium3/server.key"),
//!         Path::new("certs/hybrid/dilithium3/ca.crt"),
//!         &ClientCertMode::Required,
//!     )?;
//!
//!     // Parse addresses
//!     let listen_addr = parse_socket_addr("0.0.0.0:8443")?;
//!     let target_addr = parse_socket_addr("127.0.0.1:6000")?;
//!
//!     // Create and start proxy
//!     let proxy = Proxy::new(
//!         listen_addr,
//!         target_addr,
//!         tls_acceptor,
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

/// Buffer size constant (8KB)
pub const BUFFER_SIZE: usize = 8192;

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
/// * `config` - Current configuration
/// * `config_path` - Path to the configuration file
///
/// # Returns
///
/// Returns the updated configuration if successful, otherwise returns an error.
///
/// # Example
///
/// ```no_run
/// # use quantum_safe_proxy::{Proxy, config::ProxyConfig, reload_config};
/// # use std::path::Path;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut proxy = Proxy::new("127.0.0.1:8443".parse()?, "127.0.0.1:6000".parse()?, Default::default());
/// # let mut config = ProxyConfig::default();
/// // Reload configuration
/// let new_config = reload_config(&mut proxy, &config, Path::new("config.json"))?;
/// # Ok(())
/// # }
/// ```
pub fn reload_config(
    proxy: &mut Proxy,
    config: &config::ProxyConfig,
    config_path: &std::path::Path,
) -> Result<config::ProxyConfig> {
    use log::{info};

    info!("Reloading configuration from {}", config_path.display());

    // Reload configuration from file
    let new_config = config.reload_from_file(config_path)?;

    // Create new TLS acceptor with system-detected TLS settings
    let tls_acceptor = create_tls_acceptor(
        &new_config.cert_path,
        &new_config.key_path,
        &new_config.ca_cert_path,
        &new_config.client_cert_mode,
    )?;

    // Update proxy configuration
    proxy.update_config(new_config.target, tls_acceptor);

    info!("Proxy configuration reloaded successfully");
    Ok(new_config)
}
