//! Configuration structures and methods
//!
//! This module defines the proxy configuration structure and related methods
//! for loading configuration from different sources (command-line arguments,
//! environment variables, and configuration files).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use crate::common::{ProxyError, Result};
use crate::config::defaults;

/// Check if a file exists
fn check_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ProxyError::FileNotFound(format!("{:?}", path)));
    }

    if !path.is_file() {
        return Err(ProxyError::Config(format!("Path is not a file: {:?}", path)));
    }

    Ok(())
}

/// Parse a socket address
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    // Try direct parsing first
    if let Ok(socket_addr) = SocketAddr::from_str(addr) {
        return Ok(socket_addr);
    }

    // Try using ToSocketAddrs trait
    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr)
            } else {
                Err(ProxyError::Network(format!("Failed to parse address: {}", addr)))
            }
        }
        Err(e) => Err(ProxyError::Network(format!("Failed to parse address {}: {}", addr, e))),
    }
}

/// Client certificate verification mode
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ClientCertMode {
    /// Require client certificate, connection fails if not provided
    Required,
    /// Verify the client certificate if provided but don't require it
    Optional,
    /// Don't verify client certificates
    None,
}

// Custom deserialization implementation to make it case-insensitive
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

    /// Log level (error, warn, info, debug, trace)
    #[serde(default = "defaults::log_level")]
    pub log_level: String,

    /// Client certificate verification mode
    /// When set to Required, clients must provide a valid certificate
    /// When set to Optional, clients may provide a certificate, which will be verified if present
    /// When set to None, client certificates are not verified
    #[serde(default)]
    pub client_cert_mode: ClientCertMode,

    /// Buffer size for data transfer (in bytes)
    /// Larger buffers may improve throughput but increase memory usage
    #[serde(default = "defaults::buffer_size")]
    pub buffer_size: usize,

    /// Connection timeout in seconds
    /// How long to wait for connections to establish before giving up
    #[serde(default = "defaults::connection_timeout")]
    pub connection_timeout: u64,

    /// OpenSSL installation directory
    /// If specified, this will be used to locate OpenSSL libraries and headers
    /// This is useful when OpenSSL is installed in a non-standard location
    #[serde(default = "defaults::openssl_dir")]
    pub openssl_dir: Option<PathBuf>,

    /// Path to classic (RSA/ECDSA) cert PEM
    #[serde(default = "defaults::cert_path")]
    pub classic_cert: PathBuf,

    /// Path to classic private key PEM
    #[serde(default = "defaults::key_path")]
    pub classic_key: PathBuf,

    /// Always use SigAlgs strategy: auto-select cert by client signature_algorithms
    #[serde(default)]
    pub use_sigalgs: bool,
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
            log_level: defaults::log_level(),
            client_cert_mode: defaults::client_cert_mode(),
            buffer_size: defaults::buffer_size(),
            connection_timeout: defaults::connection_timeout(),
            openssl_dir: defaults::openssl_dir(),
            classic_cert: defaults::classic_cert_path(),
            classic_key: defaults::classic_key_path(),
            use_sigalgs: defaults::use_sigalgs(),
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
    /// 2. Configuration file (config.json)
    /// 3. Environment variables
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
    /// let config = ProxyConfig::auto_load()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn auto_load() -> Result<Self> {
        use log::{info, debug};

        // Start with the default configuration
        let mut config = Self::default();

        // Log configuration (only in debug mode to reduce overhead)
        if log::log_enabled!(log::Level::Debug) {
            debug!("Starting with default configuration");
        }

        // Use Path::try_exists to check if the file exists before attempting to load it
        let default_config_path = defaults::DEFAULT_CONFIG_FILE;
        if Path::new(default_config_path).exists() {
            info!("Loading configuration from {}", default_config_path);
            if let Ok(file_config) = Self::from_file(default_config_path) {
                config = config.merge(file_config);
                debug!("Merged configuration from file");
            }
        }

        // from_env
        if let Ok(env_config) = Self::from_env() {
            info!("Applying configuration from environment variables");
            config = config.merge(env_config);
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
        use crate::config::defaults::ENV_PREFIX;
        use std::env;

        // Use a closure to get environment variables with the prefix
        let get_env = |name: &str| -> Option<String> {
            let full_name = format!("{}{}", ENV_PREFIX, name);
            env::var(&full_name).ok()
        };

        let mut config = Self::default();
        let mut has_changes = false;

        macro_rules! update_config {
            // For Result<T, E> parsers
            ($env_name:expr, $field:expr, $parser:expr) => {
                if let Some(value) = get_env($env_name) {
                    if let Ok(parsed) = $parser(&value) {
                        $field = parsed;
                        has_changes = true;
                    }
                }
            };
            // For Option<T> parsers (used for openssl_dir)
            ($env_name:expr, $field:expr, option_fn:expr) => {
                if let Some(value) = get_env($env_name) {
                    $field = option_fn(&value);
                    has_changes = true;
                }
            };
            // For direct string conversion
            ($env_name:expr, $field:expr) => {
                if let Some(value) = get_env($env_name) {
                    $field = value.into();
                    has_changes = true;
                }
            };
        }

        // Update configuration fields from environment variables
        update_config!("LISTEN", config.listen, |v: &str| parse_socket_addr(v));
        update_config!("TARGET", config.target, |v: &str| parse_socket_addr(v));
        update_config!("CERT", config.cert_path);
        update_config!("KEY", config.key_path);
        update_config!("CA_CERT", config.ca_cert_path);
        update_config!("LOG_LEVEL", config.log_level);
        update_config!("CLIENT_CERT_MODE", config.client_cert_mode, |v: &str| ClientCertMode::from_str(v));
        update_config!("BUFFER_SIZE", config.buffer_size, |v: &str| v.parse::<usize>());
        update_config!("CONNECTION_TIMEOUT", config.connection_timeout, |v: &str| v.parse::<u64>());
        update_config!("CLASSIC_CERT", config.classic_cert);
        update_config!("CLASSIC_KEY", config.classic_key);
        update_config!("USE_SIGALGS", config.use_sigalgs, |v: &str| v.parse::<bool>());
        // Use the option_fn variant for openssl_dir
        if let Some(value) = get_env("OPENSSL_DIR") {
            config.openssl_dir = Some(PathBuf::from(value));
            has_changes = true;
        }

        // Record whether there are changes for quick comparison in auto_load
        if !has_changes {
            // No changes, return default configuration
            return Ok(Self::default());
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
    /// * `buffer_size` - Buffer size for data transfer (in bytes)
    /// * `connection_timeout` - Connection timeout in seconds
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
    ///     "required",
    ///     8192,
    ///     30
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
        buffer_size: usize,
        connection_timeout: u64,
    ) -> Result<Self> {
        // Parse listen address
        let listen = parse_socket_addr(listen)?;

        // Parse target address
        let target = parse_socket_addr(target)?;

        // Parse client certificate mode
        let client_cert_mode = ClientCertMode::from_str(client_cert_mode)?;

        // Convert paths to PathBuf
        let cert_path = PathBuf::from(cert);
        let key_path = PathBuf::from(key);
        let ca_cert_path = PathBuf::from(ca_cert);

        Ok(Self {
            listen,
            target,
            cert_path: cert_path.clone(),
            key_path: key_path.clone(),
            ca_cert_path,
            log_level: log_level.to_string(),
            client_cert_mode,
            buffer_size,
            connection_timeout,
            openssl_dir: None,  // Default to None for openssl_dir
            classic_cert: defaults::classic_cert_path(),
            classic_key: defaults::classic_key_path(),
            use_sigalgs: defaults::use_sigalgs(),
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

        // Only clone values that are not the default
        // This ensures that only explicitly set values are overridden
        let default = Self::default();

        Self {
            // For network addresses, directly override
            listen: other.listen,
            target: other.target,

            // For file paths, only override if not the default
            cert_path: if other.cert_path != default.cert_path {
                other.cert_path.clone()
            } else {
                self.cert_path.clone()
            },
            key_path: if other.key_path != default.key_path {
                other.key_path.clone()
            } else {
                self.key_path.clone()
            },
            ca_cert_path: if other.ca_cert_path != default.ca_cert_path {
                other.ca_cert_path.clone()
            } else {
                self.ca_cert_path.clone()
            },

            // For strings, only override if not the default
            log_level: if other.log_level != default.log_level {
                other.log_level.clone()
            } else {
                self.log_level.clone()
            },

            // For enums, directly override
            client_cert_mode: other.client_cert_mode,
            buffer_size: other.buffer_size,
            connection_timeout: other.connection_timeout,

            // For Option<PathBuf>, only override if Some
            openssl_dir: if other.openssl_dir.is_some() {
                other.openssl_dir.clone()
            } else {
                self.openssl_dir.clone()
            },

            // For new certificate paths, only override if not the default
            classic_cert: if other.classic_cert != default.classic_cert {
                other.classic_cert.clone()
            } else {
                self.classic_cert.clone()
            },
            classic_key: if other.classic_key != default.classic_key {
                other.classic_key.clone()
            } else {
                self.classic_key.clone()
            },

            // For boolean flags, directly override
            use_sigalgs: other.use_sigalgs,
        }
    }

    /// Load configuration from a file
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

        // Use a buffered reader for efficient reading
        let file = fs::File::open(path).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to open configuration file {}: {}",
                path.display(),
                e
            ))
        })?;

        let reader = std::io::BufReader::new(file);

        // Use serde_json to deserialize the configuration
        serde_json::from_reader(reader).map_err(|e| {
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
            if !parent.is_dir() {
                fs::create_dir_all(parent).map_err(|e| {
                    ProxyError::Config(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        // Use a buffered writer for efficient writing
        let file = fs::File::create(path).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to create file {}: {}",
                path.display(),
                e
            ))
        })?;

        let writer = std::io::BufWriter::new(file);

        // Use serde_json to serialize the configuration
        serde_json::to_writer_pretty(writer, self).map_err(|e| {
            ProxyError::Config(format!("Failed to serialize configuration to JSON: {}", e))
        })?;

        Ok(())
    }

    /// Validate configuration
    ///
    /// Checks if certificate files exist and other configuration is valid.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if configuration is valid, otherwise returns an error.
    pub fn validate(&self) -> Result<()> {
        // Check if listen address is valid
        if self.listen.port() == 0 {
            return Err(ProxyError::Config(format!(
                "Invalid listen port: 0 (random port not supported)"
            )));
        }

        // Check if target address is valid
        if self.target.port() == 0 {
            return Err(ProxyError::Config("Invalid target port: 0 (random port not supported)".to_string()));
        }

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

        // Check if certificate file exists
        check_file_exists(&self.cert_path).map_err(|_| {
            ProxyError::Config(format!(
                "Certificate file does not exist or is invalid: {:?}",
                self.cert_path
            ))
        })?;

        // Check if key file exists
        check_file_exists(&self.key_path).map_err(|_| {
            ProxyError::Config(format!(
                "Key file does not exist or is invalid: {:?}",
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

        // Check if a certificate file exists
        if !self.cert_path.exists() {
            warnings.push(format!(
                "Certificate file does not exist: {}",
                self.cert_path.display()
            ));
        }

        // Check if the key file exists
        if !self.key_path.exists() {
            warnings.push(format!(
                "Key file does not exist: {}",
                self.key_path.display()
            ));
        }

        // Check if the CA certificate file exists
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

        // Check if the target address is valid
        if self.target.port() == 0 {
            warnings.push(format!(
                "Target address has port 0, which may not work as expected: {}",
                self.target
            ));
        }

        // Check if the log level is valid
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => warnings.push(format!("Unknown log level: {}", self.log_level)),
        }

        warnings
    }

    /// Reload configuration from a file
    ///
    /// This method reloads the configuration from the specified file and merges it with
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

        // Load a new configuration from a file
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

    /// Build the SigAlgs strategy if requested, else default to single classic.
    pub fn build_cert_strategy(&self) -> Result<crate::tls::strategy::CertStrategy> {
        use crate::tls::strategy::CertStrategy;
        if self.use_sigalgs {
            Ok(CertStrategy::SigAlgs {
                classic: (self.classic_cert.clone(), self.classic_key.clone()),
                hybrid: (self.cert_path.clone(), self.key_path.clone()),
            })
        } else {
            Ok(CertStrategy::Single {
                cert: self.cert_path.clone(),
                key: self.key_path.clone(),
            })
        }
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
        assert_eq!(config.log_level, defaults::log_level());
        assert_eq!(config.client_cert_mode, defaults::client_cert_mode());
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
            8192,                                  // buffer_size
            30                              // connection_timeout
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
            ca_cert_path: base_ca_cert_path.clone(), // Use the cloned path
            log_level: "debug".to_string(),
            client_cert_mode: ClientCertMode::None,
            buffer_size: 4096,                      // Test different buffer size
            connection_timeout: 60,                 // Test different connection timeout
            openssl_dir: None,                      // No OpenSSL directory specified
            classic_cert: PathBuf::from("certs/traditional/rsa/server.crt"),
            classic_key: PathBuf::from("certs/traditional/rsa/server.key"),
            use_sigalgs: false,
        };

        // Merge configurations
        let merged = base.merge(override_config.clone());

        // Verify merged configuration
        assert_eq!(merged.listen, override_config.listen);
        assert_eq!(merged.target, base.target);
        assert_eq!(merged.cert_path, override_config.cert_path);
        assert_eq!(merged.key_path, override_config.key_path);
        assert_eq!(merged.ca_cert_path, base_ca_cert_path); // Use the cloned path
        assert_eq!(merged.log_level, override_config.log_level);
        assert_eq!(merged.client_cert_mode, override_config.client_cert_mode);
    }

    #[test]
    fn test_validation() {
        // Create a configuration with an invalid log level
        let mut config = ProxyConfig::default();
        config.log_level = "invalid".to_string();

        // Validation should fail due to invalid log level
        assert!(config.validate().is_err());

        // Fix log level
        config.log_level = "debug".to_string();

        // Set certificate paths to non-existent files
        config.cert_path = PathBuf::from("non_existent_cert.crt");
        config.key_path = PathBuf::from("non_existent_key.key");
        config.ca_cert_path = PathBuf::from("non_existent_ca.crt");

        // Validation should fail because certificate files don't exist
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
