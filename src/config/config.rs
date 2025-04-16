//! Configuration structures and methods
//!
//! This module defines the proxy configuration structure and related methods
//! for loading configuration from different sources (command-line arguments,
//! environment variables, and configuration files).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use crate::common::{ProxyError, Result, check_file_exists};
use crate::config::defaults;

/// Client certificate verification mode
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ClientCertMode {
    /// Require client certificate, connection fails if not provided
    Required,
    /// Verify client certificate if provided, but don't require it
    Optional,
    /// Don't verify client certificates
    None,
}

// 自定義反序列化實現，使其對大小寫不敏感
impl<'de> Deserialize<'de> for ClientCertMode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ClientCertMode::from_str(&s).map_err(serde::de::Error::custom)
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    // TLS 相關設定完全由系統檢測決定，不再在設定檔中提供
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

// Implement AsRef<ProxyConfig> for ProxyConfig
impl AsRef<ProxyConfig> for ProxyConfig {
    fn as_ref(&self) -> &ProxyConfig {
        self
    }
}

impl ProxyConfig {
    /// Auto-detect and load configuration from the best available source
    ///
    /// This method tries to load configuration from the following sources in order:
    /// 1. Default configuration
    /// 2. Configuration file (config.json or config.<environment>.json)
    /// 3. Environment variables
    ///
    /// # Parameters
    ///
    /// * `environment` - Optional environment name (development, testing, production)
    ///
    /// # Returns
    ///
    /// Returns the configuration result
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quantum_safe_proxy::config::ProxyConfig;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Load configuration from the best available source
    /// let config = ProxyConfig::auto_load(Some("development"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn auto_load(environment: Option<&str>) -> Result<Self> {
        use log::{info, debug};

        // Start with default configuration
        let mut config = Self::default();
        debug!("Starting with default configuration");

        // Try to load from configuration file
        let env_name = environment.unwrap_or(&config.environment);

        // Check for environment-specific configuration file
        let env_config_path = format!("config.{}.json", env_name);
        if Path::new(&env_config_path).exists() {
            info!("Loading environment-specific configuration from {}", env_config_path);
            match Self::from_file(&env_config_path) {
                Ok(env_config) => {
                    config = config.merge(env_config);
                    debug!("Merged environment-specific configuration");
                },
                Err(e) => {
                    log::warn!("Failed to load environment configuration file: {}", e);
                }
            }
        } else {
            debug!("No environment-specific configuration file found at {}", env_config_path);
        }

        // Check for default configuration file
        let default_config_path = defaults::DEFAULT_CONFIG_FILE;
        if Path::new(default_config_path).exists() {
            info!("Loading configuration from {}", default_config_path);
            match Self::from_file(default_config_path) {
                Ok(file_config) => {
                    config = config.merge(file_config);
                    debug!("Merged default configuration file");
                },
                Err(e) => {
                    log::warn!("Failed to load default configuration file: {}", e);
                }
            }
        } else {
            debug!("No default configuration file found at {}", default_config_path);
        }

        // Try to load from environment variables
        match Self::from_env() {
            Ok(env_config) => {
                // Only merge if any environment variables were actually set
                if env_config != Self::default() {
                    info!("Loading configuration from environment variables");
                    config = config.merge(env_config);
                    debug!("Merged environment variables configuration");
                } else {
                    debug!("No configuration found in environment variables");
                }
            },
            Err(e) => {
                log::warn!("Failed to load configuration from environment variables: {}", e);
            }
        }

