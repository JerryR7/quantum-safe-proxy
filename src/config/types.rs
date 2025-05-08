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

/// Certificate strategy type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CertStrategyType {
    /// Use a single certificate for all connections
    Single,
    /// Use signature algorithms extension to select certificate
    SigAlgs,
    /// Dynamically select certificate based on client hello
    Dynamic,
}

impl Default for CertStrategyType {
    fn default() -> Self {
        CertStrategyType::Dynamic
    }
}

impl std::fmt::Display for CertStrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertStrategyType::Single => write!(f, "single"),
            CertStrategyType::SigAlgs => write!(f, "sigalgs"),
            CertStrategyType::Dynamic => write!(f, "dynamic"),
        }
    }
}

impl FromStr for CertStrategyType {
    type Err = ConfigError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "single" => Ok(Self::Single),
            "sigalgs" => Ok(Self::SigAlgs),
            "dynamic" => Ok(Self::Dynamic),
            _ => Err(ConfigError::InvalidValue(
                "strategy".to_string(),
                format!("Invalid certificate strategy: {}. Valid values are: single, sigalgs, dynamic", s)
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
}

impl std::fmt::Display for ValueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueSource::Default => write!(f, "default"),
            ValueSource::File => write!(f, "file"),
            ValueSource::Environment => write!(f, "environment"),
            ValueSource::CommandLine => write!(f, "command line"),
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

    /// OpenSSL installation directory
    #[serde(default)]
    pub openssl_dir: Option<PathBuf>,

    // --- Certificate strategy settings ---

    /// Certificate strategy type (single, sigalgs, dynamic)
    #[serde(default)]
    pub strategy: Option<CertStrategyType>,

    /// Traditional certificate path (for Dynamic and SigAlgs strategies)
    #[serde(default)]
    pub traditional_cert: Option<PathBuf>,

    /// Traditional private key path (for Dynamic and SigAlgs strategies)
    #[serde(default)]
    pub traditional_key: Option<PathBuf>,

    /// Hybrid certificate path (for Dynamic and SigAlgs strategies)
    #[serde(default)]
    pub hybrid_cert: Option<PathBuf>,

    /// Hybrid private key path (for Dynamic and SigAlgs strategies)
    #[serde(default)]
    pub hybrid_key: Option<PathBuf>,

    /// PQC-only certificate path (for Dynamic strategy, optional)
    #[serde(default)]
    pub pqc_only_cert: Option<PathBuf>,

    /// PQC-only private key path (for Dynamic strategy, optional)
    #[serde(default)]
    pub pqc_only_key: Option<PathBuf>,

    /// Client CA certificate path (for client certificate validation)
    #[serde(default)]
    pub client_ca_cert_path: Option<PathBuf>,
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
            strategy: None,
            traditional_cert: None,
            traditional_key: None,
            hybrid_cert: None,
            hybrid_key: None,
            pqc_only_cert: None,
            pqc_only_key: None,
            client_ca_cert_path: None,
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

        // Certificate strategy settings
        if self.values.strategy.is_none() {
            self.values.strategy = Some(CertStrategyType::default());
            self.sources.insert("strategy".to_string(), ValueSource::Default);
        }

        if self.values.traditional_cert.is_none() {
            self.values.traditional_cert = Some(PathBuf::from("certs/traditional/rsa/server.crt"));
            self.sources.insert("traditional_cert".to_string(), ValueSource::Default);
        }

        if self.values.traditional_key.is_none() {
            self.values.traditional_key = Some(PathBuf::from("certs/traditional/rsa/server.key"));
            self.sources.insert("traditional_key".to_string(), ValueSource::Default);
        }

        if self.values.hybrid_cert.is_none() {
            self.values.hybrid_cert = Some(PathBuf::from(CERT_PATH_STR));
            self.sources.insert("hybrid_cert".to_string(), ValueSource::Default);
        }

        if self.values.hybrid_key.is_none() {
            self.values.hybrid_key = Some(PathBuf::from(KEY_PATH_STR));
            self.sources.insert("hybrid_key".to_string(), ValueSource::Default);
        }

        if self.values.client_ca_cert_path.is_none() {
            self.values.client_ca_cert_path = Some(PathBuf::from(CA_CERT_PATH_STR));
            self.sources.insert("client_ca_cert_path".to_string(), ValueSource::Default);
        }
    }

    /// Get the listen address
    pub fn listen(&self) -> SocketAddr {
        // Log the actual value for debugging
        if let Some(addr) = self.values.listen {
            debug!("Using configured listen address: {}", addr);
            return addr;
        }

        // Use default value
        let default_addr = parse_socket_addr(LISTEN_STR).unwrap_or_else(|_| {
            panic!("Invalid default listen address: {}", LISTEN_STR)
        });
        debug!("Using default listen address: {}", default_addr);
        default_addr
    }

