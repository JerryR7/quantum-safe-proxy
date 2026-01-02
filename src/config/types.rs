//! Configuration types
//!
//! This module contains the main configuration types used throughout the application.

use std::path::{Path, PathBuf};
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use std::collections::HashMap;
use std::ops::Deref;
use serde::{Deserialize, Serialize, Deserializer};
use log::debug;

use crate::config::error::{ConfigError, Result};
use crate::config::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};

/// Client certificate verification mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ClientCertMode {
    /// Require client certificate, connection fails if not provided
    Required,
    /// Verify the client certificate if provided but don't require it
    Optional,
    /// Don't verify client certificates
    None,
}

impl Default for ClientCertMode {
    fn default() -> Self {
        ClientCertMode::Optional
    }
}

impl std::fmt::Display for ClientCertMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientCertMode::Required => write!(f, "required"),
            ClientCertMode::Optional => write!(f, "optional"),
            ClientCertMode::None => write!(f, "none"),
        }
    }
}

impl FromStr for ClientCertMode {
    type Err = ConfigError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "required" => Ok(Self::Required),
            "optional" => Ok(Self::Optional),
            "none" => Ok(Self::None),
            _ => Err(ConfigError::InvalidValue(
                "client_cert_mode".to_string(),
                format!("Invalid client certificate mode: {}. Valid values are: required, optional, none", s)
            )),
        }
    }
}

/// Source of a configuration value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueSource {
    /// Default value
    Default,
    /// From configuration file
    File,
    /// From environment variable
    Environment,
    /// From command line argument
    CommandLine,
    /// From Admin API
    AdminApi,
}

impl std::fmt::Display for ValueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueSource::Default => write!(f, "default"),
            ValueSource::File => write!(f, "file"),
            ValueSource::Environment => write!(f, "environment"),
            ValueSource::CommandLine => write!(f, "command line"),
            ValueSource::AdminApi => write!(f, "admin api"),
        }
    }
}

/// Custom deserializer for socket addresses
fn deserialize_socket_addr<'de, D>(deserializer: D) -> std::result::Result<Option<SocketAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s {
        Some(addr_str) => parse_socket_addr(&addr_str)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// Parse a socket address string
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    // First try to parse as a socket address
    if let Ok(addr) = addr.parse::<SocketAddr>() {
        return Ok(addr);
    }

    // If that fails, try to resolve using ToSocketAddrs
    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr)
            } else {
                Err(ConfigError::InvalidValue(
                    "socket_addr".to_string(),
                    format!("Could not resolve address: {}", addr)
                ))
            }
        }
        Err(e) => Err(ConfigError::InvalidValue(
            "socket_addr".to_string(),
            format!("Invalid socket address '{}': {}", addr, e)
        )),
    }
}

/// Check if a file exists
pub fn check_file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Configuration values
///
/// Contains all configuration values with their optional state.
/// 
/// # Certificate Configuration
/// 
/// The proxy automatically determines the certificate strategy based on which
/// certificates are provided:
/// 
/// - If only `cert`/`key` are provided: Single certificate mode
/// - If both `cert`/`key` and `fallback_cert`/`fallback_key` are provided: 
///   Dynamic mode (automatically selects based on client PQC support)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigValues {
    // --- Network settings ---

    /// Listen address (host:port)
    #[serde(default, deserialize_with = "deserialize_socket_addr")]
    pub listen: Option<SocketAddr>,

    /// Target address (host:port)
    #[serde(default, deserialize_with = "deserialize_socket_addr")]
    pub target: Option<SocketAddr>,

    // --- General settings ---

    /// Log level (error, warn, info, debug, trace)
    #[serde(default)]
    pub log_level: Option<String>,

    /// Client certificate verification mode
    #[serde(default)]
    pub client_cert_mode: Option<ClientCertMode>,

    /// Buffer size for data transfer (in bytes)
    #[serde(default)]
    pub buffer_size: Option<usize>,

    /// Connection timeout in seconds
    #[serde(default)]
    pub connection_timeout: Option<u64>,

    /// OpenSSL installation directory (advanced option)
    /// 
    /// NOTE: This setting primarily affects compile-time linking.
    /// For runtime, use environment variables instead:
    ///   - OPENSSL_DIR
    ///   - LD_LIBRARY_PATH
    /// 
    /// In Docker containers, this is typically not needed as the
    /// environment is pre-configured via Dockerfile ENV.
    #[serde(default)]
    pub openssl_dir: Option<PathBuf>,

    // --- Certificate settings (simplified) ---

    /// Primary certificate path (typically hybrid/PQC certificate)
    #[serde(default, alias = "hybrid_cert")]
    pub cert: Option<PathBuf>,

    /// Primary private key path
    #[serde(default, alias = "hybrid_key")]
    pub key: Option<PathBuf>,

    /// Fallback certificate path for non-PQC clients (traditional RSA/ECDSA)
    #[serde(default, alias = "traditional_cert")]
    pub fallback_cert: Option<PathBuf>,

    /// Fallback private key path
    #[serde(default, alias = "traditional_key")]
    pub fallback_key: Option<PathBuf>,

    /// Client CA certificate path (for client certificate validation)
    #[serde(default, alias = "client_ca_cert_path")]
    pub client_ca_cert: Option<PathBuf>,
}

