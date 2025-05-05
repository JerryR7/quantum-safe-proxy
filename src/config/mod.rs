//! Configuration module
//!
//! This module handles application configuration, including loading from
//! different sources (files, environment variables, command line arguments)
//! and validating the configuration.

// Submodules
mod loader;
mod merger;
mod validator;
mod builder;
mod defaults;

// Re-export types and traits
pub use self::loader::ConfigLoader;
pub use self::merger::ConfigMerger;
pub use self::validator::ConfigValidator;
pub use self::builder::CertificateStrategyBuilder;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::common::{ProxyError, Result};

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
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ClientCertMode::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Default for ClientCertMode {
    #[inline]
    fn default() -> Self {
        defaults::client_cert_mode() // Use centralized defaults
    }
}

impl fmt::Display for ClientCertMode {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required => write!(f, "required"),
            Self::Optional => write!(f, "optional"),
            Self::None => write!(f, "none"),
        }
    }
}

impl FromStr for ClientCertMode {
    type Err = ProxyError;

    #[inline]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "required" => Ok(Self::Required),
            "optional" => Ok(Self::Optional),
            "none" => Ok(Self::None),
            _ => Err(ProxyError::Config(format!(
                "Invalid client certificate mode: {}. Valid values are: required, optional, none",
                s
            ))),
        }
    }
}

/// Certificate strategy type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CertStrategyType {
    /// Single certificate strategy
    Single,
    /// SigAlgs strategy
    SigAlgs,
    /// Dynamic strategy
    Dynamic,
}

impl Default for CertStrategyType {
    #[inline]
    fn default() -> Self {
        Self::Dynamic
    }
}

impl FromStr for CertStrategyType {
    type Err = ProxyError;

    #[inline]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "single" => Ok(Self::Single),
            "sigalgs" => Ok(Self::SigAlgs),
            "dynamic" => Ok(Self::Dynamic),
            _ => Err(ProxyError::Config(format!(
                "Invalid certificate strategy: {}. Valid values are: single, sigalgs, dynamic",
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
    // --- Network settings ---

    /// Listen address for the proxy server
    #[serde(default = "defaults::listen")]
    pub listen: SocketAddr,

    /// Target service address to forward traffic to
    #[serde(default = "defaults::target")]
    pub target: SocketAddr,

    // --- General settings ---

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
    #[serde(default = "defaults::openssl_dir", skip_serializing_if = "Option::is_none")]
    pub openssl_dir: Option<PathBuf>,

    // --- Certificate strategy settings ---

    /// Certificate strategy type (single, sigalgs, dynamic)
    #[serde(default)]
    pub strategy: CertStrategyType,

    /// Traditional certificate path (for Dynamic and SigAlgs strategies)
    #[serde(default = "defaults::classic_cert_path")]
    pub traditional_cert: PathBuf,

    /// Traditional private key path (for Dynamic and SigAlgs strategies)
    #[serde(default = "defaults::classic_key_path")]
    pub traditional_key: PathBuf,

    /// Hybrid certificate path (for Dynamic and SigAlgs strategies)
    #[serde(default = "defaults::cert_path")]
    pub hybrid_cert: PathBuf,

    /// Hybrid private key path (for Dynamic and SigAlgs strategies)
    #[serde(default = "defaults::key_path")]
    pub hybrid_key: PathBuf,

    /// PQC-only certificate path (for Dynamic strategy, optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pqc_only_cert: Option<PathBuf>,

    /// PQC-only private key path (for Dynamic strategy, optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pqc_only_key: Option<PathBuf>,

    /// Client CA certificate path (for client certificate validation)
    #[serde(default = "defaults::ca_cert_path")]
    pub client_ca_cert_path: PathBuf,
}

impl Default for ProxyConfig {
    /// Create a default configuration using centralized defaults
    #[inline]
    fn default() -> Self {
        Self {
            // Network settings
            listen: defaults::listen(),
            target: defaults::target(),

            // General settings
            log_level: defaults::log_level(),
            client_cert_mode: defaults::client_cert_mode(),
            buffer_size: defaults::buffer_size(),
            connection_timeout: defaults::connection_timeout(),
            openssl_dir: defaults::openssl_dir(),

            // Certificate strategy settings
            strategy: CertStrategyType::default(),
            traditional_cert: defaults::classic_cert_path(),
            traditional_key: defaults::classic_key_path(),
            hybrid_cert: defaults::cert_path(),
            hybrid_key: defaults::key_path(),
            pqc_only_cert: None,
            pqc_only_key: None,
            client_ca_cert_path: defaults::ca_cert_path(),
        }
    }
}

// Implement AsRef<ProxyConfig> for ProxyConfig to simplify merge operations
impl AsRef<ProxyConfig> for ProxyConfig {
    #[inline]
    fn as_ref(&self) -> &ProxyConfig {
        self
    }
}

/// Parse a socket address with optimized error handling
///
/// First tries direct parsing, then falls back to DNS resolution
#[inline]
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    // Try direct parsing first (most efficient)
    if let Ok(socket_addr) = SocketAddr::from_str(addr) {
        return Ok(socket_addr);
    }

    // Try using ToSocketAddrs trait for hostname resolution
    addr.to_socket_addrs()
        .map_err(|e| ProxyError::Network(format!("Failed to parse address {}: {}", addr, e)))?
        .next()
        .ok_or_else(|| ProxyError::Network(format!("Failed to resolve address: {}", addr)))
}

/// Check if a file exists and is a valid file
///
/// This is an optimized version that uses a single path display call
/// and returns more specific error types.
#[inline]
pub fn check_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ProxyError::FileNotFound(path.display().to_string()));
    }

    if !path.is_file() {
        return Err(ProxyError::Config(format!("Path is not a file: {}", path.display())));
    }

    Ok(())
}