        Ok(config)
    }

    /// Load configuration from environment variables
    ///
    /// This method loads configuration from environment variables with the prefix
    /// defined in `defaults::ENV_PREFIX` ("QUANTUM_SAFE_PROXY_").
    ///
    /// # Returns
    ///
    /// Returns the configuration result
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quantum_safe_proxy::config::ProxyConfig;
    /// # use std::env;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Set environment variables
    /// env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "0.0.0.0:9443");
    /// env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:7000");
    ///
    /// // Load configuration from environment variables
    /// let config = ProxyConfig::from_env()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<Self> {
        use crate::common::parse_socket_addr;
        use crate::config::defaults::ENV_PREFIX;
        use std::env;

        // Helper function to get environment variable with prefix
        let get_env = |name: &str| -> Option<String> {
            let full_name = format!("{}{}", ENV_PREFIX, name);
            env::var(&full_name).ok()
        };

        // Start with default configuration
        let mut config = Self::default();

        // Update configuration from environment variables
        if let Some(listen) = get_env("LISTEN") {
            config.listen = parse_socket_addr(&listen)?;
        }

        if let Some(target) = get_env("TARGET") {
            config.target = parse_socket_addr(&target)?;
        }

        if let Some(cert) = get_env("CERT") {
            config.cert_path = cert.into();
        }

        if let Some(key) = get_env("KEY") {
            config.key_path = key.into();
        }

        if let Some(ca_cert) = get_env("CA_CERT") {
            config.ca_cert_path = ca_cert.into();
        }

        if let Some(hybrid_mode) = get_env("HYBRID_MODE") {
            config.hybrid_mode = hybrid_mode.to_lowercase() == "true";
        }

        if let Some(log_level) = get_env("LOG_LEVEL") {
            config.log_level = log_level;
        }

        if let Some(client_cert_mode) = get_env("CLIENT_CERT_MODE") {
            config.client_cert_mode = ClientCertMode::from_str(&client_cert_mode)?;
        }

        if let Some(env_name) = get_env("ENVIRONMENT") {
            config.environment = env_name;
        }

        Ok(config)
    }

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
    pub fn merge(&self, other: impl AsRef<Self>) -> Self {
        let other = other.as_ref();
        Self {
            listen: other.listen,
            target: other.target,
            cert_path: other.cert_path.clone(),
            key_path: other.key_path.clone(),
            ca_cert_path: other.ca_cert_path.clone(),
            hybrid_mode: other.hybrid_mode,
            log_level: other.log_level.clone(),
            client_cert_mode: other.client_cert_mode,
            environment: other.environment.clone(),
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
        let content = fs::read_to_string(path).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to read configuration file {}: {}",
                path.display(),
                e
            ))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to parse JSON configuration file {}: {}",
                path.display(),
                e
            ))
        })
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
                fs::create_dir_all(parent).map_err(|e| {
                    ProxyError::Config(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        // Serialize configuration to JSON with pretty formatting
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| ProxyError::Config(format!("Failed to serialize configuration: {}", e)))?;

        // Write to file
        fs::write(path, content).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to write configuration to {}: {}",
                path.display(),
                e
            ))
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
        check_file_exists(&self.cert_path).map_err(|_| {
            ProxyError::Config(format!(
                "Certificate file does not exist or is invalid: {:?}",
                self.cert_path
            ))
        })?;

        // Check if private key file exists
        check_file_exists(&self.key_path).map_err(|_| {
            ProxyError::Config(format!(
                "Private key file does not exist or is invalid: {:?}",
                self.key_path
            ))
        })?;

        // Check if CA certificate file exists
        check_file_exists(&self.ca_cert_path).map_err(|_| {
            ProxyError::Config(format!(
                "CA certificate file does not exist or is invalid: {:?}",
                self.ca_cert_path
            ))
        })?;

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => {
                return Err(ProxyError::Config(format!(
                    "Invalid log level: {}. Valid values are: debug, info, warn, error",
                    self.log_level
                )));
            },
        }

        // Validate environment
        match self.environment.to_lowercase().as_str() {
            "development" | "dev" | "testing" | "test" | "production" | "prod" => {},
            _ => {
                return Err(ProxyError::Config(format!(
                    "Invalid environment: {}. Valid values are: development, testing, production",
                    self.environment
                )));
            },
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
                "Certificate file does not exist: {}",
                self.cert_path.display()
            ));
        }

        // Check if key file exists
        if !self.key_path.exists() {
            warnings.push(format!(
                "Key file does not exist: {}",
                self.key_path.display()
            ));
        }

        // Check if CA certificate file exists
        if !self.ca_cert_path.exists() {
            warnings.push(format!(
                "CA certificate file does not exist: {}",
                self.ca_cert_path.display()
            ));
        }

        // Check if listen address is valid
        if self.listen.port() == 0 {
            warnings.push(format!(
                "Listen address has port 0, which will use a random port: {}",
                self.listen
            ));
        }

        // Check if target address is valid
        if self.target.port() == 0 {
            warnings.push(format!(
                "Target address has port 0, which may not work as expected: {}",
                self.target
            ));
        }

        // Check if log level is valid
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => warnings.push(format!("Unknown log level: {}", self.log_level)),
        }

        warnings
    }

    /// Reload configuration from file
    ///
    /// This method reloads configuration from the specified file and merges it with
    /// the current configuration. It then validates the merged configuration.
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// Returns the updated configuration if successful, otherwise returns an error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quantum_safe_proxy::config::ProxyConfig;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut config = ProxyConfig::default();
    /// let updated_config = config.reload_from_file("config.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn reload_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Self> {
        let path = path.as_ref();
        log::info!("Reloading configuration from file: {}", path.display());

        // Load new configuration from file
        let new_config = Self::from_file(path)?;

        // Merge with current configuration
        let merged_config = self.merge(new_config);

        // Validate merged configuration
        merged_config.validate()?;

        // Check for warnings
        let warnings = merged_config.check();
        for warning in &warnings {
            log::warn!("Configuration warning: {}", warning);
        }

        log::info!("Configuration reloaded successfully");
        Ok(merged_config)
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
            "optional",
        );

        assert!(config.is_ok(), "Should be able to create configuration");

        if let Ok(config) = config {
            assert_eq!(config.listen.port(), 8443);
            assert_eq!(config.target.port(), 6000);
            assert_eq!(
                config.cert_path,
                PathBuf::from("certs/hybrid/dilithium3/server.crt")
            );
            assert_eq!(config.log_level, "info");
            assert!(config.hybrid_mode);
            assert_eq!(config.client_cert_mode, ClientCertMode::Optional);
        }
    }

    #[test]
    fn test_merge() {
        // Create base configuration
        let base = ProxyConfig::default();

        // Clone the ca_cert_path before using it
        let base_ca_cert_path = base.ca_cert_path.clone();

        // Create override configuration
        let override_config = ProxyConfig {
            listen: "127.0.0.1:9443".parse().unwrap(),
            target: base.target, // Keep default
            cert_path: PathBuf::from("certs/traditional/rsa/server.crt"),
            key_path: PathBuf::from("certs/traditional/rsa/server.key"),
            ca_cert_path: base_ca_cert_path.clone(), // Use cloned path
            hybrid_mode: false,              // Override
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
        assert_eq!(merged.ca_cert_path, base_ca_cert_path); // Use cloned path
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

    #[test]
    fn test_from_env() {
        // Set environment variables for testing
        env::set_var("QUANTUM_SAFE_PROXY_LISTEN", "127.0.0.1:9443");
        env::set_var("QUANTUM_SAFE_PROXY_TARGET", "127.0.0.1:7000");
        env::set_var("QUANTUM_SAFE_PROXY_CERT", "test/cert.crt");
        env::set_var("QUANTUM_SAFE_PROXY_KEY", "test/key.key");
        env::set_var("QUANTUM_SAFE_PROXY_HYBRID_MODE", "false");
        env::set_var("QUANTUM_SAFE_PROXY_LOG_LEVEL", "debug");
        env::set_var("QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE", "none");

        // Load configuration from environment variables
        let config = ProxyConfig::from_env();
        assert!(
            config.is_ok(),
            "Should be able to load configuration from environment variables"
        );

        if let Ok(config) = config {
            // Verify configuration values
            assert_eq!(config.listen.to_string(), "127.0.0.1:9443");
            assert_eq!(config.target.to_string(), "127.0.0.1:7000");
            assert_eq!(config.cert_path, PathBuf::from("test/cert.crt"));
            assert_eq!(config.key_path, PathBuf::from("test/key.key"));
            assert_eq!(config.hybrid_mode, false);
            assert_eq!(config.log_level, "debug");
            assert_eq!(config.client_cert_mode, ClientCertMode::None);
        }

        // Clean up environment variables
        env::remove_var("QUANTUM_SAFE_PROXY_LISTEN");
        env::remove_var("QUANTUM_SAFE_PROXY_TARGET");
        env::remove_var("QUANTUM_SAFE_PROXY_CERT");
        env::remove_var("QUANTUM_SAFE_PROXY_KEY");
        env::remove_var("QUANTUM_SAFE_PROXY_HYBRID_MODE");
        env::remove_var("QUANTUM_SAFE_PROXY_LOG_LEVEL");
        env::remove_var("QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE");
    }
}