    /// Get the target address
    pub fn target(&self) -> SocketAddr {
        self.values.target.unwrap_or_else(|| {
            parse_socket_addr(TARGET_STR).unwrap_or_else(|_| {
                panic!("Invalid default target address: {}", TARGET_STR)
            })
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
    pub fn openssl_dir(&self) -> Option<&PathBuf> {
        self.values.openssl_dir.as_ref()
    }

    /// Get the certificate strategy type
    pub fn strategy(&self) -> CertStrategyType {
        self.values.strategy.unwrap_or_default()
    }

    /// Get the traditional certificate path
    pub fn traditional_cert(&self) -> &Path {
        self.values.traditional_cert.as_deref().unwrap_or_else(|| {
            Path::new("certs/traditional/rsa/server.crt")
        })
    }

    /// Get the traditional private key path
    pub fn traditional_key(&self) -> &Path {
        self.values.traditional_key.as_deref().unwrap_or_else(|| {
            Path::new("certs/traditional/rsa/server.key")
        })
    }

    /// Get the hybrid certificate path
    pub fn hybrid_cert(&self) -> &Path {
        self.values.hybrid_cert.as_deref().unwrap_or_else(|| {
            Path::new(CERT_PATH_STR)
        })
    }

    /// Get the hybrid private key path
    pub fn hybrid_key(&self) -> &Path {
        self.values.hybrid_key.as_deref().unwrap_or_else(|| {
            Path::new(KEY_PATH_STR)
        })
    }

    /// Get the PQC-only certificate path
    pub fn pqc_only_cert(&self) -> Option<&Path> {
        self.values.pqc_only_cert.as_deref()
    }

    /// Get the PQC-only private key path
    pub fn pqc_only_key(&self) -> Option<&Path> {
        self.values.pqc_only_key.as_deref()
    }

    /// Get the client CA certificate path
    pub fn client_ca_cert_path(&self) -> &Path {
        self.values.client_ca_cert_path.as_deref().unwrap_or_else(|| {
            Path::new(CA_CERT_PATH_STR)
        })
    }

    /// Get the configuration file path
    pub fn config_file(&self) -> Option<&Path> {
        self.config_file.as_deref()
    }

    /// Load configuration from a file
    pub fn from_file(path: &str) -> crate::config::error::Result<Self> {
        crate::config::builder::ConfigBuilder::new()
            .with_defaults()
            .with_file(path)
            .build()
    }

    /// Auto-load configuration from all sources
    pub fn auto_load() -> crate::config::error::Result<Self> {
        crate::config::builder::auto_load(std::env::args().collect())
    }

    /// Create a ProxyConfig from a Config
    pub fn from_config(config: Self) -> Self {
        config
    }

    /// Get the underlying Config
    pub fn as_config(&self) -> &Self {
        self
    }

    // check method moved to ConfigValidator trait implementation

    // build_cert_strategy method removed - now using tls::strategy_builder::build_cert_strategy instead

    /// Get the source of a configuration value
    pub fn source(&self, name: &str) -> ValueSource {
        self.sources.get(name).copied().unwrap_or(ValueSource::Default)
    }

    /// Merge two configurations
    ///
    /// Values from `other` will override values in `self` only if they are Some.
    /// The source of each value is tracked.
    pub fn merge(&self, other: &Self, source: ValueSource) -> Self {
        debug!("Merging configurations from source: {:?}", source);
        let mut result = self.clone();

        // Helper macro to merge a field
        macro_rules! merge_field {
            ($name:expr, $field:ident) => {
                if let Some(value) = &other.values.$field {
                    debug!("Merging field '{}' from {:?}: {:?}", $name, source, value);
                    result.values.$field = Some(value.clone());
                    result.sources.insert($name.to_string(), source);
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

        // Certificate strategy settings
        merge_field!("strategy", strategy);
        merge_field!("traditional_cert", traditional_cert);
        merge_field!("traditional_key", traditional_key);
        merge_field!("hybrid_cert", hybrid_cert);
        merge_field!("hybrid_key", hybrid_key);
        merge_field!("pqc_only_cert", pqc_only_cert);
        merge_field!("pqc_only_key", pqc_only_key);
        merge_field!("client_ca_cert_path", client_ca_cert_path);

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

        // Log the raw listen address value
        debug!("  Raw listen address value: {:?}", self.values.listen);

        // Log the computed listen address
        let listen_addr = self.listen();
        debug!("  Listen address: {} (from {})", listen_addr, self.source("listen"));
        debug!("  Target address: {} (from {})", self.target(), self.source("target"));

        debug!("General settings:");
        debug!("  Log level: {} (from {})", self.log_level(), self.source("log_level"));
        debug!("  Client certificate mode: {} (from {})", self.client_cert_mode(), self.source("client_cert_mode"));
        debug!("  Buffer size: {} bytes (from {})", self.buffer_size(), self.source("buffer_size"));
        debug!("  Connection timeout: {} seconds (from {})", self.connection_timeout(), self.source("connection_timeout"));

        if let Some(dir) = self.openssl_dir() {
            debug!("  OpenSSL directory: {} (from {})", dir.display(), self.source("openssl_dir"));
        }

        debug!("Certificate strategy settings:");
        debug!("  Strategy: {:?} (from {})", self.strategy(), self.source("strategy"));
        debug!("  Traditional certificate: {} (from {})", self.traditional_cert().display(), self.source("traditional_cert"));
        debug!("  Traditional key: {} (from {})", self.traditional_key().display(), self.source("traditional_key"));
        debug!("  Hybrid certificate: {} (from {})", self.hybrid_cert().display(), self.source("hybrid_cert"));
        debug!("  Hybrid key: {} (from {})", self.hybrid_key().display(), self.source("hybrid_key"));

        if let Some(cert) = self.pqc_only_cert() {
            debug!("  PQC-only certificate: {} (from {})", cert.display(), self.source("pqc_only_cert"));
        }

        if let Some(key) = self.pqc_only_key() {
            debug!("  PQC-only key: {} (from {})", key.display(), self.source("pqc_only_key"));
        }

        debug!("  Client CA certificate: {} (from {})", self.client_ca_cert_path().display(), self.source("client_ca_cert_path"));

        if let Some(file) = self.config_file() {
            debug!("  Configuration file: {}", file.display());
        }

        debug!("=====================");
    }
}
