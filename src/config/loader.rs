//! Configuration loading functionality
//!
//! This module provides functionality for loading configuration from different sources
//! such as files, environment variables, and command-line arguments.

use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::str::FromStr;
use std::net::SocketAddr;

use crate::common::{ProxyError, Result};
use crate::config::defaults;
use crate::config::{ProxyConfig, parse_socket_addr};
use crate::config::merger::ConfigMerger;
use crate::tls::strategy::CertStrategy;
use crate::config::strategy::CertificateStrategyBuilder;

/// Trait for loading configuration from different sources
pub trait ConfigLoader {
    /// Load configuration from a file
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> where Self: Sized;

    /// Load configuration from environment variables
    fn from_env() -> Result<Self> where Self: Sized;

    /// Create configuration from command line arguments
    fn from_args(
        listen: &str,
        target: &str,
        cert: &str,
        key: &str,
        ca_cert: &str,
        log_level: &str,
        client_cert_mode: &str,
        buffer_size: usize,
        connection_timeout: u64,
    ) -> Result<Self> where Self: Sized;

    /// Auto-detect and load configuration from the best available source
    fn auto_load() -> Result<Self> where Self: Sized;
}

impl ConfigLoader for ProxyConfig {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let path_display = path.display();

        // Check if file exists and can be opened
        fs::File::open(path)
            .map_err(|e| ProxyError::Config(format!("Failed to open config file {}: {}", path_display, e)))?;

