//! Configuration module
//!
//! This module handles application configuration, including loading from
//! different sources (files, environment variables, command line arguments)
//! and validating the configuration.

mod config;
mod defaults;
mod manager;

// Re-export types
pub use config::{ProxyConfig, ClientCertMode, parse_socket_addr};
pub use manager::{initialize, get_config, update_config, reload_config, add_listener, ConfigChangeEvent, get_buffer_size, get_connection_timeout, is_client_cert_required, is_sigalgs_enabled};

// Export constants needed externally
pub use defaults::{ENV_PREFIX, DEFAULT_CONFIG_FILE, DEFAULT_CONFIG_DIR};
pub use defaults::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};

use std::path::{Path, PathBuf};
// use std::env; // 不再需要，因為我們優化了配置加載流程

use log::{info, warn, debug};

use crate::common::Result;

/// Load configuration from multiple sources
///
/// This function loads configuration from the following sources in order of priority:
/// 1. Default values (lowest priority)
/// 2. Configuration file
/// 3. Environment variables
/// 4. Command line arguments (highest priority)
///
/// Optimized for performance with reduced file system access and validation.
///
/// # Arguments
///
/// * `args` - Command line arguments
/// * `config_file` - Optional path to configuration file
///
/// # Returns
///
/// The loaded configuration
pub fn load_config(args: Vec<String>, config_file: Option<&str>) -> Result<ProxyConfig> {
    // Start with default configuration
    let mut config = ProxyConfig::default();
    debug!("Starting with default configuration");

    // Optimized configuration file loading
    // Only check the file system once for each potential path
    if let Some(path) = config_file {
        // Try specified configuration files first
        let path_exists = Path::new(path).exists();
        if path_exists {
            info!("Loading configuration from specified file: {}", path);
            if let Ok(file_config) = ProxyConfig::from_file(path) {
                config = config.merge(file_config);
                debug!("Merged configuration from file");
            } else {
                warn!("Failed to load configuration from file: {}", path);
            }
        } else {
            // Try default configuration files if the specified file doesn't exist
            let default_exists = Path::new(DEFAULT_CONFIG_FILE).exists();
            if default_exists {
                info!("Loading configuration from {}", DEFAULT_CONFIG_FILE);
                if let Ok(file_config) = ProxyConfig::from_file(DEFAULT_CONFIG_FILE) {
                    config = config.merge(file_config);
                    debug!("Merged default configuration file");
                } else {
                    warn!("Failed to load default configuration file");
                }
            }
        }
    } else if Path::new(DEFAULT_CONFIG_FILE).exists() {
        // No config file specified, try default
        info!("Loading configuration from {}", DEFAULT_CONFIG_FILE);
        if let Ok(file_config) = ProxyConfig::from_file(DEFAULT_CONFIG_FILE) {
            config = config.merge(file_config);
            debug!("Merged default configuration file");
        } else {
            warn!("Failed to load default configuration file");
        }
    }

    // Load from environment variables (optimized to avoid unnecessary processing)
    if let Ok(env_config) = ProxyConfig::from_env() {
        // Only merge if any environment variables were actually set
        if env_config != ProxyConfig::default() {
            info!("Loading configuration from environment variables");
            config = config.merge(env_config);
            debug!("Merged environment variables configuration");
        }
    }

    // Parse command line arguments
    if args.len() > 1 {  // 第一個參數是程序名稱，忽略
        info!("Applying configuration from command line arguments");
        let cli_config = parse_command_line_args(&args)?;
        config = config.merge(cli_config);
        debug!("Merged command line arguments configuration");
    }

    // Validate configuration
    config.validate()?;

    // Log configuration (only in debug mode to reduce overhead)
    info!("Configuration loaded successfully");
    if log::log_enabled!(log::Level::Debug) {
        debug!("Listen address: {}", config.listen);
        debug!("Target address: {}", config.target);
        debug!("Certificate path: {:?}", config.cert_path);
        debug!("Private key path: {:?}", config.key_path);
        debug!("CA certificate path: {:?}", config.ca_cert_path);
        debug!("Log level: {}", config.log_level);
        debug!("Client certificate mode: {}", config.client_cert_mode);
        debug!("Buffer size: {} bytes", config.buffer_size);
        debug!("Connection timeout: {} seconds", config.connection_timeout);
        debug!("Classic certificate path: {:?}", config.classic_cert);
        debug!("Classic key path: {:?}", config.classic_key);
        debug!("OpenSSL directory: {:?}", config.openssl_dir);
        debug!("Use SigAlgs strategy: {}", config.use_sigalgs);
    }

    Ok(config)
}

