//! Configuration structures and methods
//!
//! This module defines the proxy configuration structure and related methods
//! for loading configuration from different sources (command-line arguments,
//! environment variables, and configuration files).

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::fs;
use std::env;

use crate::common::{ProxyError, Result, check_file_exists, parse_socket_addr};

/// Proxy configuration
///
/// Contains all configuration options needed for the proxy server.
/// Supports loading from command-line arguments, environment variables,
/// and configuration files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Listen address for the proxy server
    pub listen: SocketAddr,

    /// Target service address to forward traffic to
    pub target: SocketAddr,

    /// Server certificate path
    pub cert_path: PathBuf,

    /// Server private key path
    pub key_path: PathBuf,

    /// CA certificate path for client certificate validation
    pub ca_cert_path: PathBuf,

    /// Whether to enable hybrid certificate mode
    /// When enabled, the proxy will detect and support hybrid PQC certificates
    #[serde(default = "default_hybrid_mode")]
    pub hybrid_mode: bool,

    /// Log level (debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_hybrid_mode() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

impl ProxyConfig {
    /// Create configuration from command line arguments
    ///
    /// # Parameters
    ///
    /// * `listen` - Listen address
    /// * `target` - Target service address
    /// * `cert` - Server certificate path
    /// * `key` - Server private key path
    /// * `ca_cert` - CA certificate path
    /// * `log_level` - Log level
    ///
    /// # Returns
    ///
    /// Returns the configuration result
    ///
    /// # Example
    ///
    /// ```
    /// # use quantum_safe_proxy::config::ProxyConfig;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ProxyConfig::from_args(
    ///     "127.0.0.1:8443",
    ///     "127.0.0.1:6000",
    ///     "certs/server.crt",
    ///     "certs/server.key",
    ///     "certs/ca.crt",
    ///     "info"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_args(
        listen: &str,
        target: &str,
        cert: &str,
        key: &str,
        ca_cert: &str,
        log_level: &str,
    ) -> Result<Self> {
        // Parse listen address
        let listen = parse_socket_addr(listen)?;

        // Parse target address
        let target = parse_socket_addr(target)?;

        Ok(Self {
            listen,
            target,
            cert_path: PathBuf::from(cert),
            key_path: PathBuf::from(key),
            ca_cert_path: PathBuf::from(ca_cert),
            hybrid_mode: true,
            log_level: log_level.to_string(),
        })
    }

    /// Load configuration from a file
    ///
    /// # Parameters
    ///
    /// * `path` - Configuration file path
    ///
    /// # Returns
    ///
    /// Returns the configuration result
    pub fn from_file(path: &str) -> Result<Self> {
        let config_str = fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(format!("Failed to read configuration file: {}", e)))?;

        let config: Self = serde_json::from_str(&config_str)
            .map_err(|e| ProxyError::Config(format!("Failed to parse configuration file: {}", e)))?;

        Ok(config)
    }

    /// Load configuration from environment variables
    ///
    /// Uses the following environment variables:
    /// - `QUANTUM_SAFE_PROXY_LISTEN` - Listen address
    /// - `QUANTUM_SAFE_PROXY_TARGET` - Target service address
    /// - `QUANTUM_SAFE_PROXY_CERT` - Server certificate path
    /// - `QUANTUM_SAFE_PROXY_KEY` - Server private key path
    /// - `QUANTUM_SAFE_PROXY_CA_CERT` - CA certificate path
    /// - `QUANTUM_SAFE_PROXY_LOG_LEVEL` - Log level
    /// - `QUANTUM_SAFE_PROXY_HYBRID_MODE` - Whether to enable hybrid certificate mode
    ///
    /// # Returns
    ///
    /// Returns the configuration result
    pub fn from_env() -> Result<Self> {
        let listen = env::var("QUANTUM_SAFE_PROXY_LISTEN")
            .unwrap_or_else(|_| "0.0.0.0:8443".to_string());

        let target = env::var("QUANTUM_SAFE_PROXY_TARGET")
            .unwrap_or_else(|_| "127.0.0.1:6000".to_string());

        let cert = env::var("QUANTUM_SAFE_PROXY_CERT")
            .unwrap_or_else(|_| "certs/server.crt".to_string());

        let key = env::var("QUANTUM_SAFE_PROXY_KEY")
            .unwrap_or_else(|_| "certs/server.key".to_string());

        let ca_cert = env::var("QUANTUM_SAFE_PROXY_CA_CERT")
            .unwrap_or_else(|_| "certs/ca.crt".to_string());

        let log_level = env::var("QUANTUM_SAFE_PROXY_LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());

        let hybrid_mode = env::var("QUANTUM_SAFE_PROXY_HYBRID_MODE")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true);

        Self::from_args(
            &listen,
            &target,
            &cert,
            &key,
            &ca_cert,
            &log_level,
        ).map(|mut config| {
            config.hybrid_mode = hybrid_mode;
            config
        })
    }

    /// Validate configuration
    ///
    /// Checks if certificate files exist and other configuration is valid.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if configuration is valid, otherwise returns an error.
    pub fn validate(&self) -> Result<()> {
        // Check if certificate file exists
        check_file_exists(&self.cert_path)
            .map_err(|_| ProxyError::Config(format!(
                "Certificate file does not exist or is invalid: {:?}",
                self.cert_path
            )))?;

        // Check if private key file exists
        check_file_exists(&self.key_path)
            .map_err(|_| ProxyError::Config(format!(
                "Private key file does not exist or is invalid: {:?}",
                self.key_path
            )))?;

        // Check if CA certificate file exists
        check_file_exists(&self.ca_cert_path)
            .map_err(|_| ProxyError::Config(format!(
                "CA certificate file does not exist or is invalid: {:?}",
                self.ca_cert_path
            )))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_args() {
        // Test creating configuration
        let config = ProxyConfig::from_args(
            "127.0.0.1:8443",
            "127.0.0.1:6000",
            "certs/server.crt",
            "certs/server.key",
            "certs/ca.crt",
            "info",
        );

        assert!(config.is_ok(), "Should be able to create configuration");

        if let Ok(config) = config {
            assert_eq!(config.listen.port(), 8443);
            assert_eq!(config.target.port(), 6000);
            assert_eq!(config.cert_path, PathBuf::from("certs/server.crt"));
            assert_eq!(config.log_level, "info");
            assert!(config.hybrid_mode);
        }
    }

    #[test]
    fn test_from_env() {
        // Set environment variables
        env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "127.0.0.1:9443");
        env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:7000");

        // Test loading configuration from environment variables
        let config = ProxyConfig::from_env();
        assert!(config.is_ok(), "Should be able to load configuration from environment variables");

        if let Ok(config) = config {
            assert_eq!(config.listen.port(), 9443);
            assert_eq!(config.target.port(), 7000);
        }

        // Clean up environment variables
        env::remove_var("QUANTUM_SAFE_PROXY_LISTEN");
        env::remove_var("QUANTUM_SAFE_PROXY_TARGET");
    }
}
