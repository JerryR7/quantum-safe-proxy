//! Compatibility layer for the old config module
//!
//! This module provides compatibility with the old config module,
//! allowing existing code to continue working with the new config module.

use std::path::Path;
use std::net::SocketAddr;

use crate::config::{Config, ClientCertMode, CertStrategyType};
use crate::config::error::Result;

/// Proxy configuration (compatibility type)
///
/// This type is provided for compatibility with the old config module.
/// It forwards all calls to the new Config type.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    config: Config,
}

impl ProxyConfig {
    /// Create a new ProxyConfig from a Config
    pub fn from_config(config: Config) -> Self {
        Self { config }
    }

    /// Get the underlying Config
    pub fn as_config(&self) -> &Config {
        &self.config
    }

    /// Get the listen address
    pub fn listen(&self) -> SocketAddr {
        self.config.listen()
    }

    /// Get the target address
    pub fn target(&self) -> SocketAddr {
        self.config.target()
    }

    /// Load configuration from a file
    pub fn from_file(path: &str) -> Result<Self> {
        let config = crate::config::builder::ConfigBuilder::new()
            .with_defaults()
            .with_file(path)
            .build()?;

        Ok(Self::from_config(config))
    }

    /// Auto-load configuration from all sources
    pub fn auto_load() -> Result<Self> {
        let config = crate::config::builder::auto_load(std::env::args().collect())?;
        Ok(Self::from_config(config))
    }

    /// Check configuration for warnings
    pub fn check(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check certificate paths
        if !Path::new(self.traditional_cert()).exists() {
            warnings.push(format!("Traditional certificate file not found: {}", self.traditional_cert().display()));
        }

        if !Path::new(self.traditional_key()).exists() {
            warnings.push(format!("Traditional key file not found: {}", self.traditional_key().display()));
        }

        if !Path::new(self.hybrid_cert()).exists() {
            warnings.push(format!("Hybrid certificate file not found: {}", self.hybrid_cert().display()));
        }

        if !Path::new(self.hybrid_key()).exists() {
            warnings.push(format!("Hybrid key file not found: {}", self.hybrid_key().display()));
        }

        if let Some(cert) = self.pqc_only_cert() {
            if !cert.exists() {
                warnings.push(format!("PQC-only certificate file not found: {}", cert.display()));
            }
        }

        if let Some(key) = self.pqc_only_key() {
            if !key.exists() {
                warnings.push(format!("PQC-only key file not found: {}", key.display()));
            }
        }

        if self.client_cert_mode() == ClientCertMode::Required && !Path::new(self.client_ca_cert_path()).exists() {
            warnings.push(format!("Client CA certificate file not found: {}", self.client_ca_cert_path().display()));
        }

        warnings
    }

    /// Build certificate strategy
    pub fn build_cert_strategy(&self) -> Result<Box<dyn std::any::Any>> {
        use crate::tls::strategy::CertStrategy;

        // Convert our strategy to the tls module's strategy
        let strategy = match self.config.strategy() {
            CertStrategyType::Single => {
                CertStrategy::Single {
                    cert: self.config.hybrid_cert().to_path_buf(),
                    key: self.config.hybrid_key().to_path_buf(),
                }
            }
            CertStrategyType::SigAlgs => {
                CertStrategy::SigAlgs {
                    classic: (
                        self.config.traditional_cert().to_path_buf(),
                        self.config.traditional_key().to_path_buf()
                    ),
                    hybrid: (
                        self.config.hybrid_cert().to_path_buf(),
                        self.config.hybrid_key().to_path_buf()
                    ),
                }
            }
            CertStrategyType::Dynamic => {
                CertStrategy::Dynamic {
                    traditional: (
                        self.config.traditional_cert().to_path_buf(),
                        self.config.traditional_key().to_path_buf()
                    ),
                    hybrid: (
                        self.config.hybrid_cert().to_path_buf(),
                        self.config.hybrid_key().to_path_buf()
                    ),
                    pqc_only: self.config.pqc_only_cert().zip(self.config.pqc_only_key())
                        .map(|(cert, key)| (cert.to_path_buf(), key.to_path_buf())),
                }
            }
        };

        // Wrap the strategy in a Box
        Ok(Box::new(strategy))
    }
}

// Forward all fields to the underlying Config
impl std::ops::Deref for ProxyConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

// Implement Default for ProxyConfig
impl Default for ProxyConfig {
    fn default() -> Self {
        Self::from_config(Config::default())
    }
}

// Implement AsRef<Path> for compatibility
impl AsRef<Path> for ProxyConfig {
    fn as_ref(&self) -> &Path {
        self.config.config_file().unwrap_or_else(|| Path::new("config.json"))
    }
}

// Re-export the certificate strategy module
pub mod strategy {
    use crate::config::error::Result;

    /// Certificate strategy builder trait
    pub trait CertificateStrategyBuilder {
        /// Build certificate strategy
        fn build_cert_strategy(&self) -> Result<Box<dyn std::any::Any>>;
    }

    impl CertificateStrategyBuilder for super::ProxyConfig {
        fn build_cert_strategy(&self) -> Result<Box<dyn std::any::Any>> {
            self.build_cert_strategy()
        }
    }
}

/// Log configuration
pub fn log_config(config: &ProxyConfig) {
    config.config.log();
}

/// Parse a socket address
pub fn parse_socket_addr(addr: &str) -> Result<SocketAddr> {
    crate::config::types::parse_socket_addr(addr)
}

/// Initialize configuration
pub fn initialize(args: Vec<String>, config_file: Option<&str>) -> Result<()> {
    let config = if let Some(path) = config_file {
        crate::config::builder::ConfigBuilder::new()
            .with_defaults()
            .with_file(path)
            .with_env(crate::config::ENV_PREFIX)
            .with_cli(args)
            .build()?
    } else {
        crate::config::builder::auto_load(args)?
    };

    crate::config::initialize(config)
}

/// Reload configuration
pub fn reload_config(path: &str) -> Result<()> {
    crate::config::reload_config(path)
}
