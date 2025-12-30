//! Configuration sources
//!
//! This module defines traits and implementations for loading configuration
//! from different sources.

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::env;
use std::collections::HashMap;
use log::{debug, warn};

use crate::config::types::{ProxyConfig, ConfigValues, ValueSource, ClientCertMode, parse_socket_addr};
use crate::config::error::{ConfigError, Result};

/// Configuration source trait
pub trait ConfigSource {
    /// Load configuration from this source
    fn load(&self) -> Result<ProxyConfig>;

    /// Get the source type
    fn source_type(&self) -> ValueSource;
}

/// Default configuration source
pub struct DefaultSource;

impl ConfigSource for DefaultSource {
    fn load(&self) -> Result<ProxyConfig> {
        debug!("Loading default configuration");
        Ok(ProxyConfig::default())
    }

    fn source_type(&self) -> ValueSource {
        ValueSource::Default
    }
}

/// File configuration source
pub struct FileSource {
    pub path: PathBuf,
}

impl FileSource {
    /// Create a new file source
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl ConfigSource for FileSource {
    fn load(&self) -> Result<ProxyConfig> {
        debug!("Loading configuration from file: {}", self.path.display());

        // Check if file exists
        if !self.path.exists() {
            warn!("Configuration file not found: {}", self.path.display());
            warn!("Will use default values unless overridden by environment variables or command line arguments");
            return Ok(ProxyConfig {
                values: ConfigValues::default(),
                config_file: None,
                sources: HashMap::new(),
            });
        }

        // Read file contents
        let mut contents = String::new();
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) => {
                let err_msg = format!("Failed to open configuration file {}: {}", self.path.display(), e);
                warn!("{}", err_msg);
                return Err(ConfigError::FileReadError(self.path.clone(), e.to_string()));
            }
        };

        if let Err(e) = file.read_to_string(&mut contents) {
            let err_msg = format!("Failed to read configuration file {}: {}", self.path.display(), e);
            warn!("{}", err_msg);
            return Err(ConfigError::FileReadError(self.path.clone(), e.to_string()));
        }

        // Parse JSON
        debug!("Parsing JSON from file: {}", self.path.display());

        let values: ConfigValues = match serde_json::from_str::<ConfigValues>(&contents) {
            Ok(v) => v,
            Err(e) => {
                let err_msg = format!("Error parsing {}: {}", self.path.display(), e);
                warn!("{}", err_msg);
                return Err(ConfigError::ParseError(err_msg));
            }
        };

        // Create config with values
        let mut config = ProxyConfig {
            values,
            config_file: Some(self.path.clone()),
            sources: HashMap::new(),
        };

        // Update sources for all non-None fields
        let source = self.source_type();

        let fields = [
            "listen", "target", "log_level", "client_cert_mode", "buffer_size",
            "connection_timeout", "openssl_dir", "cert", "key", "fallback_cert",
            "fallback_key", "client_ca_cert",
        ];

        for name in fields {
            let has_value = match name {
                "listen" => config.values.listen.is_some(),
                "target" => config.values.target.is_some(),
                "log_level" => config.values.log_level.is_some(),
                "client_cert_mode" => config.values.client_cert_mode.is_some(),
                "buffer_size" => config.values.buffer_size.is_some(),
                "connection_timeout" => config.values.connection_timeout.is_some(),
                "openssl_dir" => config.values.openssl_dir.is_some(),
                "cert" => config.values.cert.is_some(),
                "key" => config.values.key.is_some(),
                "fallback_cert" => config.values.fallback_cert.is_some(),
                "fallback_key" => config.values.fallback_key.is_some(),
                "client_ca_cert" => config.values.client_ca_cert.is_some(),
                _ => false,
            };

            if has_value {
                config.sources.insert(name.to_string(), source);
            }
        }

        Ok(config)
    }

    fn source_type(&self) -> ValueSource {
        ValueSource::File
    }
}

/// Environment variable configuration source
pub struct EnvSource {
    pub prefix: String,
}

