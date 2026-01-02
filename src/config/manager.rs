//! Configuration manager
//!
//! This module provides functionality for managing configuration at runtime,
//! including reloading configuration from files and updating the global configuration.

use std::sync::{Arc, RwLock};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;
use log::info;

use crate::config::types::{ProxyConfig, ClientCertMode, ValueSource};
use crate::config::source::{ConfigSource, FileSource};
use crate::config::validator::validate_config;
use crate::config::error::Result;

/// Configuration change event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigChangeEvent {
    /// Configuration was updated
    Updated,
    /// Configuration was reloaded from file
    Reloaded,
}

/// Configuration change listener type
pub type ConfigChangeListener = Box<dyn Fn(ConfigChangeEvent) + Send + Sync>;

/// Global configuration manager
pub struct ConfigManager {
    /// Current configuration
    config: RwLock<Arc<ProxyConfig>>,

    /// Configuration change listeners
    listeners: RwLock<Vec<ConfigChangeListener>>,

    /// Cached value for client certificate required
    client_cert_required: AtomicBool,

    /// Cached value for dynamic certificate selection enabled
    dynamic_cert_enabled: AtomicBool,
}

impl ConfigManager {
    /// Create a new configuration manager
    fn new() -> Self {
        let config = ProxyConfig::default();
        let client_cert_required = config.client_cert_mode() == ClientCertMode::Required;
        // Dynamic mode is enabled when fallback certificates are configured
        let dynamic_cert_enabled = config.has_fallback();

        log::info!("Creating new ConfigManager with default configuration");
        log::info!("Default listen address: {}", config.listen());
        log::info!("Default target address: {}", config.target());
        log::info!("Default log level: {}", config.log_level());

        Self {
            config: RwLock::new(Arc::new(config)),
            listeners: RwLock::new(Vec::new()),
            client_cert_required: AtomicBool::new(client_cert_required),
            dynamic_cert_enabled: AtomicBool::new(dynamic_cert_enabled),
        }
    }

    /// Get the current configuration
    fn get_config(&self) -> Arc<ProxyConfig> {
        let config = self.config.read().unwrap();
        Arc::clone(&config)
    }

    /// Update the configuration
    fn update_config(&self, config: ProxyConfig, event: ConfigChangeEvent) -> Result<()> {
        // Validate the configuration
        validate_config(&config)?;

        // Update cached values
        let client_cert_required = config.client_cert_mode() == ClientCertMode::Required;
        let dynamic_cert_enabled = config.has_fallback();

        // Update the configuration
        {
            let mut current_config = self.config.write().unwrap();
            *current_config = Arc::new(config);
        }

        // Update cached values
        self.client_cert_required.store(client_cert_required, Ordering::Relaxed);
        self.dynamic_cert_enabled.store(dynamic_cert_enabled, Ordering::Relaxed);

        // Notify listeners
        self.notify_listeners(event);

        Ok(())
    }

    /// Reload configuration from a file
    fn reload_config<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        info!("Reloading configuration from {}", path.display());

        // Load configuration from file
        let source = FileSource::new(path);
        let file_config = ConfigSource::load(&source)?;

        // Get the current configuration
        let current_config = self.get_config();

        // Create a new configuration by merging the current and file configurations
        let mut new_config = current_config.as_ref().clone();

        // Merge with file config (file config has priority over current config)
        new_config = new_config.merge(&file_config, ValueSource::File);

        // Update the configuration file path
        new_config.config_file = Some(path.to_path_buf());

        // Update the configuration
        self.update_config(new_config, ConfigChangeEvent::Reloaded)
    }

    /// Add a configuration change listener
    fn add_listener<F>(&self, listener: F) -> Result<()>
    where
        F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.listeners.write().unwrap();
        listeners.push(Box::new(listener));
        Ok(())
    }

    /// Notify all listeners of a configuration change
    fn notify_listeners(&self, event: ConfigChangeEvent) {
        let listeners = self.listeners.read().unwrap();
        for listener in listeners.iter() {
            listener(event);
        }
    }

    /// Check if client certificate is required
    fn is_client_cert_required(&self) -> bool {
        self.client_cert_required.load(Ordering::Relaxed)
    }

    /// Check if dynamic certificate selection is enabled
    fn is_dynamic_cert_enabled(&self) -> bool {
        self.dynamic_cert_enabled.load(Ordering::Relaxed)
    }

    /// Get the buffer size
    fn get_buffer_size(&self) -> usize {
        self.get_config().buffer_size()
    }

    /// Get the connection timeout
    fn get_connection_timeout(&self) -> u64 {
        self.get_config().connection_timeout()
    }
}

