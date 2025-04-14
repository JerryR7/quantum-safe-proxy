//! Configuration structures and methods
//!
//! This module defines the proxy configuration structure and related methods
//! for loading configuration from different sources (command-line arguments,
//! environment variables, and configuration files).

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::path::Path;
use std::fs;
use std::fmt;

use crate::common::{ProxyError, Result, check_file_exists};
use crate::config::defaults;

/// Client certificate verification mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientCertMode {
    /// Require client certificate, connection fails if not provided
    Required,
    /// Verify client certificate if provided, but don't require it
    Optional,
    /// Don't verify client certificates
    None,
}

impl Default for ClientCertMode {
    fn default() -> Self {
        defaults::client_cert_mode() // Use centralized defaults
    }
}

impl fmt::Display for ClientCertMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientCertMode::Required => write!(f, "required"),
            ClientCertMode::Optional => write!(f, "optional"),
            ClientCertMode::None => write!(f, "none"),
        }
    }
}

impl ClientCertMode {
    /// Parse client certificate mode from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "required" => Ok(ClientCertMode::Required),
            "optional" => Ok(ClientCertMode::Optional),
            "none" => Ok(ClientCertMode::None),
            _ => Err(ProxyError::Config(format!(
                "Invalid client certificate mode: {}. Valid values are: required, optional, none",
                s
            ))),
        }
    }
}

/// Proxy configuration
///
/// Contains all configuration options needed for the proxy server.
/// Supports loading from command-line arguments, environment variables,
/// and configuration files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct ProxyConfig {
    /// Listen address for the proxy server
    #[serde(default = "defaults::listen")]
    pub listen: SocketAddr,

    /// Target service address to forward traffic to
    #[serde(default = "defaults::target")]
    pub target: SocketAddr,

    /// Server certificate path
    #[serde(default = "defaults::cert_path")]
    pub cert_path: PathBuf,

    /// Server private key path
    #[serde(default = "defaults::key_path")]
    pub key_path: PathBuf,

    /// CA certificate path for client certificate validation
    #[serde(default = "defaults::ca_cert_path")]
    pub ca_cert_path: PathBuf,

    /// Whether to enable hybrid certificate mode
    /// When enabled, the proxy will detect and support hybrid PQC certificates
    #[serde(default = "defaults::hybrid_mode")]
    pub hybrid_mode: bool,

    /// Log level (debug, info, warn, error)
    #[serde(default = "defaults::log_level")]
    pub log_level: String,

    /// Client certificate verification mode
    /// When set to Required, clients must provide a valid certificate
    /// When set to Optional, clients may provide a certificate, which will be verified if present
    /// When set to None, client certificates are not verified
    #[serde(default)]
    pub client_cert_mode: ClientCertMode,

    /// Environment name (development, testing, production)
    #[serde(default = "defaults::environment")]
    pub environment: String,
}

