//! Configuration management functionality
//!
//! This module provides functionality for managing configuration at runtime,
//! including reloading configuration from files and updating the global configuration.

use std::path::Path;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use log::{debug, info};

use crate::common::Result;
use crate::config::ProxyConfig;
use crate::config::traits::{ConfigLoader, ConfigValidator};

// Global configuration storage with improved locking
static CONFIG: Lazy<RwLock<ProxyConfig>> = Lazy::new(|| {
    RwLock::new(ProxyConfig::default())
});

/// Initialize the global configuration
///
/// This function initializes the global configuration with the provided configuration.
/// It should be called once at application startup.
pub fn initialize(config: ProxyConfig) -> Result<()> {
    // Set the configuration in the global state
    let mut global_config = CONFIG.write().unwrap();
    *global_config = config;

    debug!("Global configuration initialized");
    Ok(())
}

/// Get the current global configuration
///
/// This function returns a clone of the current global configuration.
pub fn get_config() -> ProxyConfig {
    // Retrieve the config from the global state
    let config = CONFIG.read().unwrap();
    config.clone()
}

/// Update the global configuration
///
/// This function updates the global configuration with the provided configuration.
/// It validates the configuration before updating.
pub fn update_config(config: ProxyConfig) -> Result<()> {
    // Validate the configuration before updating
    config.validate()?;

    // Update the configuration in the global state
    let mut global_config = CONFIG.write().unwrap();
    *global_config = config;

    debug!("Global configuration updated");

    Ok(())
}

/// Reload configuration from a file
///
/// This function reloads the configuration from the specified file,
/// merges it with the current configuration, and updates the global configuration.
pub fn reload_config<P: AsRef<Path>>(path: P) -> Result<ProxyConfig> {
    let path = path.as_ref();
    info!("Reloading configuration from {}", path.display());

    // Get the current configuration
    let current_config = get_config();

    // Load configuration from file
    let file_config = ProxyConfig::from_file(path)?;
    debug!("Loaded configuration from file");

    // Merge with current configuration
    let loaded_config = current_config.merge(file_config);
    debug!("Merged with current configuration");

    // Update the configuration in the global state
    update_config(loaded_config.clone())?;
    info!("Configuration updated successfully");

    Ok(loaded_config)
}

/// Configuration change event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChangeEvent {
    /// Configuration was updated
    Updated,
    /// Configuration was reloaded from a file
    Reloaded,
}

/// Get the buffer size from the current configuration
pub fn get_buffer_size() -> usize {
    get_config().buffer_size
}

/// Get the connection timeout from the current configuration
pub fn get_connection_timeout() -> u64 {
    get_config().connection_timeout
}

/// Check if client certificates are required
pub fn is_client_cert_required() -> bool {
    matches!(get_config().client_cert_mode, crate::config::ClientCertMode::Required)
}

/// Check if sigalgs are enabled
pub fn is_sigalgs_enabled() -> bool {
    matches!(get_config().strategy, crate::config::CertStrategyType::SigAlgs)
}