impl EnvSource {
    /// Create a new environment source
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

impl ConfigSource for EnvSource {
    fn load(&self) -> Result<ProxyConfig> {
        debug!("Loading configuration from environment variables with prefix: {}", self.prefix);

        let mut config = ProxyConfig {
            values: ConfigValues::default(),
            config_file: None,
            sources: HashMap::new(),
        };

        // Environment variable mappings (env_name -> config_name)
        // Includes backward compatibility aliases
        let env_vars = [
            ("QUANTUM_SAFE_PROXY_LISTEN", "listen"),
            ("QUANTUM_SAFE_PROXY_TARGET", "target"),
            ("QUANTUM_SAFE_PROXY_LOG_LEVEL", "log_level"),
            ("QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE", "client_cert_mode"),
            ("QUANTUM_SAFE_PROXY_BUFFER_SIZE", "buffer_size"),
            ("QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT", "connection_timeout"),
            ("QUANTUM_SAFE_PROXY_OPENSSL_DIR", "openssl_dir"),
            // New simplified names
            ("QUANTUM_SAFE_PROXY_CERT", "cert"),
            ("QUANTUM_SAFE_PROXY_KEY", "key"),
            ("QUANTUM_SAFE_PROXY_FALLBACK_CERT", "fallback_cert"),
            ("QUANTUM_SAFE_PROXY_FALLBACK_KEY", "fallback_key"),
            ("QUANTUM_SAFE_PROXY_CLIENT_CA_CERT", "client_ca_cert"),
            // Backward compatibility aliases
            ("QUANTUM_SAFE_PROXY_HYBRID_CERT", "cert"),
            ("QUANTUM_SAFE_PROXY_HYBRID_KEY", "key"),
            ("QUANTUM_SAFE_PROXY_TRADITIONAL_CERT", "fallback_cert"),
            ("QUANTUM_SAFE_PROXY_TRADITIONAL_KEY", "fallback_key"),
            ("QUANTUM_SAFE_PROXY_CLIENT_CA_CERT_PATH", "client_ca_cert"),
        ];

        for (env_name, config_name) in env_vars {
            if let Ok(value) = env::var(env_name) {
                debug!("Found environment variable {}={}", env_name, value);

                match config_name {
                    "listen" | "target" => {
                        if let Ok(addr) = parse_socket_addr(&value) {
                            if config_name == "listen" {
                                config.values.listen = Some(addr);
                            } else {
                                config.values.target = Some(addr);
                            }
                            config.sources.insert(config_name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", config_name, value);
                        }
                    },
                    "log_level" => {
                        config.values.log_level = Some(value);
                        config.sources.insert(config_name.to_string(), self.source_type());
                    },
                    "client_cert_mode" => {
                        if let Ok(mode) = value.parse::<ClientCertMode>() {
                            config.values.client_cert_mode = Some(mode);
                            config.sources.insert(config_name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", config_name, value);
                        }
                    },
                    "buffer_size" => {
                        if let Ok(size) = value.parse::<usize>() {
                            config.values.buffer_size = Some(size);
                            config.sources.insert(config_name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", config_name, value);
                        }
                    },
                    "connection_timeout" => {
                        if let Ok(timeout) = value.parse::<u64>() {
                            config.values.connection_timeout = Some(timeout);
                            config.sources.insert(config_name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", config_name, value);
                        }
                    },
                    // Path fields
                    "openssl_dir" | "cert" | "key" | "fallback_cert" | "fallback_key" | "client_ca_cert" => {
                        let path = PathBuf::from(&value);
                        match config_name {
                            "openssl_dir" => config.values.openssl_dir = Some(path),
                            "cert" => config.values.cert = Some(path),
                            "key" => config.values.key = Some(path),
                            "fallback_cert" => config.values.fallback_cert = Some(path),
                            "fallback_key" => config.values.fallback_key = Some(path),
                            "client_ca_cert" => config.values.client_ca_cert = Some(path),
                            _ => {}
                        }
                        config.sources.insert(config_name.to_string(), self.source_type());
                    },
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    fn source_type(&self) -> ValueSource {
        ValueSource::Environment
    }
}

/// Command line argument configuration source
pub struct CliSource {
    pub args: Vec<String>,
}

impl CliSource {
    /// Create a new command line source
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

impl ConfigSource for CliSource {
    fn load(&self) -> Result<ProxyConfig> {
        debug!("Loading configuration from command line arguments");

        let mut config = ProxyConfig {
            values: ConfigValues::default(),
            config_file: None,
            sources: HashMap::new(),
        };
        let args = &self.args;

        let mut i = 1; // Skip program name

        while i < args.len() {
            let arg = &args[i];
            i += 1;

            match arg.as_str() {
                // Network settings
                "--listen" => {
                    if i < args.len() {
                        if let Ok(addr) = parse_socket_addr(&args[i]) {
                            config.values.listen = Some(addr);
                            config.sources.insert("listen".to_string(), self.source_type());
                        } else {
                            warn!("Invalid listen address: {}", args[i]);
                        }
                        i += 1;
                    }
                }

                "--target" => {
                    if i < args.len() {
                        if let Ok(addr) = parse_socket_addr(&args[i]) {
                            config.values.target = Some(addr);
                            config.sources.insert("target".to_string(), self.source_type());
                        } else {
                            warn!("Invalid target address: {}", args[i]);
                        }
                        i += 1;
                    }
                }

                // General settings
                "--log-level" => {
                    if i < args.len() {
                        config.values.log_level = Some(args[i].clone());
                        config.sources.insert("log_level".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--client-cert-mode" => {
                    if i < args.len() {
                        if let Ok(mode) = args[i].parse::<ClientCertMode>() {
                            config.values.client_cert_mode = Some(mode);
                            config.sources.insert("client_cert_mode".to_string(), self.source_type());
                        } else {
                            warn!("Invalid client certificate mode: {}", args[i]);
                        }
                        i += 1;
                    }
                }

                "--buffer-size" => {
                    if i < args.len() {
                        if let Ok(size) = args[i].parse::<usize>() {
                            config.values.buffer_size = Some(size);
                            config.sources.insert("buffer_size".to_string(), self.source_type());
                        } else {
                            warn!("Invalid buffer size: {}", args[i]);
                        }
                        i += 1;
                    }
                }

                "--connection-timeout" => {
                    if i < args.len() {
                        if let Ok(timeout) = args[i].parse::<u64>() {
                            config.values.connection_timeout = Some(timeout);
                            config.sources.insert("connection_timeout".to_string(), self.source_type());
                        } else {
                            warn!("Invalid connection timeout: {}", args[i]);
                        }
                        i += 1;
                    }
                }

                "--openssl-dir" => {
                    if i < args.len() {
                        config.values.openssl_dir = Some(PathBuf::from(&args[i]));
                        config.sources.insert("openssl_dir".to_string(), self.source_type());
                        i += 1;
                    }
                }

                // Certificate settings (new names)
                "--cert" => {
                    if i < args.len() {
                        config.values.cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--key" => {
                    if i < args.len() {
                        config.values.key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--fallback-cert" => {
                    if i < args.len() {
                        config.values.fallback_cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("fallback_cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--fallback-key" => {
                    if i < args.len() {
                        config.values.fallback_key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("fallback_key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--client-ca-cert" => {
                    if i < args.len() {
                        config.values.client_ca_cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("client_ca_cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                // Backward compatibility aliases
                "--hybrid-cert" => {
                    if i < args.len() {
                        config.values.cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--hybrid-key" => {
                    if i < args.len() {
                        config.values.key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--traditional-cert" => {
                    if i < args.len() {
                        config.values.fallback_cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("fallback_cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--traditional-key" => {
                    if i < args.len() {
                        config.values.fallback_key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("fallback_key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--config-file" => {
                    if i < args.len() {
                        config.config_file = Some(PathBuf::from(&args[i]));
                        i += 1;
                    }
                }

                // Skip version and help arguments
                "--version" | "--show-version" | "--help" | "-h" => {}

                // Ignore deprecated --strategy flag
                "--strategy" => {
                    if i < args.len() {
                        warn!("--strategy is deprecated and will be ignored. Strategy is now auto-detected.");
                        i += 1;
                    }
                }

                // Unknown argument
                _ => {
                    if arg.starts_with("--") {
                        warn!("Unknown command line argument: {}", arg);
                    }
                }
            }
        }

        Ok(config)
    }

    fn source_type(&self) -> ValueSource {
        ValueSource::CommandLine
    }
}