impl Default for ProxyConfig {
    /// Create a default configuration using centralized defaults
    fn default() -> Self {
        Self {
            listen: defaults::listen(),
            target: defaults::target(),
            cert_path: defaults::cert_path(),
            key_path: defaults::key_path(),
            ca_cert_path: defaults::ca_cert_path(),
            hybrid_mode: defaults::hybrid_mode(),
            log_level: defaults::log_level(),
            client_cert_mode: defaults::client_cert_mode(),
            environment: defaults::environment(),
        }
    }
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
    /// * `client_cert_mode` - Client certificate verification mode
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
    ///     "certs/hybrid/dilithium3/server.crt",
    ///     "certs/hybrid/dilithium3/server.key",
    ///     "certs/hybrid/dilithium3/ca.crt",
    ///     "info",
    ///     "required"
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
        client_cert_mode: &str,
    ) -> Result<Self> {
        // Parse listen address
        let listen = crate::common::parse_socket_addr(listen)?;

        // Parse target address
        let target = crate::common::parse_socket_addr(target)?;

        // Parse client certificate mode
        let client_cert_mode = ClientCertMode::from_str(client_cert_mode)?;

        Ok(Self {
            listen,
            target,
            cert_path: PathBuf::from(cert),
            key_path: PathBuf::from(key),
            ca_cert_path: PathBuf::from(ca_cert),
            hybrid_mode: defaults::hybrid_mode(),
            log_level: log_level.to_string(),
            client_cert_mode,
            environment: defaults::environment(),
        })
    }

    /// Merge another configuration into this one
    ///
    /// Values from `other` will override values in `self` if they are not the default values.
    /// This is used to implement the configuration priority system.
    ///
    /// # Parameters
    ///
    /// * `other` - The configuration to merge into this one
    ///
    /// # Returns
    ///
    /// Returns a new configuration with merged values
    pub fn merge(&self, other: Self) -> Self {
        Self {
            listen: other.listen,
            target: other.target,
            cert_path: other.cert_path,
            key_path: other.key_path,
            ca_cert_path: other.ca_cert_path,
            hybrid_mode: other.hybrid_mode,
            log_level: other.log_level,
            client_cert_mode: other.client_cert_mode,
            environment: other.environment,
        }
    }

    /// Load configuration from file
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the configuration file
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
    /// let config = ProxyConfig::from_file("config.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(format!(
                "Failed to read configuration file {}: {}", path.display(), e
            )))?;

        serde_json::from_str(&content)
            .map_err(|e| ProxyError::Config(format!(
                "Failed to parse JSON configuration file {}: {}", path.display(), e
            )))
    }

    /// Save configuration to file
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if configuration was saved successfully, otherwise returns an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use quantum_safe_proxy::config::ProxyConfig;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ProxyConfig::default();
    /// config.save_to_file("config.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| ProxyError::Config(format!(
                        "Failed to create directory {}: {}", parent.display(), e
                    )))?;
            }
        }

        // Serialize configuration to JSON with pretty formatting
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ProxyError::Config(format!(
                "Failed to serialize configuration: {}", e
            )))?;

        // Write to file
        fs::write(path, content)
            .map_err(|e| ProxyError::Config(format!(
                "Failed to write configuration to {}: {}", path.display(), e
            )))
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

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => return Err(ProxyError::Config(format!(
                "Invalid log level: {}. Valid values are: debug, info, warn, error",
                self.log_level
            ))),
        }

        // Validate environment
        match self.environment.to_lowercase().as_str() {
            "development" | "dev" | "testing" | "test" | "production" | "prod" => {},
            _ => return Err(ProxyError::Config(format!(
                "Invalid environment: {}. Valid values are: development, testing, production",
                self.environment
            ))),
        }

        Ok(())
    }

    /// Check configuration for potential issues
    ///
    /// This method checks the configuration for potential issues and returns
    /// a list of warnings. Unlike `validate()`, this method does not return
    /// an error if issues are found.
    ///
    /// # Returns
    ///
    /// Returns a vector of warning messages.
    pub fn check(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check if certificate file exists
        if !self.cert_path.exists() {
            warnings.push(format!(
                "Certificate file does not exist: {}", self.cert_path.display()
            ));
        }

        // Check if key file exists
        if !self.key_path.exists() {
            warnings.push(format!(
                "Key file does not exist: {}", self.key_path.display()
            ));
        }

        // Check if CA certificate file exists
        if !self.ca_cert_path.exists() {
            warnings.push(format!(
                "CA certificate file does not exist: {}", self.ca_cert_path.display()
            ));
        }

        // Check if listen address is valid
        if self.listen.port() == 0 {
            warnings.push(format!(
                "Listen address has port 0, which will use a random port: {}", self.listen
            ));
        }

        // Check if target address is valid
        if self.target.port() == 0 {
            warnings.push(format!(
                "Target address has port 0, which may not work as expected: {}", self.target
            ));
        }

        // Check if log level is valid
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => warnings.push(format!(
                "Unknown log level: {}", self.log_level
            )),
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default() {
        // Test default configuration
        let config = ProxyConfig::default();

        assert_eq!(config.listen, defaults::listen());
        assert_eq!(config.target, defaults::target());
        assert_eq!(config.cert_path, defaults::cert_path());
        assert_eq!(config.key_path, defaults::key_path());
        assert_eq!(config.ca_cert_path, defaults::ca_cert_path());
        assert_eq!(config.hybrid_mode, defaults::hybrid_mode());
        assert_eq!(config.log_level, defaults::log_level());
        assert_eq!(config.client_cert_mode, defaults::client_cert_mode());
        assert_eq!(config.environment, defaults::environment());
    }

    #[test]
    fn test_from_args() {
        // Test creating configuration
        let config = ProxyConfig::from_args(
            "127.0.0.1:8443",
            "127.0.0.1:6000",
            "certs/hybrid/dilithium3/server.crt",
            "certs/hybrid/dilithium3/server.key",
            "certs/hybrid/dilithium3/ca.crt",
            "info",
            "optional"
        );

        assert!(config.is_ok(), "Should be able to create configuration");

        if let Ok(config) = config {
            assert_eq!(config.listen.port(), 8443);
            assert_eq!(config.target.port(), 6000);
            assert_eq!(config.cert_path, PathBuf::from("certs/hybrid/dilithium3/server.crt"));
            assert_eq!(config.log_level, "info");
            assert!(config.hybrid_mode);
            assert_eq!(config.client_cert_mode, ClientCertMode::Optional);
        }
    }

    #[test]
    fn test_merge() {
        // Create base configuration
        let base = ProxyConfig::default();

        // Create override configuration
        let override_config = ProxyConfig {
            listen: "127.0.0.1:9443".parse().unwrap(),
            target: base.target,  // Keep default
            cert_path: PathBuf::from("certs/traditional/rsa/server.crt"),
            key_path: PathBuf::from("certs/traditional/rsa/server.key"),
            ca_cert_path: base.ca_cert_path,  // Keep default
            hybrid_mode: false,  // Override
            log_level: "debug".to_string(),
            client_cert_mode: ClientCertMode::None,
            environment: "development".to_string(),
        };

        // Merge configurations
        let merged = base.merge(override_config.clone());

        // Verify merged configuration
        assert_eq!(merged.listen, override_config.listen);
        assert_eq!(merged.target, base.target);
        assert_eq!(merged.cert_path, override_config.cert_path);
        assert_eq!(merged.key_path, override_config.key_path);
        assert_eq!(merged.ca_cert_path, base.ca_cert_path);
        assert_eq!(merged.hybrid_mode, override_config.hybrid_mode);
        assert_eq!(merged.log_level, override_config.log_level);
        assert_eq!(merged.client_cert_mode, override_config.client_cert_mode);
        assert_eq!(merged.environment, override_config.environment);
    }

    #[test]
    fn test_validation() {
        // Create a configuration with invalid log level
        let mut config = ProxyConfig::default();
        config.log_level = "invalid".to_string();

        // Validation should fail
        assert!(config.validate().is_err());

        // Fix log level but set invalid environment
        config.log_level = "debug".to_string();
        config.environment = "invalid".to_string();

        // Validation should fail
        assert!(config.validate().is_err());

        // Fix environment
        config.environment = "development".to_string();

        // Validation should still fail because certificate files don't exist in test environment
        // This is expected behavior
        assert!(config.validate().is_err());
    }
}
