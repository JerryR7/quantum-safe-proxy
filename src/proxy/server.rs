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
use tokio::task::JoinSet;

use crate::common::{ProxyError, Result};
use crate::config::ProxyConfig;
use std::time::SystemTime;

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Source address
    pub source: String,
    /// Target address
    pub target: String,
    /// Connection timestamp
    #[allow(dead_code)]
    pub timestamp: SystemTime,
}

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
    /// Proxy configuration (wrapped in Arc for efficient sharing)
    config: Arc<ProxyConfig>,
}

impl Proxy {
    /// Create a new proxy instance
    ///
    /// # Parameters
    ///
    /// * `listen_addr` - Listen address
    /// * `target_addr` - Target service address
    /// * `tls_acceptor` - TLS acceptor
    /// * `config` - Proxy configuration
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
    /// # use quantum_safe_proxy::config::ProxyConfig;
    /// # fn main() {
    /// # let tls_acceptor = SslAcceptor::mozilla_modern(openssl::ssl::SslMethod::tls()).unwrap().build();
    /// # use std::sync::Arc;
    /// # let config = Arc::new(ProxyConfig::default());
    /// let proxy = Proxy::new(
    ///     "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
    ///     "127.0.0.1:6000".parse::<SocketAddr>().unwrap(),
    ///     tls_acceptor,
    ///     config
    /// );
    /// # }
    /// ```
    pub fn new(
        listen_addr: impl Into<SocketAddr>,
        target_addr: impl Into<SocketAddr>,
        tls_acceptor: SslAcceptor,
        config: Arc<ProxyConfig>,
    ) -> Self {
        Self {
            listen_addr: listen_addr.into(),
            target_addr: target_addr.into(),
            tls_acceptor: Arc::new(tls_acceptor),
            config,
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
    /// * `config` - New proxy configuration
    ///
    /// # Returns
    ///
    /// Returns a reference to the updated proxy
    pub fn update_config(&mut self, target_addr: SocketAddr, tls_acceptor: SslAcceptor, config: &Arc<ProxyConfig>) -> &Self {
        log::info!("Updating proxy configuration");
        log::info!("New target address: {}", target_addr);

        self.target_addr = target_addr;
        self.tls_acceptor = Arc::new(tls_acceptor);
        self.config = Arc::clone(config); // 使用 Arc::clone 更明確地表達只是增加引用計數

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

        // Create a JoinSet to manage tasks efficiently
        let mut tasks = JoinSet::new();

        // Accept connections
        loop {
            // Check for completed tasks and log any errors
            while let Some(result) = tasks.try_join_next() {
                if let Err(e) = result {
                    error!("Task error: {}", e);
                }
            }

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
                    let tls_acceptor = Arc::clone(&self.tls_acceptor);
                    let target_addr = self.target_addr;
                    let config = Arc::clone(&self.config);

                    // Add connection handling task to JoinSet
                    tasks.spawn(async move {
                        debug!("Starting to handle connection: {} -> {}", conn_info.source, conn_info.target);
                        handle_connection(client_stream, target_addr, tls_acceptor, &config).await
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
    use crate::config::ProxyConfig;

    #[test]
    fn test_proxy_new() {
        // Create a simple SSL acceptor for testing
        let acceptor = SslAcceptor::mozilla_modern(SslMethod::tls()).unwrap().build();

        // Test creating a proxy instance
        let config = ProxyConfig::default();
        let proxy = Proxy::new(
            "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:6000".parse::<SocketAddr>().unwrap(),
            acceptor,
            Arc::new(config)  // 將 ProxyConfig 包裝在 Arc 中
        );

        assert_eq!(proxy.listen_addr.port(), 8443);
        assert_eq!(proxy.target_addr.port(), 6000);
    }
}
