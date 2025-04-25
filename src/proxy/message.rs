//! Proxy message types
//!
//! This module defines the message types that can be sent to the proxy service.
//! Using a message-based architecture allows for better decoupling and avoids deadlocks.
//!
//! This design leverages Rust's ownership model and trait system to provide a clean,
//! type-safe interface for controlling the proxy service without locks.

use openssl::ssl::SslAcceptor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::common::Result;
use crate::common::ProxyError;
use crate::config::ProxyConfig;

/// Messages that can be sent to the proxy service
// 不能為 SslAcceptor 實現 Debug，所以不能為整個枚舉派生 Debug
pub enum ProxyMessage {
    /// Handle a new client connection
    HandleConnection {
        /// Client stream
        client_stream: TcpStream,
        /// Client address
        client_addr: SocketAddr,
    },
    /// Update proxy configuration
    UpdateConfig {
        /// New target address
        target_addr: SocketAddr,
        /// New TLS acceptor
        tls_acceptor: SslAcceptor,
        /// New proxy configuration
        config: Arc<ProxyConfig>,
    },
    /// Shutdown the proxy service
    Shutdown,
}

// 手動實現 Debug
impl std::fmt::Debug for ProxyMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandleConnection { client_addr, .. } => {
                f.debug_struct("HandleConnection")
                    .field("client_addr", client_addr)
                    .field("client_stream", &"<TcpStream>")
                    .finish()
            }
            Self::UpdateConfig { target_addr, config, .. } => {
                f.debug_struct("UpdateConfig")
                    .field("target_addr", target_addr)
                    .field("tls_acceptor", &"<SslAcceptor>")
                    .field("config", config)
                    .finish()
            }
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

/// Proxy control handle
///
/// This structure provides a way to control the proxy service
/// without holding a lock on the proxy itself.
///
/// It leverages Rust's ownership model to provide a clean interface
/// for sending messages to the proxy service.
#[derive(Debug, Clone)]
pub struct ProxyHandle {
    /// Message sender
    sender: mpsc::Sender<ProxyMessage>,
}

impl ProxyHandle {
    /// Create a new proxy handle
    pub fn new(sender: mpsc::Sender<ProxyMessage>) -> Self {
        Self { sender }
    }

    /// Send a message to the proxy service
    ///
    /// This is a generic method to send any message to the proxy service.
    ///
    /// # Parameters
    ///
    /// * `message` - Message to send
    ///
    /// # Returns
    ///
    /// Returns a result indicating success or failure
    pub async fn send(&self, message: ProxyMessage) -> Result<()> {
        self.sender.send(message).await
            .map_err(|_| ProxyError::Other("Failed to send message to proxy service".to_string()))
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
    pub async fn update_config(
        &self,
        target_addr: SocketAddr,
        tls_acceptor: SslAcceptor,
        config: Arc<ProxyConfig>
    ) -> Result<()> {
        self.send(ProxyMessage::UpdateConfig {
            target_addr,
            tls_acceptor,
            config,
        }).await
    }

    /// Shutdown the proxy service
    ///
    /// This method sends a shutdown message to the proxy service.
    ///
    /// # Returns
    ///
    /// Returns a result indicating success or failure
    pub async fn shutdown(&self) -> Result<()> {
        self.send(ProxyMessage::Shutdown).await
    }
}

/// Create a new proxy message channel
///
/// This function creates a new channel for sending messages to the proxy service.
/// It returns a sender and receiver pair.
///
/// # Returns
///
/// Returns a tuple containing the sender and receiver
pub fn create_channel() -> (ProxyHandle, mpsc::Receiver<ProxyMessage>) {
    let (tx, rx) = mpsc::channel(100);
    (ProxyHandle::new(tx), rx)
}
