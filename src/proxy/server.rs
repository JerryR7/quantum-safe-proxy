//! Proxy server module
//!
//! This module implements the core functionality of the proxy server,
//! including TLS connection handling and traffic forwarding between
//! clients and the target service.

use log::{info, error, debug};
use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::common::{ProxyError, Result};
use crate::common::types::ConnectionInfo;
use std::time::SystemTime;

use super::handler::handle_connection;

/// Proxy server structure
///
/// Handles client connections and forwards traffic to the target service.
/// Supports TLS termination with hybrid certificate support for quantum-resistant
/// secure communications.
pub struct Proxy {
    /// Listen address for the proxy server
    listen_addr: SocketAddr,
    /// Target service address to forward traffic to
    target_addr: SocketAddr,
    /// TLS acceptor for handling secure connections
    tls_acceptor: Arc<SslAcceptor>,
}

impl Proxy {
    /// Create a new proxy instance
    ///
    /// # Parameters
    ///
    /// * `listen_addr` - Listen address
    /// * `target_addr` - Target service address
    /// * `tls_acceptor` - TLS acceptor
    ///
    /// # Returns
    ///
    /// Returns a new proxy instance
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::net::SocketAddr;
    /// # use openssl::ssl::SslAcceptor;
    /// # use quantum_safe_proxy::proxy::Proxy;
    /// # fn main() {
    /// # let tls_acceptor = SslAcceptor::mozilla_modern(openssl::ssl::SslMethod::tls()).unwrap().build();
    /// let proxy = Proxy::new(
    ///     "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
    ///     "127.0.0.1:6000".parse::<SocketAddr>().unwrap(),
    ///     tls_acceptor
    /// );
    /// # }
    /// ```
    pub fn new(
        listen_addr: impl Into<SocketAddr>,
        target_addr: impl Into<SocketAddr>,
        tls_acceptor: SslAcceptor,
    ) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            target_addr: target_addr.into(),
            tls_acceptor: Arc::new(tls_acceptor),
        }
    }

    /// Update the proxy configuration
    ///
    /// This method updates the proxy configuration with new values.
    /// Note that this does not affect existing connections, only new ones.
    ///
    /// # Parameters
    ///
    /// * `target_addr` - New target service address
    /// * `tls_acceptor` - New TLS acceptor
    ///
    /// # Returns
    ///
    /// Returns a reference to the updated proxy
    pub fn update_config(&mut self, target_addr: SocketAddr, tls_acceptor: SslAcceptor) -> &Self {
        log::info!("Updating proxy configuration");
        log::info!("New target address: {}", target_addr);

        self.target_addr = target_addr;
        self.tls_acceptor = Arc::new(tls_acceptor);

        log::info!("Proxy configuration updated successfully");
        self
    }

    /// Start the proxy service
    ///
    /// This method starts the proxy service, listens for connections and handles them.
    /// This is a blocking method that will run until an error occurs.
    ///
    /// # Returns
    ///
    /// Returns an error if one occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot bind to the listen address.
    pub async fn run(&self) -> Result<()> {
        // Create TCP listener
        let listener = TcpListener::bind(self.listen_addr).await
            .map_err(ProxyError::Io)?;

        info!("Proxy service started, listening on {}", self.listen_addr);

        // Accept connections
        loop {
            match listener.accept().await {
                Ok((client_stream, client_addr)) => {
                    info!("Accepted connection from {}", client_addr);

                    // Create connection info
                    let conn_info = ConnectionInfo {
                        source: client_addr.to_string(),
                        target: self.target_addr.to_string(),
                        timestamp: SystemTime::now(),
                    };

                    // Clone necessary data for use in the new task
                    let tls_acceptor = self.tls_acceptor.clone();
                    let target_addr = self.target_addr;

                    // Handle connection in a new task
                    tokio::spawn(async move {
                        debug!("Starting to handle connection: {} -> {}", conn_info.source, conn_info.target);
                        if let Err(e) = handle_connection(client_stream, target_addr, tls_acceptor).await {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ssl::{SslMethod, SslAcceptor};

    #[test]
    fn test_proxy_new() {
        // Create a simple SSL acceptor for testing
        let acceptor = SslAcceptor::mozilla_modern(SslMethod::tls()).unwrap().build();

        // Test creating a proxy instance
        let proxy = Proxy::new(
            "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:6000".parse::<SocketAddr>().unwrap(),
            acceptor
        );

        assert_eq!(proxy.listen_addr.port(), 8443);
        assert_eq!(proxy.target_addr.port(), 6000);
    }
}
