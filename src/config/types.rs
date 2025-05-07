//! Configuration types
//!
//! This module contains the main configuration types used throughout the application.

use std::path::{Path, PathBuf};
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::FromStr;
use serde::{Deserialize, Serialize, Deserializer};

use crate::config::traits::ConfigMerger;

use crate::common::{ProxyError, Result};
use crate::config::defaults;

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

impl std::fmt::Display for ClientCertMode {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// Custom deserializer for socket addresses
fn deserialize_socket_addr<'de, D>(deserializer: D) -> std::result::Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_socket_addr(&s).map_err(serde::de::Error::custom)
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

/// Parse a socket address string for clap
///
/// This is a wrapper around parse_socket_addr that returns a clap-compatible error
pub fn parse_socket_addr_string(addr: &str) -> std::result::Result<SocketAddr, String> {
    parse_socket_addr(addr).map_err(|e| e.to_string())
}

/// Parse a client certificate mode string for clap
///
/// This is a wrapper around ClientCertMode::from_str that returns a clap-compatible error
pub fn parse_client_cert_mode(mode: &str) -> std::result::Result<ClientCertMode, String> {
    ClientCertMode::from_str(mode).map_err(|e| e.to_string())
}

/// Parse a certificate strategy type string for clap
///
/// This is a wrapper around CertStrategyType::from_str that returns a clap-compatible error
pub fn parse_cert_strategy_type(strategy: &str) -> std::result::Result<CertStrategyType, String> {
    CertStrategyType::from_str(strategy).map_err(|e| e.to_string())
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

    /// Listen address (host:port)
    #[serde(default = "defaults::listen", deserialize_with = "deserialize_socket_addr")]
    pub listen: SocketAddr,

    /// Target address (host:port)
    #[serde(default = "defaults::target", deserialize_with = "deserialize_socket_addr")]
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

    /// Configuration file path
    #[serde(skip)]
    pub config_file: Option<PathBuf>,

    /// Print version information and exit
    #[serde(skip)]
    pub show_version: bool,
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

            // Configuration file path
            config_file: None,

            // Command line flags
            show_version: false,
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
    pub fn resolve_target(&self) -> Result<SocketAddr> {
        parse_socket_addr(&self.target.to_string())
    }

    /// Apply command line arguments to the configuration
    ///
    /// This method applies CLI arguments to the configuration, ensuring that
    /// certificate strategy is updated when certificate paths are changed.
    pub fn apply_cli_args(config: &mut Self, cli: &Self) {
        use log::debug;

        // Track certificate path changes for strategy auto-selection
        let mut cert_paths_changed = false;
        let mut pqc_changed = false;

        // Helper macro to reduce repetitive code
        macro_rules! override_if_changed {
            ($field:ident, $default:expr) => {
                if cli.$field != $default() {
                    debug!("Overriding {} with {:?}", stringify!($field), cli.$field);
                    config.$field = cli.$field.clone();
                }
            };
            ($field:ident, $default:expr, $flag:ident) => {
                if cli.$field != $default() {
                    debug!("Overriding {} with {:?}", stringify!($field), cli.$field);
                    config.$field = cli.$field.clone();
                    $flag = true;
                }
            };
            ($field:ident) => {
                if cli.$field.is_some() {
                    debug!("Overriding {} with {:?}", stringify!($field), cli.$field);
                    config.$field = cli.$field.clone();
                }
            };
        }

        // Network settings
        override_if_changed!(listen, defaults::listen);
        override_if_changed!(target, defaults::target);

        // Certificate paths
        override_if_changed!(traditional_cert, defaults::classic_cert_path, cert_paths_changed);
        override_if_changed!(traditional_key, defaults::classic_key_path, cert_paths_changed);
        override_if_changed!(hybrid_cert, defaults::cert_path, cert_paths_changed);
        override_if_changed!(hybrid_key, defaults::key_path, cert_paths_changed);
        override_if_changed!(client_ca_cert_path, defaults::ca_cert_path);

        // Optional PQC certificate fields
        if cli.pqc_only_cert.is_some() || cli.pqc_only_key.is_some() {
            config.pqc_only_cert = cli.pqc_only_cert.clone();
            config.pqc_only_key = cli.pqc_only_key.clone();
            pqc_changed = true;
        }

        // General settings
        override_if_changed!(log_level, defaults::log_level);
        override_if_changed!(client_cert_mode, defaults::client_cert_mode);
        override_if_changed!(buffer_size, defaults::buffer_size);
        override_if_changed!(connection_timeout, defaults::connection_timeout);
        override_if_changed!(openssl_dir);
        override_if_changed!(config_file);

        // Strategy setting - update based on certificate changes if not explicitly set
        if cli.strategy != CertStrategyType::default() {
            config.strategy = cli.strategy.clone();
        } else if cert_paths_changed || pqc_changed {
            // Auto-select appropriate strategy based on certificate paths
            config.strategy = if pqc_changed {
                CertStrategyType::Dynamic
            } else {
                CertStrategyType::SigAlgs
            };
        }

        // Command line flags
        config.show_version = cli.show_version;
    }

    /// Merge another configuration into this one
    ///
    /// Values from `other` will override values in `self` if they are not the default values.
    /// This is used to implement the configuration priority system.
    pub fn merge(&self, other: impl AsRef<ProxyConfig>) -> Self {
        // Create a clone of self
        let mut result = self.clone();

        // Override with values from other
        Self::apply_cli_args(&mut result, other.as_ref());

        // Return the merged result
        result
    }
}

// Implement AsRef<ProxyConfig> for ProxyConfig to simplify operations
impl AsRef<ProxyConfig> for ProxyConfig {
    #[inline]
    fn as_ref(&self) -> &ProxyConfig {
        self
    }
}

// Implement ConfigMerger for ProxyConfig
impl ConfigMerger for ProxyConfig {
    fn merge(&self, other: impl AsRef<ProxyConfig>) -> Self {
        self.merge(other)
    }
}
