//! Proxy server module
//!
//! This module implements the core functionality of the proxy server,
//! including TLS connection handling and traffic forwarding between
//! clients and the target service.
//!
//! The proxy server uses a message-driven architecture to avoid deadlocks
//! and provide better separation of concerns.

use log::{info, error, debug, warn};
// Metrics temporarily commented out, will be added later
// use metrics::{counter, gauge, histogram};
use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinSet;
use tokio::select;

use crate::common::{ProxyError, Result};
use crate::config::ProxyConfig;

use super::message::ProxyMessage;

use super::handler::handle_connection;

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Source address
    pub source: String,
    /// Target address
    pub target: String,
    /// Connection timestamp
    pub timestamp: SystemTime,
}

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
    /// Message sender for proxy control
    message_tx: Option<Sender<ProxyMessage>>,
}

impl Proxy {
    /// Handle a new client connection
    ///
    /// This method handles a new client connection by spawning a new task to handle it.
    ///
    /// # Parameters
    ///
    /// * `client_stream` - Client TCP stream
    /// * `client_addr` - Client address
    /// * `state` - Proxy state
    async fn handle_new_connection(
        client_stream: TcpStream,
        client_addr: SocketAddr,
        state: &mut ProxyState,
    ) {
        debug!("Accepted connection from {}", client_addr);

        // Update metrics
        state.active_connections += 1;
        // TODO: Add metrics support
        // gauge!("proxy.connections.active", state.active_connections as f64);
        // counter!("proxy.connections.total", 1);

        // Create connection info
        let conn_info = ConnectionInfo {
            source: client_addr.to_string(),
            target: state.target_addr.to_string(),
            timestamp: SystemTime::now(),
        };

        // Clone necessary data for use in the new task
        let tls_acceptor = Arc::clone(&state.tls_acceptor);
        let target_addr = state.target_addr;
        let config = Arc::clone(&state.config);

        // Add connection handling task to JoinSet
        state.tasks.spawn(async move {
            let start_time = SystemTime::now();
            debug!("Starting to handle connection: {} -> {}", conn_info.source, conn_info.target);

            let result = handle_connection(client_stream, target_addr, tls_acceptor, &config).await;

            // Record connection duration
            if let Ok(duration) = SystemTime::now().duration_since(start_time) {
                // TODO: Add metrics support
                // histogram!("proxy.connection.duration_ms", duration.as_millis() as f64);
                debug!("Connection duration: {} ms", duration.as_millis());
            }

            result
        });
    }

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
            target_addr: target_addr.into(),  // We'll still use this for initial setup
            tls_acceptor: Arc::new(tls_acceptor),
            config,
            message_tx: None,
        }
    }

    /// Update the proxy configuration
    ///
    /// This method sends a configuration update message to the proxy service.
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
    /// Returns a result indicating success or failure
    pub async fn update_config(&self, tls_acceptor: SslAcceptor, config: &Arc<ProxyConfig>) -> Result<()> {
        if let Some(tx) = &self.message_tx {
            // Use the current target address
            let target_addr = self.target_addr;

            info!("Sending configuration update message");
            info!("New target address: {}", target_addr);

            // Send update message to proxy service
            tx.send(ProxyMessage::UpdateConfig {
                target_addr,
                tls_acceptor,
                config: Arc::clone(config),
            }).await.map_err(|_| ProxyError::Other("Failed to send configuration update message".to_string()))?;

            info!("Configuration update message sent successfully");
            Ok(())
        } else {
            Err(ProxyError::Other("Proxy service not running".to_string()))
        }
    }

    /// Start the proxy service
    ///
    /// This method starts the proxy service, listens for connections and handles them.
    /// It uses a message-driven architecture to avoid deadlocks and provide better
    /// separation of concerns.
    ///
    /// # Returns
    ///
    /// Returns a result indicating success or failure
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot bind to the listen address.
    pub async fn run(&mut self) -> Result<()> {
        // Create message channel
        let (tx, rx) = mpsc::channel(100);
        self.message_tx = Some(tx.clone());

        // Start proxy service
        self.run_service(rx).await
    }

    /// Run the proxy service with the given message receiver
    ///
    /// This method is the core of the proxy service. It listens for connections
    /// and handles messages from the message channel.
    ///
    /// # Parameters
    ///
    /// * `rx` - Message receiver
    ///
    /// # Returns
    ///
    /// Returns a result indicating success or failure
    async fn run_service(&self, mut rx: Receiver<ProxyMessage>) -> Result<()> {
        // Create TCP listener
        let listener = TcpListener::bind(self.listen_addr).await
            .map_err(ProxyError::Io)?;

        info!("Proxy service started, listening on {}", self.listen_addr);
        info!("Forwarding to {}", self.target_addr);

        // Initialize metrics
        // TODO: Add metrics support
        // gauge!("proxy.connections.active", 0.0);
        // counter!("proxy.connections.total", 0);

        // Create proxy state
        let mut proxy_state = ProxyState {
            target_addr: self.target_addr,
            tls_acceptor: Arc::clone(&self.tls_acceptor),
            config: Arc::clone(&self.config),
            tasks: JoinSet::new(),
            active_connections: 0,
        };

        // Main event loop
        loop {
            // Use select to handle both incoming connections and messages
            select! {
                // Handle incoming connection
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((client_stream, client_addr)) => {
                            // Directly handle connection, no need to send message
                            Self::handle_new_connection(client_stream, client_addr, &mut proxy_state).await;
                        }
                        Err(e) => {
                            error!("Error accepting connection: {}", e);
                        }
                    }
                }

                // Handle message
                Some(message) = rx.recv() => {
                    match message {
                        ProxyMessage::HandleConnection { client_stream, client_addr } => {
                            Self::handle_new_connection(client_stream, client_addr, &mut proxy_state).await;
                        }
                        ProxyMessage::UpdateConfig { target_addr, tls_acceptor, config } => {
                            info!("Updating proxy configuration");
                            info!("New target address: {}", target_addr);

                            // Update proxy state
                            proxy_state.target_addr = target_addr;
                            proxy_state.tls_acceptor = Arc::new(tls_acceptor);
                            proxy_state.config = config;

                            info!("Proxy configuration updated successfully");
                        }
                        ProxyMessage::Shutdown => {
                            info!("Shutting down proxy service");
                            break;
                        }
                    }
                }

                // Check for completed tasks
                Some(result) = proxy_state.tasks.join_next() => {
                    // Update metrics
                    proxy_state.active_connections = proxy_state.active_connections.saturating_sub(1);
                    // TODO: Add metrics support
                    // gauge!("proxy.connections.active", proxy_state.active_connections as f64);

                    // Log any errors
                    if let Err(e) = result {
                        error!("Task error: {}", e);
                        // TODO: Add metrics support
                        // counter!("proxy.errors", 1);
                    }
                }
            }

            // Periodically log statistics
            if proxy_state.active_connections > 0 && proxy_state.active_connections % 100 == 0 {
                info!("Active connections: {}", proxy_state.active_connections);
            }
        }

        // Wait for all tasks to complete with a timeout
        info!("Waiting for all connections to complete...");
        let shutdown_timeout = Duration::from_secs(30);
        let shutdown_start = SystemTime::now();

        while proxy_state.active_connections > 0 {
            if let Ok(elapsed) = SystemTime::now().duration_since(shutdown_start) {
                if elapsed > shutdown_timeout {
                    warn!("Shutdown timeout reached, {} connections still active", proxy_state.active_connections);
                    break;
                }
            }

            if let Some(result) = proxy_state.tasks.join_next().await {
                proxy_state.active_connections = proxy_state.active_connections.saturating_sub(1);
                if let Err(e) = result {
                    error!("Task error during shutdown: {}", e);
                }
            }
        }

        info!("Proxy service shutdown complete");
        Ok(())
    }
}

/// Internal proxy state
///
/// This structure holds the mutable state of the proxy service.
struct ProxyState {
    /// Target service address to forward traffic to
    target_addr: SocketAddr,
    /// TLS acceptor for handling secure connections
    tls_acceptor: Arc<SslAcceptor>,
    /// Proxy configuration
    config: Arc<ProxyConfig>,
    /// Task set for managing connection tasks
    tasks: JoinSet<Result<()>>,
    /// Number of active connections
    active_connections: usize,
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
        let config = crate::config::ProxyConfig::default();
        let proxy = Proxy::new(
            "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:6000".parse::<SocketAddr>().unwrap(),
            acceptor,
            Arc::new(config)  // Wrap ProxyConfig in Arc
        );

        assert_eq!(proxy.listen_addr.port(), 8443);
        assert_eq!(proxy.target_addr.port(), 6000);
    }
}