/// Proxy configuration
///
/// Contains all configuration options needed for the proxy server.
/// Supports loading from command-line arguments, environment variables,
/// and configuration files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConfig {
    /// Configuration values
    pub values: ConfigValues,

    /// Configuration file path
    pub config_file: Option<PathBuf>,

    /// Source tracking for configuration values
    pub sources: HashMap<String, ValueSource>,
}

// Manual implementation of Hash for ProxyConfig that ignores sources
impl std::hash::Hash for ProxyConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.values.hash(state);
        self.config_file.hash(state);
        // Deliberately skip hashing sources as HashMap doesn't implement Hash
    }
}

impl Deref for ProxyConfig {
    type Target = ConfigValues;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl Serialize for ProxyConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.values.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProxyConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = ConfigValues::deserialize(deserializer)?;
        Ok(Self {
            values,
            config_file: None,
            sources: HashMap::new(),
        })
    }
}

impl Default for ConfigValues {
    fn default() -> Self {
        Self {
            // All fields are None by default
            listen: None,
            target: None,
            log_level: None,
            client_cert_mode: None,
            buffer_size: None,
            connection_timeout: None,
            openssl_dir: None,
            cert: None,
            key: None,
            fallback_cert: None,
            fallback_key: None,
            client_ca_cert: None,
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        let mut config = Self {
            values: ConfigValues::default(),
            config_file: None,
            sources: HashMap::new(),
        };

        // Apply default values and track their source
        config.set_default_values();

        config
    }
}

impl ProxyConfig {
    /// Set default values for all configuration options
    pub fn set_default_values(&mut self) {
        // Network settings
        if self.values.listen.is_none() {
            self.values.listen = Some(parse_socket_addr(LISTEN_STR).unwrap_or_else(|_| {
                panic!("Invalid default listen address: {}", LISTEN_STR)
            }));
            self.sources.insert("listen".to_string(), ValueSource::Default);
        }

        if self.values.target.is_none() {
            self.values.target = Some(parse_socket_addr(TARGET_STR).unwrap_or_else(|_| {
                panic!("Invalid default target address: {}", TARGET_STR)
            }));
            self.sources.insert("target".to_string(), ValueSource::Default);
        }

        // General settings
        if self.values.log_level.is_none() {
            self.values.log_level = Some(LOG_LEVEL_STR.to_string());
            self.sources.insert("log_level".to_string(), ValueSource::Default);
        }

        if self.values.client_cert_mode.is_none() {
            self.values.client_cert_mode = Some(ClientCertMode::default());
            self.sources.insert("client_cert_mode".to_string(), ValueSource::Default);
        }

        if self.values.buffer_size.is_none() {
            self.values.buffer_size = Some(8192);
            self.sources.insert("buffer_size".to_string(), ValueSource::Default);
        }

        if self.values.connection_timeout.is_none() {
            self.values.connection_timeout = Some(30);
            self.sources.insert("connection_timeout".to_string(), ValueSource::Default);
        }

        // Certificate settings
        if self.values.cert.is_none() {
            self.values.cert = Some(PathBuf::from(CERT_PATH_STR));
            self.sources.insert("cert".to_string(), ValueSource::Default);
        }

        if self.values.key.is_none() {
            self.values.key = Some(PathBuf::from(KEY_PATH_STR));
            self.sources.insert("key".to_string(), ValueSource::Default);
        }

        if self.values.client_ca_cert.is_none() {
            self.values.client_ca_cert = Some(PathBuf::from(CA_CERT_PATH_STR));
            self.sources.insert("client_ca_cert".to_string(), ValueSource::Default);
        }
    }

