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
pub use manager::{initialize, get_config, update_config, reload_config, add_listener, ConfigChangeEvent};

// Export constants needed externally
pub use defaults::{ENV_PREFIX, DEFAULT_CONFIG_FILE, DEFAULT_CONFIG_DIR};
pub use defaults::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};

use std::path::Path;
use std::env;
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

    // Load from configuration file if specified
    if let Some(path) = config_file {
        if Path::new(path).exists() {
            info!("Loading configuration from specified file: {}", path);
            match ProxyConfig::from_file(path) {
                Ok(file_config) => {
                    config = config.merge(file_config);
                    debug!("Merged configuration from file");
                },
                Err(e) => {
                    warn!("Failed to load configuration from file: {}", e);
                }
            }
        } else {
            // Try environment-specific configuration file
            let env = env::var(format!("{}_ENVIRONMENT", ENV_PREFIX))
                .unwrap_or_else(|_| "development".to_string());
            let env_config_file = format!("config.{}.json", env);

            if Path::new(&env_config_file).exists() {
                info!("Loading environment-specific configuration from {}", env_config_file);
                match ProxyConfig::from_file(&env_config_file) {
                    Ok(env_config) => {
                        config = config.merge(env_config);
                        debug!("Merged environment-specific configuration");
                    },
                    Err(e) => {
                        warn!("Failed to load environment configuration file: {}", e);
                    }
                }
            } else if Path::new(DEFAULT_CONFIG_FILE).exists() {
                // Try default configuration file
                info!("Loading configuration from {}", DEFAULT_CONFIG_FILE);
                match ProxyConfig::from_file(DEFAULT_CONFIG_FILE) {
                    Ok(file_config) => {
                        config = config.merge(file_config);
                        debug!("Merged default configuration file");
                    },
                    Err(e) => {
                        warn!("Failed to load default configuration file: {}", e);
                    }
                }
            }
        }
    }

    // Load from environment variables
    match ProxyConfig::from_env() {
        Ok(env_config) => {
            // Only merge if any environment variables were actually set
            if env_config != ProxyConfig::default() {
                info!("Loading configuration from environment variables");
                config = config.merge(env_config);
                debug!("Merged environment variables configuration");
            } else {
                debug!("No configuration found in environment variables");
            }
        },
        Err(e) => {
            warn!("Failed to load configuration from environment variables: {}", e);
        }
    }

    // Parse command line arguments
    // This is handled in main.rs using clap

    // Validate configuration
    config.validate()?;

    // Log configuration
    info!("Configuration loaded successfully");
    debug!("Listen address: {}", config.listen);
    debug!("Target address: {}", config.target);
    debug!("Certificate path: {:?}", config.cert_path);
    debug!("Private key path: {:?}", config.key_path);
    debug!("CA certificate path: {:?}", config.ca_cert_path);
    debug!("Hybrid mode: {}", config.hybrid_mode);
    debug!("Log level: {}", config.log_level);
    debug!("Client certificate mode: {}", config.client_cert_mode);

    Ok(config)
}

// Note: The reload_config function is now provided by the manager module
