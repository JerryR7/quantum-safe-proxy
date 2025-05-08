//! Configuration builder
//!
//! This module provides a builder pattern for constructing configuration.

use std::path::{Path, PathBuf};
use log::{debug, info};

use crate::config::types::ProxyConfig;
use crate::config::source::{ConfigSource, DefaultSource, FileSource, EnvSource, CliSource};
use crate::config::validator::validate_config;
use crate::config::error::Result;
use crate::config::{ENV_PREFIX, DEFAULT_CONFIG_FILE};

/// Configuration builder
///
/// Provides a fluent API for building configuration from multiple sources.
pub struct ConfigBuilder {
    sources: Vec<Box<dyn ConfigSource>>,
    validate: bool,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            validate: true,
        }
    }

    /// Add default source
    pub fn with_defaults(mut self) -> Self {
        debug!("Adding default configuration source");
        self.sources.push(Box::new(DefaultSource));
        self
    }

    /// Add file source
    pub fn with_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        let path = path.as_ref();
        debug!("Adding file configuration source: {}", path.display());
        self.sources.push(Box::new(FileSource::new(path)));
        self
    }

    /// Add environment source
    pub fn with_env(mut self, prefix: &str) -> Self {
        debug!("Adding environment configuration source with prefix: {}", prefix);
        self.sources.push(Box::new(EnvSource::new(prefix)));
        self
    }

    /// Add command line source
    pub fn with_cli(mut self, args: Vec<String>) -> Self {
        debug!("Adding command line configuration source");
        self.sources.push(Box::new(CliSource::new(args)));
        self
    }

    /// Disable validation
    pub fn without_validation(mut self) -> Self {
        self.validate = false;
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<ProxyConfig> {
        // Start with an empty configuration
        let mut config = ProxyConfig::default();

        // Apply sources in order (lowest to highest priority)
        for source in self.sources {
            let source_config = source.load()?;

            // Merge configurations
            config = config.merge(&source_config, source.source_type());
        }

        // Validate the configuration if enabled
        if self.validate {
            validate_config(&config)?;
        }

        // Log the final configuration
        config.log();

        Ok(config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
            .with_defaults()
            .with_file(DEFAULT_CONFIG_FILE)
            .with_env(ENV_PREFIX)
    }
}

/// Load configuration from auto-detected sources
///
/// This function loads configuration with proper priority:
/// 1. Default values (lowest priority)
/// 2. Configuration file (if exists)
/// 3. Environment variables
/// 4. Command line arguments (highest priority)
pub fn auto_load(args: Vec<String>) -> Result<ProxyConfig> {
    info!("Auto-loading configuration");
    debug!("Command line arguments: {:?}", args);

    // Extract config file path from command line arguments
    let config_file = extract_config_file(&args)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));

    debug!("Using configuration file: {}", config_file.display());

    // Build configuration
    let config = ConfigBuilder::new()
        .with_defaults()
        .with_file(&config_file)
        .with_env(ENV_PREFIX)
        .with_cli(args)
        .build()?;

    Ok(config)
}

/// Auto-load configuration using command line arguments
///
/// This function automatically collects command line arguments and loads configuration
/// with proper priority:
/// 1. Default values (lowest priority)
/// 2. Configuration file (if exists)
/// 3. Environment variables
/// 4. Command line arguments (highest priority)
///
/// It also handles special arguments like --version and --help.
pub fn auto_load_default() -> Result<ProxyConfig> {
    let args: Vec<String> = std::env::args().collect();

    // Handle special arguments
    if args.contains(&"--version".to_string()) || args.contains(&"--show-version".to_string()) {
        println!("quantum-safe-proxy {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    // Use the existing auto_load function
    auto_load(args)
}

/// Print help information
fn print_help() {
    println!("Usage: quantum-safe-proxy [OPTIONS]");
    println!("Options:");
    println!("  --listen ADDR                 Listen address (host:port)");
    println!("  --target ADDR                 Target address (host:port)");
    println!("  --log-level LEVEL             Log level (error, warn, info, debug, trace)");
    println!("  --client-cert-mode MODE       Client certificate verification mode (required, optional, none)");
    println!("  --buffer-size SIZE            Buffer size for data transfer (in bytes)");
    println!("  --connection-timeout SECONDS  Connection timeout in seconds");
    println!("  --openssl-dir DIR             OpenSSL installation directory");
    println!("  --strategy STRATEGY           Certificate strategy type (single, sigalgs, dynamic)");
    println!("  --traditional-cert FILE       Traditional certificate path");
    println!("  --traditional-key FILE        Traditional private key path");
    println!("  --hybrid-cert FILE            Hybrid certificate path");
    println!("  --hybrid-key FILE             Hybrid private key path");
    println!("  --pqc-only-cert FILE          PQC-only certificate path");
    println!("  --pqc-only-key FILE           PQC-only private key path");
    println!("  --client-ca-cert FILE         Client CA certificate path");
    println!("  --config-file FILE            Configuration file path");
    println!("  --show-version                Print version information and exit");
    println!("  --version                     Print version information and exit");
    println!("  --help                        Print help information");
}

/// Extract config file path from command line arguments
pub fn extract_config_file(args: &[String]) -> Option<PathBuf> {
    let mut args_iter = args.iter();

    // Skip the program name
    args_iter.next();

    while let Some(arg) = args_iter.next() {
        if arg == "--config-file" {
            if let Some(value) = args_iter.next() {
                return Some(PathBuf::from(value));
            }
        }
    }

    // Check environment variable
    if let Ok(path) = std::env::var(format!("{}CONFIG_FILE", ENV_PREFIX)) {
        return Some(PathBuf::from(path));
    }

    None
}