// Note: The reload_config function is now provided by the manager module

/// Parse command line arguments into a ProxyConfig
///
/// This function parses command line arguments and returns a ProxyConfig
/// with the values from the command line arguments.
///
/// # Arguments
///
/// * `args` - Command line arguments
///
/// # Returns
///
/// A ProxyConfig with values from command line arguments
fn parse_command_line_args(args: &[String]) -> Result<ProxyConfig> {
    use crate::common::ProxyError;

    // Create a default configuration
    let mut config = ProxyConfig::default();

    // Simple command line argument parsing
    let mut i = 1;  // Skip program name
    while i < args.len() {
        match args[i].as_str() {
            "--listen" => {
                if i + 1 < args.len() {
                    config.listen = self::parse_socket_addr(&args[i + 1])?;
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --listen".to_string()));
                }
            },
            "--target" => {
                if i + 1 < args.len() {
                    config.target = self::parse_socket_addr(&args[i + 1])?;
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --target".to_string()));
                }
            },
            "--cert" => {
                if i + 1 < args.len() {
                    config.cert_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --cert".to_string()));
                }
            },
            "--key" => {
                if i + 1 < args.len() {
                    config.key_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --key".to_string()));
                }
            },
            "--ca-cert" => {
                if i + 1 < args.len() {
                    config.ca_cert_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --ca-cert".to_string()));
                }
            },
            "--log-level" => {
                if i + 1 < args.len() {
                    config.log_level = args[i + 1].clone();
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --log-level".to_string()));
                }
            },
            "--client-cert-mode" => {
                if i + 1 < args.len() {
                    config.client_cert_mode = ClientCertMode::from_str(&args[i + 1])?;
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --client-cert-mode".to_string()));
                }
            },
            "--buffer-size" => {
                if i + 1 < args.len() {
                    config.buffer_size = args[i + 1].parse().map_err(|_| {
                        ProxyError::Config(format!("Invalid buffer size: {}", args[i + 1]))
                    })?;
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --buffer-size".to_string()));
                }
            },
            "--connection-timeout" => {
                if i + 1 < args.len() {
                    config.connection_timeout = args[i + 1].parse().map_err(|_| {
                        ProxyError::Config(format!("Invalid connection timeout: {}", args[i + 1]))
                    })?;
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --connection-timeout".to_string()));
                }
            },
            "--openssl-dir" => {
                if i + 1 < args.len() {
                    config.openssl_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --openssl-dir".to_string()));
                }
            },
            "--classic-cert" => {
                if i + 1 < args.len() {
                    config.classic_cert = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --classic-cert".to_string()));
                }
            },
            "--classic-key" => {
                if i + 1 < args.len() {
                    config.classic_key = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    return Err(ProxyError::Config("Missing value for --classic-key".to_string()));
                }
            },
            "--use-sigalgs" => {
                if i + 1 < args.len() {
                    config.use_sigalgs = args[i + 1].parse().map_err(|_| {
                        ProxyError::Config(format!("Invalid use_sigalgs value: {}", args[i + 1]))
                    })?;
                    i += 2;
                } else {
                    // If --use-sigalgs is specified without a value, assume true
                    config.use_sigalgs = true;
                    i += 1;
                }
            },
            _ => {
                // Skip unknown arguments
                i += 1;
            }
        }
    }

    Ok(config)
}
