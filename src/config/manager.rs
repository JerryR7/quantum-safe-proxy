//! Configuration manager
//!
//! This module provides a singleton configuration manager that handles
//! configuration loading, access, and hot reloading.
//! Optimized for lightweight and high-performance operation.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock, Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use once_cell::sync::{OnceCell, Lazy};
use log::info;

use crate::common::{Result, ProxyError};
use super::config::ProxyConfig;
use super::defaults;

/// Configuration change event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChangeEvent {
    /// Configuration has been reloaded
    Reloaded,
    /// Configuration has been updated
    Updated,
}

/// Configuration change listener
pub type ConfigChangeListener = Box<dyn Fn(ConfigChangeEvent) + Send + Sync>;

// Cached configuration values for high-performance access
static BUFFER_SIZE: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(defaults::buffer_size()));
static CONNECTION_TIMEOUT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(defaults::connection_timeout()));

/// Configuration manager singleton
pub struct ConfigManager {
    /// Current configuration (wrapped in Arc to reduce clone cost)
    config: RwLock<Arc<ProxyConfig>>,
    /// Configuration file path
    config_path: Mutex<Option<PathBuf>>,
    /// Configuration change listeners
    listeners: Mutex<Vec<ConfigChangeListener>>,
}

/// Global configuration manager instance
static CONFIG_MANAGER: OnceCell<ConfigManager> = OnceCell::new();

impl ConfigManager {
    /// Initialize the configuration manager
    ///
    /// This function initializes the configuration manager with the specified
    /// configuration. It should be called once at application startup.
    ///
    /// # Arguments
    ///
    /// * `config` - Initial configuration
    /// * `config_path` - Optional path to the configuration file
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization was successful, an error otherwise
    pub fn initialize(config: ProxyConfig, config_path: Option<impl AsRef<Path>>) -> Result<()> {
        let config_path = config_path.map(|p| p.as_ref().to_path_buf());

        // Initialize cached values
        BUFFER_SIZE.store(config.buffer_size, Ordering::Relaxed);
        CONNECTION_TIMEOUT.store(config.connection_timeout, Ordering::Relaxed);

        let manager = ConfigManager {
            config: RwLock::new(Arc::new(config)),
            config_path: Mutex::new(config_path),
            listeners: Mutex::new(Vec::new()),
        };

        CONFIG_MANAGER.set(manager)
            .map_err(|_| ProxyError::Config("Configuration manager already initialized".to_string()))?;

        Ok(())
    }