// Global instance
static CONFIG_MANAGER: Lazy<ConfigManager> = Lazy::new(|| {
    ConfigManager::new()
});

/// Initialize the global configuration
///
/// This function initializes the global configuration with the provided configuration.
/// It should be called once at application startup.
pub fn initialize(config: ProxyConfig) -> Result<()> {
    // Log the configuration being initialized
    log::info!("Initializing global configuration");
    log::info!("Listen address: {}", config.listen());
    log::info!("Target address: {}", config.target());
    log::info!("Log level: {}", config.log_level());
    log::info!("Certificate mode: {}", if config.has_fallback() { "Dynamic" } else { "Single" });

    if let Some(file) = &config.config_file {
        log::info!("Configuration file: {}", file.display());
    } else {
        log::info!("No configuration file specified");
    }

    // Log all configuration values
    log::info!("Configuration values:");
    if let Some(listen) = config.values.listen {
        log::info!("  listen: {}", listen);
    }
    if let Some(target) = config.values.target {
        log::info!("  target: {}", target);
    }
    if let Some(ref log_level) = config.values.log_level {
        log::info!("  log_level: {}", log_level);
    }
    if let Some(ref cert) = config.values.cert {
        log::info!("  cert: {}", cert.display());
    }
    if let Some(ref fallback_cert) = config.values.fallback_cert {
        log::info!("  fallback_cert: {}", fallback_cert.display());
    }

    // Update the global configuration
    CONFIG_MANAGER.update_config(config, ConfigChangeEvent::Updated)
}

/// Get the current global configuration
///
/// This function returns a clone of the current global configuration.
pub fn get_config() -> Arc<ProxyConfig> {
    CONFIG_MANAGER.get_config()
}

/// Update the global configuration
///
/// This function updates the global configuration with the provided configuration.
/// It validates the configuration before updating.
pub fn update_config(config: ProxyConfig) -> Result<()> {
    CONFIG_MANAGER.update_config(config, ConfigChangeEvent::Updated)
}

/// Reload configuration from a file
///
/// This function reloads the configuration from the specified file,
/// merges it with the current configuration, and updates the global configuration.
pub fn reload_config<P: AsRef<Path>>(path: P) -> Result<()> {
    CONFIG_MANAGER.reload_config(path)
}

/// Add a configuration change listener
///
/// This function adds a listener that will be called when the configuration changes.
pub fn add_listener<F>(listener: F) -> Result<()>
where
    F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
{
    CONFIG_MANAGER.add_listener(listener)
}

/// Check if client certificate is required
///
/// This function returns true if client certificate verification is required.
pub fn is_client_cert_required() -> bool {
    CONFIG_MANAGER.is_client_cert_required()
}

/// Check if dynamic certificate selection is enabled
///
/// This function returns true if dynamic certificate selection is enabled
/// (i.e., fallback certificates are configured).
pub fn is_dynamic_cert_enabled() -> bool {
    CONFIG_MANAGER.is_dynamic_cert_enabled()
}

/// Get the buffer size
///
/// This function returns the buffer size from the current configuration.
pub fn get_buffer_size() -> usize {
    CONFIG_MANAGER.get_buffer_size()
}

/// Get the connection timeout
///
/// This function returns the connection timeout from the current configuration.
pub fn get_connection_timeout() -> u64 {
    CONFIG_MANAGER.get_connection_timeout()
}

/// Save the current configuration to a file
///
/// This function saves the current configuration to the specified file path.
/// This is useful for persisting configuration changes made via Admin API.
pub fn save_config<P: AsRef<Path>>(path: P) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let config = CONFIG_MANAGER.get_config();
    let path = path.as_ref();

    // Serialize configuration to JSON
    let json = serde_json::to_string_pretty(&config.values)?;

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    log::info!("Configuration saved to {}", path.display());

    Ok(())
}
