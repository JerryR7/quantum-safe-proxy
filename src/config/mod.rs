//! Configuration module
//!
//! This module handles application configuration, including loading from
//! different sources (files, environment variables, command line arguments)
//! and validating the configuration.

mod config;
mod defaults;
mod manager;

// Re-export types
pub use config::{ProxyConfig, ClientCertMode};
pub use manager::{initialize, get_config, update_config, reload_config, add_listener, ConfigChangeEvent, get_buffer_size, get_connection_timeout};

// Export constants needed externally
pub use defaults::{ENV_PREFIX, DEFAULT_CONFIG_FILE, DEFAULT_CONFIG_DIR};
pub use defaults::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};

use std::path::Path;
// use std::env; // 不再需要，因為我們優化了配置加載流程
use log::{info, warn, debug};

use crate::common::Result;

/// Load configuration from multiple sources
///
/// This function loads configuration from the following sources in order of priority:
/// 1. Default values (lowest priority)
/// 2. Configuration file
/// 3. Environment variables
/// 4. Command line arguments (highest priority)
///
/// Optimized for performance with reduced file system access and validation.
///
/// # Arguments
///
/// * `args` - Command line arguments
/// * `config_file` - Optional path to configuration file
///
/// # Returns
///
/// The loaded configuration
pub fn load_config(_args: Vec<String>, config_file: Option<&str>) -> Result<ProxyConfig> {
    // Note: Command line arguments are currently handled by clap in main.rs
    // The _args parameter is kept for API compatibility
    // Start with default configuration
    let mut config = ProxyConfig::default();
    debug!("Starting with default configuration");

    // Optimized configuration file loading
    // Only check the file system once for each potential path
    if let Some(path) = config_file {
        // Try specified configuration file first
        let path_exists = Path::new(path).exists();
        if path_exists {
            info!("Loading configuration from specified file: {}", path);
            if let Ok(file_config) = ProxyConfig::from_file(path) {
                config = config.merge(file_config);
                debug!("Merged configuration from file");
            } else {
                warn!("Failed to load configuration from file: {}", path);
            }
        } else {
            // Try default configuration file if specified file doesn't exist
            let default_exists = Path::new(DEFAULT_CONFIG_FILE).exists();
            if default_exists {
                info!("Loading configuration from {}", DEFAULT_CONFIG_FILE);
                if let Ok(file_config) = ProxyConfig::from_file(DEFAULT_CONFIG_FILE) {
                    config = config.merge(file_config);
                    debug!("Merged default configuration file");
                } else {
                    warn!("Failed to load default configuration file");
                }
            }
        }
    } else if Path::new(DEFAULT_CONFIG_FILE).exists() {
        // No config file specified, try default
        info!("Loading configuration from {}", DEFAULT_CONFIG_FILE);
        if let Ok(file_config) = ProxyConfig::from_file(DEFAULT_CONFIG_FILE) {
            config = config.merge(file_config);
            debug!("Merged default configuration file");
        } else {
            warn!("Failed to load default configuration file");
        }
    }

    // Load from environment variables (optimized to avoid unnecessary processing)
    if let Ok(env_config) = ProxyConfig::from_env() {
        // Only merge if any environment variables were actually set
        if env_config != ProxyConfig::default() {
            info!("Loading configuration from environment variables");
            config = config.merge(env_config);
            debug!("Merged environment variables configuration");
        }
    }

    // Parse command line arguments
    // This is handled in main.rs using clap

    // Validate configuration
    config.validate()?;

    // Log configuration (only in debug mode to reduce overhead)
    info!("Configuration loaded successfully");
    if log::log_enabled!(log::Level::Debug) {
        debug!("Listen address: {}", config.listen);
        debug!("Target address: {}", config.target);
        debug!("Certificate path: {:?}", config.cert_path);
        debug!("Private key path: {:?}", config.key_path);
        debug!("CA certificate path: {:?}", config.ca_cert_path);
        debug!("Log level: {}", config.log_level);
        debug!("Client certificate mode: {}", config.client_cert_mode);
        debug!("Buffer size: {} bytes", config.buffer_size);
        debug!("Connection timeout: {} seconds", config.connection_timeout);
    }

    Ok(config)
}

// Note: The reload_config function is now provided by the manager module
