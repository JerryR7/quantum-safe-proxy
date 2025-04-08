//! Quantum Proxy: PQC-Enabled Sidecar with Hybrid Certificate Support
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
//! use quantum_proxy::{Proxy, create_tls_acceptor, Result, parse_socket_addr};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create TLS acceptor
//!     let tls_acceptor = create_tls_acceptor(
//!         Path::new("certs/server.crt"),
//!         Path::new("certs/server.key"),
//!         Path::new("certs/ca.crt"),
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
