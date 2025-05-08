//! Configuration builder
//!
//! This module provides a builder pattern for constructing configuration.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use log::debug;

use crate::config::types::{ProxyConfig, ConfigValues};
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
        // Start with an empty configuration (without defaults)
        let mut config = ProxyConfig {
            values: ConfigValues::default(),
            config_file: None,
            sources: HashMap::new(),
        };

        debug!("Building configuration from {} sources", self.sources.len());

        // Apply sources in order (lowest to highest priority)
        for source in self.sources {
            let source_type = source.source_type();
            debug!("Loading configuration from source: {:?}", source_type);

            let source_config = source.load()?;

            // Merge configurations
            config = config.merge(&source_config, source_type);
        }

        // Apply default values for any fields that are still None
        config.set_default_values();

        // Validate the configuration if enabled
        if self.validate {
            debug!("Validating configuration");
            validate_config(&config)?;
        }

        // Log the final configuration at debug level
        debug!("Final configuration:");
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
    // Handle special arguments
    if args.contains(&"--version".to_string()) || args.contains(&"--show-version".to_string()) {
        println!("quantum-safe-proxy {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    // Get config file path from command line arguments or environment
    let config_file = extract_config_file(&args)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));

    log::info!("Configuration file path: {}", config_file.display());

    if !config_file.exists() {
        log::warn!("Configuration file not found: {}", config_file.display());
        log::warn!("Will use default values unless overridden by environment variables or command line arguments");
    } else {
        log::info!("Using configuration file: {}", config_file.display());
    }

    // Build configuration using the builder
    debug!("Building configuration with file: {}", config_file.display());
    let mut builder = ConfigBuilder::new();

    // Add sources in order of priority
    builder = builder.with_defaults();

    if config_file.exists() {
        debug!("Adding file source: {}", config_file.display());
        builder = builder.with_file(&config_file);
    }

    builder = builder.with_env(ENV_PREFIX);
    builder = builder.with_cli(args);

    // Build the configuration
    let config = builder.build()?;

    // Log the final configuration
    debug!("Configuration loaded successfully");
    debug!("Listen address: {}", config.listen());
    debug!("Target address: {}", config.target());
    debug!("Log level: {}", config.log_level());

    Ok(config)
}

/// Extract config file path from command line arguments
fn extract_config_file(args: &[String]) -> Option<PathBuf> {
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
