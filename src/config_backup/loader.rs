//! Configuration loader implementation
//!
//! This module provides functionality for loading configuration from different sources.

use std::path::Path;
use log::debug;
use config::{Config, Environment, File};

use crate::common::Result;
use crate::config::defaults;
use crate::config::traits::{ConfigLoader, ConfigValidator};
use crate::config::types::ProxyConfig;

impl ConfigLoader for ProxyConfig {
    /// Auto-detect and load configuration from the best available source
    ///
    /// This method loads configuration with proper priority:
    /// 1. Default values (lowest priority)
    /// 2. Configuration file (config.json by default)
    /// 3. Environment variables
    /// 4. Command line arguments (highest priority)
    fn auto_load() -> Result<Self> {
        // Start with default configuration
        let mut config = Self::default();

        // Get config file path from environment or use default
        let config_path = std::env::var("QUANTUM_SAFE_PROXY_CONFIG_FILE")
            .map(|p| Path::new(&p).to_path_buf())
            .unwrap_or_else(|_| Path::new("config.json").to_path_buf());

        // Build configuration from multiple sources using config crate
        let config_builder = Config::builder()
            // Layer 1: Default values (handled by serde's default attribute)
            // Layer 2: Configuration file (if exists)
            .add_source(File::from(config_path.clone()).required(false))
            // Layer 3: Environment variables with prefix
            .add_source(Environment::with_prefix(defaults::ENV_PREFIX).separator("_"));

        // Build and deserialize the configuration
        if let Ok(cfg) = config_builder.build() {
            if let Ok(file_config) = cfg.try_deserialize::<Self>() {
                // Set the config file path if it exists
                config = file_config;
                if config_path.exists() {
                    config.config_file = Some(config_path.clone());
                    debug!("Configuration loaded from {}", config_path.display());
                }
            } else if config_path.exists() {
                debug!("Error deserializing configuration from {}", config_path.display());
            }
        } else if config_path.exists() {
            debug!("Error loading configuration from {}", config_path.display());
        }

        // Parse command line arguments
        let args: Vec<String> = std::env::args().collect();

        // Check for --version flag
        if args.contains(&"--version".to_string()) {
            println!("quantum-safe-proxy {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        // Check for --show-version flag
        if args.contains(&"--show-version".to_string()) {
            println!("quantum-safe-proxy {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        // Check for --help flag
        if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
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
            println!("  --client-ca-cert-path FILE    Client CA certificate path");
            println!("  --config-file FILE            Configuration file path");
            println!("  --show-version                Print version information and exit");
            println!("  --version                     Print version information and exit");
            println!("  --help                        Print help information");
            std::process::exit(0);
        }

        // Parse command line arguments
        for i in 1..args.len() {
            if args[i].starts_with("--") && i + 1 < args.len() {
                let key = args[i].trim_start_matches("--");
                let value = &args[i + 1];

                match key {
                    "listen" => {
                        if let Ok(addr) = value.parse() {
                            config.listen = addr;
                        }
                    },
                    "target" => {
                        if let Ok(addr) = value.parse() {
                            config.target = addr;
                        }
                    },
                    "log-level" => {
                        config.log_level = value.clone();
                    },
                    "client-cert-mode" => {
                        if let Ok(mode) = value.parse() {
                            config.client_cert_mode = mode;
                        }
                    },
                    "buffer-size" => {
                        if let Ok(size) = value.parse() {
                            config.buffer_size = size;
                        }
                    },
                    "connection-timeout" => {
                        if let Ok(timeout) = value.parse() {
                            config.connection_timeout = timeout;
                        }
                    },
                    "openssl-dir" => {
                        config.openssl_dir = Some(Path::new(value).to_path_buf());
                    },
                    "strategy" => {
                        if let Ok(strategy) = value.parse() {
                            config.strategy = strategy;
                        }
                    },
                    "traditional-cert" => {
                        config.traditional_cert = Path::new(value).to_path_buf();
                    },
                    "traditional-key" => {
                        config.traditional_key = Path::new(value).to_path_buf();
                    },
                    "hybrid-cert" => {
                        config.hybrid_cert = Path::new(value).to_path_buf();
                    },
                    "hybrid-key" => {
                        config.hybrid_key = Path::new(value).to_path_buf();
                    },
                    "pqc-only-cert" => {
                        config.pqc_only_cert = Some(Path::new(value).to_path_buf());
                    },
                    "pqc-only-key" => {
                        config.pqc_only_key = Some(Path::new(value).to_path_buf());
                    },
                    "client-ca-cert-path" => {
                        config.client_ca_cert_path = Path::new(value).to_path_buf();
                    },
                    "config-file" => {
                        config.config_file = Some(Path::new(value).to_path_buf());
                    },
                    _ => {}
                }
            }
        }

        // Validate configuration
        config.validate()?;

        debug!("Configuration loaded and validated successfully");
        Ok(config)
    }

    /// Load configuration from a specific file
    ///
    /// This method loads configuration from a file, environment variables, and defaults,
    /// then validates the configuration before returning it.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Start with default configuration
        let mut config = Self::default();

        // Build configuration from multiple sources using config crate
        let config_builder = Config::builder()
            // Layer 1: Default values (handled by serde's default attribute)
            // Layer 2: Configuration file (if exists)
            .add_source(File::from(path).required(false))
            // Layer 3: Environment variables with prefix
            .add_source(Environment::with_prefix(defaults::ENV_PREFIX).separator("_"));

        // Build and deserialize the configuration
        if let Ok(cfg) = config_builder.build() {
            if let Ok(file_config) = cfg.try_deserialize::<Self>() {
                // Merge file configuration with default
                config = file_config;

                if path.exists() {
                    debug!("Configuration loaded from {}", path.display());
                } else {
                    debug!("Using default configuration with environment variables");
                }
            } else if path.exists() {
                debug!("Error deserializing configuration from {}", path.display());
            }
        } else if path.exists() {
            debug!("Error loading configuration from {}", path.display());
        }

        // Set the config file path if it exists
        if path.exists() {
            config.config_file = Some(path.to_path_buf());
        }

        // Validate configuration
        config.validate()?;

        debug!("Configuration validated successfully");
        Ok(config)
    }
}


