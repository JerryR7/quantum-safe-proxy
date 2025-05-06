//! Configuration module
//!
//! This module handles application configuration, including loading from
//! different sources (files, environment variables, command line arguments)
//! and validating the configuration.

// Submodules
mod loader;
mod merger;
mod validator;
mod defaults;
pub mod strategy;

// Re-export types and traits
pub use self::loader::ConfigLoader;
pub use self::merger::ConfigMerger;
pub use self::validator::ConfigValidator;
pub use self::strategy::CertificateStrategyBuilder;

use serde::{Deserialize, Serialize, Deserializer};
use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Custom deserializer for socket addresses
fn deserialize_socket_addr<'de, D>(deserializer: D) -> std::result::Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_socket_addr(&s).map_err(serde::de::Error::custom)
}

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
    #[serde(default = "defaults::listen", deserialize_with = "deserialize_socket_addr")]
    pub listen: SocketAddr,

    /// Target service address to forward traffic to (host:port format)
    #[serde(default = "defaults::target_str")]
    pub target: String,

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
            target: defaults::target_str(),

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

impl ProxyConfig {
    /// Resolve the target address string to a SocketAddr
    ///
    /// This method handles various formats including:
    /// - IP:port (e.g., "127.0.0.1:6000")
    /// - Hostname:port (e.g., "localhost:6000")
    /// - Docker service name:port (e.g., "backend:6000")
    ///
    /// # Returns
    ///
    /// Returns a Result containing the resolved SocketAddr or an error
    pub fn resolve_target(&self) -> Result<SocketAddr> {
        parse_socket_addr(&self.target)
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
/// In Docker environments, service names like "backend:6000" are handled specially
#[inline]
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    use log::{debug, warn};

    debug!("Parsing socket address: {}", addr);

    // Try direct parsing first (most efficient)
    if let Ok(socket_addr) = SocketAddr::from_str(addr) {
        debug!("Successfully parsed as direct socket address: {}", socket_addr);
        return Ok(socket_addr);
    }

    debug!("Not a direct socket address, trying to parse as host:port");

    // Special handling for Docker service names (format: "service:port")
    if let Some((host, port_str)) = addr.split_once(':') {
        debug!("Split into host: '{}' and port: '{}'", host, port_str);

        if let Ok(port) = port_str.parse::<u16>() {
            debug!("Successfully parsed port: {}", port);

            // For Docker environments, we'll use a placeholder IP that will be resolved at connection time
            // This allows us to parse the address without actual DNS resolution
            use std::net::{IpAddr, Ipv4Addr};
            let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
            debug!("Using placeholder IP for Docker service name: {}:{}", ip, port);
            return Ok(SocketAddr::new(ip, port));
        } else {
            warn!("Failed to parse port part '{}' as u16", port_str);
        }
    } else {
        warn!("Address '{}' does not contain a colon separator", addr);
    }

    debug!("Trying standard DNS resolution for: {}", addr);

    // Try using ToSocketAddrs trait for hostname resolution
    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(resolved) = addrs.next() {
                debug!("Successfully resolved to: {}", resolved);
                Ok(resolved)
            } else {
                let err = format!("Address resolved but no socket addresses returned: {}", addr);
                warn!("{}", err);
                Err(ProxyError::Network(err))
            }
        },
        Err(e) => {
            let err = format!("Failed to parse or resolve address {}: {}", addr, e);
            warn!("{}", err);
            Err(ProxyError::Network(err))
        }
    }
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
pub fn log_config(config: &ProxyConfig) {
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
    use log::info;
    // Initialize configuration with default values
    let config = ProxyConfig::auto_load()?;

    // Log the initial configuration (before command line arguments)
    info!("Initial configuration from file and environment variables:");
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
    let loaded_config = current_config.merge(file_config);
    debug!("Merged with current configuration");

    // Update the configuration in the global state
    update_config(loaded_config.clone())?;
    info!("Configuration updated successfully");

    // Log the final configuration
    log_config(&loaded_config);

    Ok(loaded_config)
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