    /// Get the configuration manager instance
    ///
    /// # Returns
    ///
    /// The configuration manager instance
    fn instance() -> &'static ConfigManager {
        CONFIG_MANAGER.get()
            .expect("Configuration manager not initialized. Call ConfigManager::initialize() first.")
    }

    /// Get the current configuration
    ///
    /// This function returns a clone of the current configuration.
    ///
    /// # Returns
    ///
    /// The current configuration
    pub fn get_config() -> Result<Arc<ProxyConfig>> {
        let config = Self::instance().config.read()
            .map_err(|e| ProxyError::Config(format!("Failed to read configuration: {}", e)))?;

        // Return Arc clone instead of full config clone for better performance
        Ok(Arc::clone(&config))
    }

    /// Get buffer size without acquiring a lock (high performance)
    pub fn get_buffer_size() -> usize {
        BUFFER_SIZE.load(Ordering::Relaxed)
    }

    /// Get connection timeout without acquiring a lock (high performance)
    pub fn get_connection_timeout() -> u64 {
        CONNECTION_TIMEOUT.load(Ordering::Relaxed)
    }

    /// Update the configuration
    ///
    /// This function updates the current configuration and notifies all listeners.
    ///
    /// # Arguments
    ///
    /// * `config` - New configuration
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update was successful, an error otherwise
    pub fn update_config(config: ProxyConfig) -> Result<()> {
        // Validate the new configuration
        config.validate()?;

        // Update cached values for fast access
        BUFFER_SIZE.store(config.buffer_size, Ordering::Relaxed);
        CONNECTION_TIMEOUT.store(config.connection_timeout, Ordering::Relaxed);

        // Update the configuration
        {
            let mut current = Self::instance().config.write()
                .map_err(|e| ProxyError::Config(format!("Failed to write configuration: {}", e)))?;

            *current = Arc::new(config);
        }

        // Notify listeners
        Self::notify_listeners(ConfigChangeEvent::Updated)?;

        Ok(())
    }

    /// Reload the configuration from file
    ///
    /// This function reloads the configuration from the specified file or the
    /// file that was used to initialize the configuration manager.
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path to the configuration file
    ///
    /// # Returns
    ///
    /// `Ok(())` if the reload was successful, an error otherwise
    pub fn reload_config<P: AsRef<Path>>(path: Option<P>) -> Result<()> {
        // Get the configuration file path
        let config_path = if let Some(path) = path {
            path.as_ref().to_path_buf()
        } else {
            let path_guard = Self::instance().config_path.lock()
                .map_err(|e| ProxyError::Config(format!("Failed to lock configuration path: {}", e)))?;

            match &*path_guard {
                Some(path) => path.clone(),
                None => return Err(ProxyError::Config("No configuration file path specified".to_string())),
            }
        };

        // Load the new configuration
        info!("Reloading configuration from file: {}", config_path.display());
        let new_config = ProxyConfig::from_file(&config_path)?;

        // Get the current configuration
        let current_config = Self::get_config()?;

        // Merge the configurations
        let merged_config = current_config.as_ref().merge(new_config);

        // Validate the merged configuration
        merged_config.validate()?;

        // Update cached values for fast access
        BUFFER_SIZE.store(merged_config.buffer_size, Ordering::Relaxed);
        CONNECTION_TIMEOUT.store(merged_config.connection_timeout, Ordering::Relaxed);

        // Update the configuration
        {
            let mut config = Self::instance().config.write()
                .map_err(|e| ProxyError::Config(format!("Failed to write configuration: {}", e)))?;

            *config = Arc::new(merged_config);
        }

        // Update the configuration file path
        {
            let mut path_guard = Self::instance().config_path.lock()
                .map_err(|e| ProxyError::Config(format!("Failed to lock configuration path: {}", e)))?;

            *path_guard = Some(config_path);
        }

        // Notify listeners
        Self::notify_listeners(ConfigChangeEvent::Reloaded)?;

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Add a configuration change listener
    ///
    /// This function adds a listener that will be notified when the configuration changes.
    ///
    /// # Arguments
    ///
    /// * `listener` - Configuration change listener
    ///
    /// # Returns
    ///
    /// `Ok(())` if the listener was added successfully, an error otherwise
    pub fn add_listener<F>(listener: F) -> Result<()>
    where
        F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
    {
        let mut listeners = Self::instance().listeners.lock()
            .map_err(|e| ProxyError::Config(format!("Failed to lock listeners: {}", e)))?;

        listeners.push(Box::new(listener));

        Ok(())
    }

    /// Notify all listeners of a configuration change
    ///
    /// # Arguments
    ///
    /// * `event` - Configuration change event
    ///
    /// # Returns
    ///
    /// `Ok(())` if all listeners were notified successfully, an error otherwise
    fn notify_listeners(event: ConfigChangeEvent) -> Result<()> {
        let listeners = Self::instance().listeners.lock()
            .map_err(|e| ProxyError::Config(format!("Failed to lock listeners: {}", e)))?;

        for listener in &*listeners {
            listener(event.clone());
        }

        Ok(())
    }
}

/// Initialize the configuration system
///
/// This function initializes the configuration system with the specified
/// configuration. It should be called once at application startup.
///
/// # Arguments
///
/// * `args` - Command line arguments
/// * `config_file` - Optional path to the configuration file
///
/// # Returns
///
/// The loaded configuration, or an error if initialization failed
pub fn initialize(args: Vec<String>, config_file: Option<&str>) -> Result<ProxyConfig> {
    // Load configuration from all sources
    let config = super::load_config(args, config_file)?;

    // Initialize the configuration manager
    ConfigManager::initialize(config.clone(), config_file)?;

    Ok(config)
}

/// Get the current configuration
///
/// This function returns a clone of the current configuration.
///
/// # Returns
///
/// The current configuration
pub fn get_config() -> Result<Arc<ProxyConfig>> {
    ConfigManager::get_config()
}

/// Get buffer size without acquiring a lock (high performance)
pub fn get_buffer_size() -> usize {
    ConfigManager::get_buffer_size()
}

/// Get connection timeout without acquiring a lock (high performance)
pub fn get_connection_timeout() -> u64 {
    ConfigManager::get_connection_timeout()
}

/// Update the configuration
///
/// This function updates the current configuration and notifies all listeners.
///
/// # Arguments
///
/// * `config` - New configuration
///
/// # Returns
///
/// `Ok(())` if the update was successful, an error otherwise
pub fn update_config(config: ProxyConfig) -> Result<()> {
    ConfigManager::update_config(config)
}

/// Reload the configuration from file
///
/// This function reloads the configuration from the specified file or the
/// file that was used to initialize the configuration manager.
///
/// # Arguments
///
/// * `path` - Optional path to the configuration file
///
/// # Returns
///
/// `Ok(())` if the reload was successful, an error otherwise
pub fn reload_config<P: AsRef<Path>>(path: Option<P>) -> Result<()> {
    ConfigManager::reload_config(path)
}

/// Add a configuration change listener
///
/// This function adds a listener that will be notified when the configuration changes.
///
/// # Arguments
///
/// * `listener` - Configuration change listener
///
/// # Returns
///
/// `Ok(())` if the listener was added successfully, an error otherwise
pub fn add_listener<F>(listener: F) -> Result<()>
where
    F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
{
    ConfigManager::add_listener(listener)
}