        // Read the file content
        let content = fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(format!("Failed to read config file {}: {}", path_display, e)))?;

        // Deserialize with error mapping
        serde_json::from_str(&content)
            .map_err(|e| ProxyError::Config(format!("Failed to parse JSON config file {}: {}", path_display, e)))
    }

    fn from_env() -> Result<Self> {
        use crate::config::defaults::ENV_PREFIX;

        // Optimized closure to get environment variables with the prefix
        let get_env = |name: &str| -> Option<String> {
            env::var(format!("{}{}", ENV_PREFIX, name)).ok()
        };

        let mut config = Self::default();
        let mut has_changes = false;

        // Helper function to update a field with a parser
        fn update_field<T, F>(
            get_env: &impl Fn(&str) -> Option<String>,
            env_name: &str,
            field: &mut T,
            parser: F,
            has_changes: &mut bool,
        ) where
            F: FnOnce(&str) -> Result<T>,
        {
            if let Some(value) = get_env(env_name) {
                if let Ok(parsed) = parser(&value) {
                    *field = parsed;
                    *has_changes = true;
                }
            }
        }

        // Network settings
        update_field(&get_env, "LISTEN", &mut config.listen, parse_socket_addr, &mut has_changes);

        // Target is now parsed as SocketAddr
        if let Some(value) = get_env("TARGET") {
            config.target = value.parse::<SocketAddr>().expect("Invalid target address");
            has_changes = true;
        }

        // Certificate strategy
        update_field(&get_env, "STRATEGY", &mut config.strategy, |s| s.parse(), &mut has_changes);

        // Certificate paths (direct string conversion)
        if let Some(value) = get_env("TRADITIONAL_CERT") {
            config.traditional_cert = PathBuf::from(value);
            has_changes = true;
        }
        if let Some(value) = get_env("TRADITIONAL_KEY") {
            config.traditional_key = PathBuf::from(value);
            has_changes = true;
        }
        if let Some(value) = get_env("HYBRID_CERT") {
            config.hybrid_cert = PathBuf::from(value);
            has_changes = true;
        }
        if let Some(value) = get_env("HYBRID_KEY") {
            config.hybrid_key = PathBuf::from(value);
            has_changes = true;
        }
        if let Some(value) = get_env("CLIENT_CA_CERT") {
            config.client_ca_cert_path = PathBuf::from(value);
            has_changes = true;
        }

        // Optional PQC-only certificate paths
        if let Some(value) = get_env("PQC_ONLY_CERT") {
            config.pqc_only_cert = Some(PathBuf::from(value));
            has_changes = true;
        }
        if let Some(value) = get_env("PQC_ONLY_KEY") {
            config.pqc_only_key = Some(PathBuf::from(value));
            has_changes = true;
        }

        // Other settings
        if let Some(value) = get_env("LOG_LEVEL") {
            config.log_level = value;
            has_changes = true;
        }
        update_field(&get_env, "CLIENT_CERT_MODE", &mut config.client_cert_mode, |s| s.parse(), &mut has_changes);

        // Parse numeric values
        if let Some(value) = get_env("BUFFER_SIZE") {
            if let Ok(size) = value.parse::<usize>() {
                config.buffer_size = size;
                has_changes = true;
            }
        }
        if let Some(value) = get_env("CONNECTION_TIMEOUT") {
            if let Ok(timeout) = value.parse::<u64>() {
                config.connection_timeout = timeout;
                has_changes = true;
            }
        }

        // OpenSSL directory
        if let Some(value) = get_env("OPENSSL_DIR") {
            config.openssl_dir = Some(PathBuf::from(value));
            has_changes = true;
        }

        // Return default configuration if no changes were made
        if !has_changes {
            return Ok(Self::default());
        }

        Ok(config)
    }

    fn auto_load() -> Result<Self> {
        use log::{info, debug};

        // Start with the default configuration
        let mut config = Self::default();

        // Log configuration (only in debug mode to reduce overhead)
        debug!("Starting with default configuration");

        // Check if the default config file exists before attempting to load it
        let default_config_path = defaults::DEFAULT_CONFIG_FILE;
        if Path::new(default_config_path).exists() {
            info!("Loading configuration from {}", default_config_path);
            match Self::from_file(default_config_path) {
                Ok(file_config) => {
                    config = config.merge(file_config);
                    debug!("Merged configuration from file");
                }
                Err(e) => {
                    debug!("Failed to load configuration from file: {}", e);
                }
            }
        }

        // Load from environment variables (only if there are actual changes)
        match Self::from_env() {
            Ok(env_config) if env_config != Self::default() => {
                info!("Applying configuration from environment variables");
                config = config.merge(env_config);
            }
            _ => debug!("No environment variable configuration found or applied"),
        }

        // Check for command line arguments in the environment
        // This is a simplified approach - in a real implementation, you would parse actual command line arguments
        if std::env::args().len() > 1 {
            info!("Applying configuration from command line arguments");
            // In a real implementation, this would parse command line arguments
            // For now, we'll just log that command line arguments were detected
        }

        Ok(config)
    }

    fn from_args(
        listen: &str,
        target: &str,
        cert: &str,
        key: &str,
        ca_cert: &str,
        log_level: &str,
        client_cert_mode: &str,
        buffer_size: usize,
        connection_timeout: u64,
    ) -> Result<Self> {
        use crate::config::{ClientCertMode, CertStrategyType};

        // Create a default configuration first
        let mut config = Self::default();

        // Update with provided values
        config.listen = parse_socket_addr(listen)?;
        config.target = target.parse::<SocketAddr>().expect("Invalid target address");
        config.log_level = log_level.to_string();
        config.client_cert_mode = ClientCertMode::from_str(client_cert_mode)?;
        config.buffer_size = buffer_size;
        config.connection_timeout = connection_timeout;

        // Set certificate paths
        config.hybrid_cert = PathBuf::from(cert);
        config.hybrid_key = PathBuf::from(key);
        config.client_ca_cert_path = PathBuf::from(ca_cert);

        // Use Dynamic strategy by default
        config.strategy = CertStrategyType::Dynamic;

        Ok(config)
    }
}

impl ProxyConfig {
    pub fn load() -> Result<Self> {
        // Attempt to load configuration from a file
        if let Ok(file_config) = Self::from_file("config.json") {
            return Ok(file_config);
        }

        // Fallback to loading from environment variables
        Self::from_env()
    }

    pub fn build_cert_strategy(&self) -> Result<CertStrategy> {
        CertificateStrategyBuilder::build_cert_strategy(self)
    }
}