    /// Load configuration from a specific file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> crate::common::Result<Self> {
        use crate::config::{ENV_PREFIX, builder::ConfigBuilder};
        use log::{debug, warn};

        let path = path.as_ref();

        if !path.exists() {
            warn!("Configuration file not found: {}", path.display());
            warn!("Will use default values unless overridden by environment variables");
        } else {
            debug!("Using configuration file: {}", path.display());
        }

        let config = ConfigBuilder::new()
            .with_defaults()
            .with_file(path)
            .with_env(ENV_PREFIX)
            .build()?;

        debug!("Configuration validated successfully");
        Ok(config)
    }

    /// Auto-detect and load configuration from the best available source
    pub fn auto_load() -> crate::common::Result<Self> {
        let args: Vec<String> = std::env::args().collect();
        crate::config::builder::auto_load(args).map_err(|e| e.into())
    }

    /// Create a new configuration from an existing one
    pub fn from_config(config: ProxyConfig) -> Self {
        config
    }

    /// Get the underlying configuration
    pub fn as_config(&self) -> &ProxyConfig {
        self
    }

    /// Get the source of a configuration value
    pub fn source(&self, name: &str) -> &str {
        match self.sources.get(name) {
            Some(source) => match source {
                ValueSource::Default => "default",
                ValueSource::File => "file",
                ValueSource::Environment => "environment",
                ValueSource::CommandLine => "command line",
                ValueSource::AdminApi => "admin api",
            },
            None => "unknown",
        }
    }

    /// Get the listen address
    pub fn listen(&self) -> SocketAddr {
        self.values.listen.unwrap_or_else(|| {
            parse_socket_addr(LISTEN_STR).expect("Invalid default listen address")
        })
    }

    /// Get the target address
    pub fn target(&self) -> SocketAddr {
        self.values.target.unwrap_or_else(|| {
            parse_socket_addr(TARGET_STR).expect("Invalid default target address")
        })
    }

    /// Get the log level
    pub fn log_level(&self) -> &str {
        self.values.log_level.as_deref().unwrap_or(LOG_LEVEL_STR)
    }

    /// Get the client certificate mode
    pub fn client_cert_mode(&self) -> ClientCertMode {
        self.values.client_cert_mode.unwrap_or_default()
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.values.buffer_size.unwrap_or(8192)
    }

    /// Get the connection timeout
    pub fn connection_timeout(&self) -> u64 {
        self.values.connection_timeout.unwrap_or(30)
    }

    /// Get the OpenSSL directory
    pub fn openssl_dir(&self) -> Option<&Path> {
        self.values.openssl_dir.as_deref()
    }

    /// Get the primary certificate path
    pub fn cert(&self) -> &Path {
        self.values.cert.as_deref().unwrap_or_else(|| Path::new(CERT_PATH_STR))
    }

    /// Get the primary private key path
    pub fn key(&self) -> &Path {
        self.values.key.as_deref().unwrap_or_else(|| Path::new(KEY_PATH_STR))
    }

    /// Get the fallback certificate path (for non-PQC clients)
    pub fn fallback_cert(&self) -> Option<&Path> {
        self.values.fallback_cert.as_deref()
    }

    /// Get the fallback private key path
    pub fn fallback_key(&self) -> Option<&Path> {
        self.values.fallback_key.as_deref()
    }

    /// Get the client CA certificate path
    pub fn client_ca_cert(&self) -> &Path {
        self.values.client_ca_cert.as_deref().unwrap_or_else(|| Path::new(CA_CERT_PATH_STR))
    }