// Global configuration storage
use std::sync::RwLock;
use once_cell::sync::Lazy;

static CONFIG: Lazy<RwLock<ProxyConfig>> = Lazy::new(|| {
    RwLock::new(ProxyConfig::default())
});

/// Log the configuration with source information
fn log_config(config: &ProxyConfig) {
    use log::info;

    // Only log in info level or below
    if !log::log_enabled!(log::Level::Info) {
        return;
    }

    info!("=== Final Configuration ===");

    // Network settings
    info!("Network Settings:");
    info!("  Listen address: {}", config.listen);
    info!("  Target address: {}", config.target);

    // General settings
    info!("General Settings:");
    info!("  Log level: {}", config.log_level);
    info!("  Client certificate mode: {}", config.client_cert_mode);
    info!("  Buffer size: {} bytes", config.buffer_size);
    info!("  Connection timeout: {} seconds", config.connection_timeout);

    // Certificate strategy settings
    info!("Certificate Strategy Settings:");
    info!("  Strategy: {:?}", config.strategy);
    info!("  Traditional certificate: {}", config.traditional_cert.display());
    info!("  Traditional key: {}", config.traditional_key.display());
    info!("  Hybrid certificate: {}", config.hybrid_cert.display());
    info!("  Hybrid key: {}", config.hybrid_key.display());

    if let Some(ref cert) = config.pqc_only_cert {
        info!("  PQC-only certificate: {}", cert.display());
    }

    if let Some(ref key) = config.pqc_only_key {
        info!("  PQC-only key: {}", key.display());
    }

    info!("  Client CA certificate: {}", config.client_ca_cert_path.display());

    // OpenSSL directory
    if let Some(ref dir) = config.openssl_dir {
        info!("  OpenSSL directory: {}", dir.display());
    }

    info!("=========================");
}

// Configuration management functions
pub fn initialize() -> Result<()> {
    // Initialize configuration with default values
    let config = ProxyConfig::auto_load()?;

    // Log the final configuration
    log_config(&config);

    // Set the configuration in the global state
    let mut global_config = CONFIG.write().unwrap();
    *global_config = config;

    Ok(())
}

pub fn get_config() -> ProxyConfig {
    // Retrieve the config from the global state
    let config = CONFIG.read().unwrap();
    config.clone()
}

pub fn update_config(config: ProxyConfig) -> Result<()> {
    // Validate the configuration before updating
    config.validate()?;

    // Update the configuration in the global state
    let mut global_config = CONFIG.write().unwrap();
    *global_config = config;

    Ok(())
}

pub fn reload_config<P: AsRef<Path>>(path: P) -> Result<ProxyConfig> {
    use log::{info, debug};

    let path = path.as_ref();
    info!("Reloading configuration from {}", path.display());

    // Get the current configuration
    let current_config = get_config();

    // Load configuration from file
    let file_config = ProxyConfig::from_file(path)?;
    debug!("Loaded configuration from file");

    // Merge with current configuration
    let new_config = current_config.merge(file_config);
    debug!("Merged with current configuration");

    // Update the configuration in the global state
    update_config(new_config.clone())?;
    info!("Configuration updated successfully");

    // Log the final configuration
    log_config(&new_config);

    Ok(new_config)
}

/// Configuration change event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigChangeEvent {
    /// Configuration was updated
    Updated,
    /// Configuration was reloaded from a file
    Reloaded,
}

/// Add a listener for configuration changes
pub fn add_listener<F>(_listener: F)
where
    F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
{
    // In a real implementation, this would add a listener for config changes
    // For now, we'll just do nothing
}

/// Get the buffer size from the current configuration
pub fn get_buffer_size() -> usize {
    get_config().buffer_size
}

/// Get the connection timeout from the current configuration
pub fn get_connection_timeout() -> u64 {
    get_config().connection_timeout
}

/// Check if client certificates are required
pub fn is_client_cert_required() -> bool {
    matches!(get_config().client_cert_mode, ClientCertMode::Required)
}

/// Check if sigalgs are enabled
pub fn is_sigalgs_enabled() -> bool {
    matches!(get_config().strategy, CertStrategyType::SigAlgs)
}

/// Certificate pair for TLS configuration
#[derive(Debug, Clone)]
pub struct CertificatePair {
    /// Path to the certificate file
    pub cert_path: PathBuf,
    /// Path to the private key file
    pub key_path: PathBuf,
}

/// Certificate configuration for TLS
#[derive(Debug, Clone)]
pub struct CertificateConfig {
    /// Traditional certificate pair
    pub traditional: Option<CertificatePair>,
    /// Hybrid certificate pair
    pub hybrid: Option<CertificatePair>,
    /// PQC-only certificate pair
    pub pqc_only: Option<CertificatePair>,
    /// Client CA certificate path
    pub client_ca_cert_path: Option<PathBuf>,
    /// Client certificate verification mode
    pub client_cert_mode: ClientCertMode,
}

// Export constants needed externally
pub use defaults::{ENV_PREFIX, DEFAULT_CONFIG_FILE, DEFAULT_CONFIG_DIR};
pub use defaults::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};
