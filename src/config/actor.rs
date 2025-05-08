//! Configuration actor implementation
//!
//! This module provides an actor-based approach to configuration management,
//! avoiding locks and providing better separation of concerns.

use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use log::{debug, info, warn};

use crate::config::types::ProxyConfig;
use crate::config::validator::validate_config;
use crate::config::error::{Result, ConfigError};

/// Configuration message types
#[derive(Debug)]
pub enum ConfigMessage {
    /// Get the current configuration
    GetConfig {
        /// Response channel
        response: oneshot::Sender<Arc<ProxyConfig>>,
    },

    /// Update the configuration
    UpdateConfig {
        /// New configuration
        config: ProxyConfig,
        /// Response channel
        response: oneshot::Sender<Result<()>>,
    },

    /// Reload configuration from a file
    ReloadConfig {
        /// Path to the configuration file
        path: Box<Path>,
        /// Response channel
        response: oneshot::Sender<Result<Arc<ProxyConfig>>>,
    },

    /// Shutdown the actor
    Shutdown,
}

/// Configuration actor handle
#[derive(Clone)]
pub struct ConfigActor {
    /// Message sender
    sender: mpsc::Sender<ConfigMessage>,
}

impl ConfigActor {
    /// Create a new configuration actor
    pub fn new(initial_config: ProxyConfig) -> Self {
        let (sender, receiver) = mpsc::channel(32);

        // Start the actor task
        tokio::spawn(Self::run(receiver, initial_config));

        Self { sender }
    }

    /// Run the actor task
    async fn run(mut receiver: mpsc::Receiver<ConfigMessage>, initial_config: ProxyConfig) {
        let mut config = Arc::new(initial_config);

        while let Some(msg) = receiver.recv().await {
            match msg {
                ConfigMessage::GetConfig { response } => {
                    let _ = response.send(Arc::clone(&config));
                },

                ConfigMessage::UpdateConfig { config: new_config, response } => {
                    match validate_config(&new_config) {
                        Ok(()) => {
                            config = Arc::new(new_config);
                            debug!("Configuration updated successfully");
                            let _ = response.send(Ok(()));
                        },
                        Err(e) => {
                            warn!("Failed to validate configuration: {}", e);
                            let _ = response.send(Err(e));
                        },
                    }
                },

                ConfigMessage::ReloadConfig { path, response } => {
                    info!("Reloading configuration from {}", path.display());

                    // Use the builder to load configuration
                    let result = crate::config::builder::ConfigBuilder::new()
                        .with_defaults()
                        .with_file(&*path)
                        .build();

                    match result {
                        Ok(new_config) => {
                            // Update the configuration
                            config = Arc::new(new_config);
                            debug!("Configuration reloaded successfully");
                            let _ = response.send(Ok(Arc::clone(&config)));
                        },
                        Err(e) => {
                            warn!("Failed to load configuration from {}: {}", path.display(), e);
                            let _ = response.send(Err(e));
                        },
                    }
                },

                ConfigMessage::Shutdown => {
                    debug!("Configuration actor shutting down");
                    break;
                },
            }
        }

        debug!("Configuration actor stopped");
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> Arc<ProxyConfig> {
        let (sender, receiver) = oneshot::channel();

        if let Err(e) = self.sender.send(ConfigMessage::GetConfig { response: sender }).await {
            warn!("Failed to send GetConfig message: {}", e);
            return Arc::new(ProxyConfig::default());
        }

        match receiver.await {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to receive configuration: {}", e);
                Arc::new(ProxyConfig::default())
            },
        }
    }

    /// Update the configuration
    pub async fn update_config(&self, config: ProxyConfig) -> Result<()> {
        let (sender, receiver) = oneshot::channel();

        if let Err(e) = self.sender.send(ConfigMessage::UpdateConfig { config, response: sender }).await {
            warn!("Failed to send UpdateConfig message: {}", e);
            return Err(ConfigError::Other(format!("Failed to send message: {}", e)));
        }

        match receiver.await {
            Ok(result) => result,
            Err(e) => {
                warn!("Failed to receive update result: {}", e);
                Err(ConfigError::Other(format!("Failed to receive response: {}", e)))
            },
        }
    }

    /// Reload configuration from a file
    pub async fn reload_config<P: AsRef<Path>>(&self, path: P) -> Result<Arc<ProxyConfig>> {
        let path = path.as_ref().to_path_buf().into_boxed_path();
        let (sender, receiver) = oneshot::channel();

        if let Err(e) = self.sender.send(ConfigMessage::ReloadConfig { path, response: sender }).await {
            warn!("Failed to send ReloadConfig message: {}", e);
            return Err(ConfigError::Other(format!("Failed to send message: {}", e)));
        }

        match receiver.await {
            Ok(result) => result,
            Err(e) => {
                warn!("Failed to receive reload result: {}", e);
                Err(ConfigError::Other(format!("Failed to receive response: {}", e)))
            },
        }
    }

    /// Shutdown the actor
    pub async fn shutdown(&self) {
        if let Err(e) = self.sender.send(ConfigMessage::Shutdown).await {
            warn!("Failed to send Shutdown message: {}", e);
        }
    }
}
