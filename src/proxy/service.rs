//! Proxy service implementation
//!
//! This module implements the proxy service using a message-driven architecture.
//! It leverages Rust's trait system and ownership model to provide a clean,
//! lock-free implementation.

use log::{debug, error, info, warn};
// 暫時註釋掉 metrics，等待後續添加
// use metrics::{counter, gauge, histogram};
use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::select;

use crate::common::{ProxyError, Result};
use crate::config::ProxyConfig;
use super::handler::handle_connection;
use super::message::{ProxyMessage, ProxyHandle, create_channel};

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

/// Proxy service trait
///
/// This trait defines the interface for a proxy service.
/// It allows for different implementations of the proxy service
/// while maintaining a consistent interface.
pub trait ProxyService {
    /// Start the proxy service
    ///
    /// This method starts the proxy service and returns a handle
    /// that can be used to control the service.
    ///
    /// # Returns
    ///
    /// Returns a result containing the proxy handle
    fn start(self) -> Result<ProxyHandle>;
}

/// Proxy service state
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

/// Standard proxy service implementation
///
/// This structure implements the `ProxyService` trait using a message-driven
/// architecture to avoid locks and provide better separation of concerns.
pub struct StandardProxyService {
    /// Listen address for the proxy server
    listen_addr: SocketAddr,
    /// Target service address to forward traffic to
    target_addr: SocketAddr,
    /// TLS acceptor for handling secure connections
    tls_acceptor: Arc<SslAcceptor>,
    /// Proxy configuration (wrapped in Arc for efficient sharing)
    config: Arc<ProxyConfig>,
}

impl StandardProxyService {
    /// Create a new proxy service
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
    /// Returns a new proxy service
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
    async fn run_service(self, mut rx: mpsc::Receiver<ProxyMessage>) -> Result<()> {
        // Create TCP listener
        let listener = TcpListener::bind(self.listen_addr).await
            .map_err(ProxyError::Io)?;

        info!("Proxy service started, listening on {}", self.listen_addr);
        info!("Forwarding to {}", self.target_addr);

        // Initialize metrics
        // TODO: 添加 metrics 支持
        // gauge!("proxy.connections.active", 0.0);
        // counter!("proxy.connections.total", 0);

        // Create proxy state
        let mut proxy_state = ProxyState {
            target_addr: self.target_addr,
            tls_acceptor: self.tls_acceptor,
            config: self.config,
            tasks: JoinSet::new(),
            active_connections: 0,
        };

        // Create handle for sending messages back to the service
        let (handle, mut internal_rx) = create_channel();

        // Main event loop
        loop {
            // Use select to handle both incoming connections and messages
            select! {
                // Handle incoming connection
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((client_stream, client_addr)) => {
                            // Send message to handle connection
                            if let Err(e) = handle.send(ProxyMessage::HandleConnection {
                                client_stream,
                                client_addr,
                            }).await {
                                error!("Failed to send connection message: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Error accepting connection: {}", e);
                        }
                    }
                }

                // Handle message from external source
                Some(message) = rx.recv() => {
                    Self::process_message(&mut proxy_state, message).await;
                }

                // Handle message from internal source
                Some(message) = internal_rx.recv() => {
                    if let ProxyMessage::Shutdown = message {
                        info!("Received shutdown message from internal source");
                        break;
                    } else {
                        Self::process_message(&mut proxy_state, message).await;
                    }
                }

                // Check for completed tasks
                Some(result) = proxy_state.tasks.join_next() => {
                    // Update metrics
                    proxy_state.active_connections = proxy_state.active_connections.saturating_sub(1);
                    // TODO: 添加 metrics 支持
                    // gauge!("proxy.connections.active", proxy_state.active_connections as f64);

                    // Log any errors
                    if let Err(e) = result {
                        error!("Task error: {}", e);
                        // TODO: 添加 metrics 支持
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

    /// Process a proxy message
    ///
    /// This method processes a message received by the proxy service.
    ///
    /// # Parameters
    ///
    /// * `state` - Proxy state
    /// * `message` - Message to process
    async fn process_message(state: &mut ProxyState, message: ProxyMessage) {
        match message {
            ProxyMessage::HandleConnection { client_stream, client_addr } => {
                debug!("New connection attempt from {}", client_addr);

                // Update metrics
                state.active_connections += 1;
                // TODO: 添加 metrics 支持
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

                    // Log connection result
                    if let Err(e) = &result {
                        if let crate::common::ProxyError::NonTlsConnection(_) = e {
                            debug!("Rejected non-TLS connection from {}", conn_info.source);
                        }
                    }

                    // Record connection duration
                    if let Ok(duration) = SystemTime::now().duration_since(start_time) {
                        // TODO: 添加 metrics 支持
                        // histogram!("proxy.connection.duration_ms", duration.as_millis() as f64);
                        debug!("Connection duration: {} ms", duration.as_millis());
                    }

                    result
                });
            }
            ProxyMessage::UpdateConfig { target_addr, tls_acceptor, config } => {
                info!("Updating proxy configuration");
                info!("New target address: {}", target_addr);

                // Update proxy state
                state.target_addr = target_addr;
                state.tls_acceptor = Arc::new(tls_acceptor);
                state.config = config;

                info!("Proxy configuration updated successfully");
            }
            ProxyMessage::Shutdown => {
                info!("Received shutdown message");
                // Shutdown is handled in the main loop
            }
        }
    }
}

impl ProxyService for StandardProxyService {
    fn start(self) -> Result<ProxyHandle> {
        // Create message channel
        let (handle, rx) = create_channel();

        // Clone handle for returning
        let return_handle = handle.clone();

        // Spawn task to run the service
        tokio::spawn(async move {
            if let Err(e) = self.run_service(rx).await {
                error!("Proxy service error: {}", e);
            }
        });

        Ok(return_handle)
    }
}