    /// Check if fallback certificates are configured (enables dynamic mode)
    pub fn has_fallback(&self) -> bool {
        self.values.fallback_cert.is_some() && self.values.fallback_key.is_some()
    }

    /// Get the configuration file path
    pub fn config_file(&self) -> Option<&Path> {
        self.config_file.as_deref()
    }

    // --- Backward compatibility aliases ---

    /// Alias for cert() - backward compatibility
    pub fn hybrid_cert(&self) -> &Path {
        self.cert()
    }

    /// Alias for key() - backward compatibility
    pub fn hybrid_key(&self) -> &Path {
        self.key()
    }

    /// Alias for fallback_cert() - backward compatibility
    pub fn traditional_cert(&self) -> &Path {
        self.fallback_cert().unwrap_or_else(|| self.cert())
    }

    /// Alias for fallback_key() - backward compatibility
    pub fn traditional_key(&self) -> &Path {
        self.fallback_key().unwrap_or_else(|| self.key())
    }

    /// Alias for client_ca_cert() - backward compatibility
    pub fn client_ca_cert_path(&self) -> &Path {
        self.client_ca_cert()
    }

    /// Merge two configurations
    pub fn merge(&self, other: &ProxyConfig, source: ValueSource) -> Self {
        let mut result = self.clone();

        macro_rules! merge_field {
            ($field:expr, $name:ident) => {
                if other.values.$name.is_some() {
                    result.values.$name = other.values.$name.clone();
                    result.sources.insert($field.to_string(), source);
                }
            };
        }

        // Network settings
        merge_field!("listen", listen);
        merge_field!("target", target);

        // General settings
        merge_field!("log_level", log_level);
        merge_field!("client_cert_mode", client_cert_mode);
        merge_field!("buffer_size", buffer_size);
        merge_field!("connection_timeout", connection_timeout);
        merge_field!("openssl_dir", openssl_dir);

        // Certificate settings
        merge_field!("cert", cert);
        merge_field!("key", key);
        merge_field!("fallback_cert", fallback_cert);
        merge_field!("fallback_key", fallback_key);
        merge_field!("client_ca_cert", client_ca_cert);

        // Configuration file path
        if let Some(path) = &other.config_file {
            result.config_file = Some(path.clone());
        }

        result
    }

    /// Log the configuration
    pub fn log(&self) {
        debug!("=== Configuration ===");
        debug!("Network settings:");
        debug!("  Listen address: {} (from {})", self.listen(), self.source("listen"));
        debug!("  Target address: {} (from {})", self.target(), self.source("target"));

        debug!("General settings:");
        debug!("  Log level: {} (from {})", self.log_level(), self.source("log_level"));
        debug!("  Client certificate mode: {} (from {})", self.client_cert_mode(), self.source("client_cert_mode"));
        debug!("  Buffer size: {} bytes (from {})", self.buffer_size(), self.source("buffer_size"));
        debug!("  Connection timeout: {} seconds (from {})", self.connection_timeout(), self.source("connection_timeout"));

        if let Some(dir) = self.openssl_dir() {
            debug!("  OpenSSL directory: {} (from {})", dir.display(), self.source("openssl_dir"));
        }

        debug!("Certificate settings:");
        debug!("  Mode: {}", if self.has_fallback() { "Dynamic (auto-select)" } else { "Single" });
        debug!("  Primary certificate: {} (from {})", self.cert().display(), self.source("cert"));
        debug!("  Primary key: {} (from {})", self.key().display(), self.source("key"));

        if let Some(cert) = self.fallback_cert() {
            debug!("  Fallback certificate: {} (from {})", cert.display(), self.source("fallback_cert"));
        }
        if let Some(key) = self.fallback_key() {
            debug!("  Fallback key: {} (from {})", key.display(), self.source("fallback_key"));
        }

        debug!("  Client CA certificate: {} (from {})", self.client_ca_cert().display(), self.source("client_ca_cert"));

        if let Some(file) = self.config_file() {
            debug!("  Configuration file: {}", file.display());
        }

        debug!("=====================");
    }
}
