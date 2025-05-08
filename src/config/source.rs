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

use crate::config::types::{ProxyConfig, ConfigValues, ValueSource, ClientCertMode, CertStrategyType, parse_socket_addr};
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
            // Return an empty configuration instead of an error
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

        // Set source for all fields that have values
        let fields = [
            "listen", "target", "log_level", "client_cert_mode", "buffer_size",
            "connection_timeout", "openssl_dir", "strategy", "traditional_cert",
            "traditional_key", "hybrid_cert", "hybrid_key", "pqc_only_cert",
            "pqc_only_key", "client_ca_cert_path",
        ];

        // Check each field
        for name in fields {
            let has_value = match name {
                "listen" => config.values.listen.is_some(),
                "target" => config.values.target.is_some(),
                "log_level" => config.values.log_level.is_some(),
                "client_cert_mode" => config.values.client_cert_mode.is_some(),
                "buffer_size" => config.values.buffer_size.is_some(),
                "connection_timeout" => config.values.connection_timeout.is_some(),
                "openssl_dir" => config.values.openssl_dir.is_some(),
                "strategy" => config.values.strategy.is_some(),
                "traditional_cert" => config.values.traditional_cert.is_some(),
                "traditional_key" => config.values.traditional_key.is_some(),
                "hybrid_cert" => config.values.hybrid_cert.is_some(),
                "hybrid_key" => config.values.hybrid_key.is_some(),
                "pqc_only_cert" => config.values.pqc_only_cert.is_some(),
                "pqc_only_key" => config.values.pqc_only_key.is_some(),
                "client_ca_cert_path" => config.values.client_ca_cert_path.is_some(),
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

        // Helper function to get environment variable
        let get_env = |name: &str| -> Option<String> {
            // Try with underscore format (QUANTUM_SAFE_PROXY_LISTEN)
            let env_name_underscore = format!("{}_{}", self.prefix, name.to_uppercase());

            // Try with an underscore format
            match env::var(&env_name_underscore) {
                Ok(value) => Some(value),
                Err(_) => None
            }
        };

        // Directly check for all environment variables
        let direct_env_vars = [
            ("QUANTUM_SAFE_PROXY_LISTEN", "listen"),
            ("QUANTUM_SAFE_PROXY_TARGET", "target"),
            ("QUANTUM_SAFE_PROXY_LOG_LEVEL", "log_level"),
            ("QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE", "client_cert_mode"),
            ("QUANTUM_SAFE_PROXY_BUFFER_SIZE", "buffer_size"),
            ("QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT", "connection_timeout"),
            ("QUANTUM_SAFE_PROXY_OPENSSL_DIR", "openssl_dir"),
            ("QUANTUM_SAFE_PROXY_STRATEGY", "strategy"),
            ("QUANTUM_SAFE_PROXY_TRADITIONAL_CERT", "traditional_cert"),
            ("QUANTUM_SAFE_PROXY_TRADITIONAL_KEY", "traditional_key"),
            ("QUANTUM_SAFE_PROXY_HYBRID_CERT", "hybrid_cert"),
            ("QUANTUM_SAFE_PROXY_HYBRID_KEY", "hybrid_key"),
            ("QUANTUM_SAFE_PROXY_PQC_ONLY_CERT", "pqc_only_cert"),
            ("QUANTUM_SAFE_PROXY_PQC_ONLY_KEY", "pqc_only_key"),
            ("QUANTUM_SAFE_PROXY_CLIENT_CA_CERT_PATH", "client_ca_cert_path"),
        ];

        for (env_name, config_name) in direct_env_vars {
            if let Ok(value) = env::var(env_name) {
                debug!("Found environment variable {}={}", env_name, value);

                match config_name {
                    "listen" | "target" => {
                        if let Ok(addr) = parse_socket_addr(&value) {
                            config.values.listen = if config_name == "listen" {
                                Some(addr)
                            } else {
                                config.values.listen
                            };

                            config.values.target = if config_name == "target" {
                                Some(addr)
                            } else {
                                config.values.target
                            };

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
                    "strategy" => {
                        if let Ok(strategy) = value.parse::<CertStrategyType>() {
                            config.values.strategy = Some(strategy);
                            config.sources.insert(config_name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", config_name, value);
                        }
                    },
                    // Path fields
                    "openssl_dir" | "traditional_cert" | "traditional_key" | "hybrid_cert" |
                    "hybrid_key" | "pqc_only_cert" | "pqc_only_key" | "client_ca_cert_path" => {
                        let path = PathBuf::from(&value);
                        match config_name {
                            "openssl_dir" => config.values.openssl_dir = Some(path),
                            "traditional_cert" => config.values.traditional_cert = Some(path),
                            "traditional_key" => config.values.traditional_key = Some(path),
                            "hybrid_cert" => config.values.hybrid_cert = Some(path),
                            "hybrid_key" => config.values.hybrid_key = Some(path),
                            "pqc_only_cert" => config.values.pqc_only_cert = Some(path),
                            "pqc_only_key" => config.values.pqc_only_key = Some(path),
                            "client_ca_cert_path" => config.values.client_ca_cert_path = Some(path),
                            _ => {} // Should never happen
                        }
                        config.sources.insert(config_name.to_string(), self.source_type());
                    },
                    _ => {} // Should never happen
                }
            }
        }

        // Process all environment variables
        for name in [
            "listen", "target", "log_level", "client_cert_mode", "buffer_size",
            "connection_timeout", "openssl_dir", "strategy", "traditional_cert",
            "traditional_key", "hybrid_cert", "hybrid_key", "pqc_only_cert",
            "pqc_only_key", "client_ca_cert_path"
        ] {
            if let Some(value) = get_env(name) {
                match name {
                    "listen" | "target" => {
                        if let Ok(addr) = parse_socket_addr(&value) {
                            if name == "listen" {
                                config.values.listen = Some(addr);
                            } else {
                                config.values.target = Some(addr);
                            }
                            config.sources.insert(name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", name, value);
                        }
                    },
                    "log_level" => {
                        config.values.log_level = Some(value);
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "client_cert_mode" => {
                        if let Ok(mode) = value.parse::<ClientCertMode>() {
                            config.values.client_cert_mode = Some(mode);
                            config.sources.insert(name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", name, value);
                        }
                    },
                    "buffer_size" => {
                        if let Ok(size) = value.parse::<usize>() {
                            config.values.buffer_size = Some(size);
                            config.sources.insert(name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", name, value);
                        }
                    },
                    "connection_timeout" => {
                        if let Ok(timeout) = value.parse::<u64>() {
                            config.values.connection_timeout = Some(timeout);
                            config.sources.insert(name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", name, value);
                        }
                    },
                    "strategy" => {
                        if let Ok(strategy) = value.parse::<CertStrategyType>() {
                            config.values.strategy = Some(strategy);
                            config.sources.insert(name.to_string(), self.source_type());
                        } else {
                            warn!("Invalid {} in environment: {}", name, value);
                        }
                    },
                    // Path fields
                    "openssl_dir" => {
                        config.values.openssl_dir = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "traditional_cert" => {
                        config.values.traditional_cert = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "traditional_key" => {
                        config.values.traditional_key = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "hybrid_cert" => {
                        config.values.hybrid_cert = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "hybrid_key" => {
                        config.values.hybrid_key = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "pqc_only_cert" => {
                        config.values.pqc_only_cert = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "pqc_only_key" => {
                        config.values.pqc_only_key = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    "client_ca_cert_path" => {
                        config.values.client_ca_cert_path = Some(PathBuf::from(&value));
                        config.sources.insert(name.to_string(), self.source_type());
                    },
                    _ => {} // Should never happen due to our controlled list
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

        // Skip the program name
        let mut i = 1;

        while i < args.len() {
            let arg = &args[i];
            i += 1; // Move to next argument

            match arg.as_str() {
                // Network settings
                "--listen" => {
                    if i < args.len() {
                        match parse_socket_addr(&args[i]) {
                            Ok(addr) => {
                                config.values.listen = Some(addr);
                                config.sources.insert("listen".to_string(), self.source_type());
                            }
                            Err(e) => warn!("Invalid listen address in command line: {}", e),
                        }
                        i += 1;
                    }
                }

                "--target" => {
                    if i < args.len() {
                        match parse_socket_addr(&args[i]) {
                            Ok(addr) => {
                                config.values.target = Some(addr);
                                config.sources.insert("target".to_string(), self.source_type());
                            }
                            Err(e) => warn!("Invalid target address in command line: {}", e),
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
                        match args[i].parse::<ClientCertMode>() {
                            Ok(mode) => {
                                config.values.client_cert_mode = Some(mode);
                                config.sources.insert("client_cert_mode".to_string(), self.source_type());
                            }
                            Err(e) => warn!("Invalid client certificate mode in command line: {}", e),
                        }
                        i += 1;
                    }
                }

                "--buffer-size" => {
                    if i < args.len() {
                        match args[i].parse::<usize>() {
                            Ok(size) => {
                                config.values.buffer_size = Some(size);
                                config.sources.insert("buffer_size".to_string(), self.source_type());
                            }
                            Err(_) => warn!("Invalid buffer size in command line: {}", args[i]),
                        }
                        i += 1;
                    }
                }

                "--connection-timeout" => {
                    if i < args.len() {
                        match args[i].parse::<u64>() {
                            Ok(timeout) => {
                                config.values.connection_timeout = Some(timeout);
                                config.sources.insert("connection_timeout".to_string(), self.source_type());
                            }
                            Err(_) => warn!("Invalid connection timeout in command line: {}", args[i]),
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

                // Certificate strategy settings
                "--strategy" => {
                    if i < args.len() {
                        match args[i].parse::<CertStrategyType>() {
                            Ok(strategy) => {
                                config.values.strategy = Some(strategy);
                                config.sources.insert("strategy".to_string(), self.source_type());
                            }
                            Err(e) => warn!("Invalid certificate strategy in command line: {}", e),
                        }
                        i += 1;
                    }
                }

                "--traditional-cert" => {
                    if i < args.len() {
                        config.values.traditional_cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("traditional_cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--traditional-key" => {
                    if i < args.len() {
                        config.values.traditional_key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("traditional_key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--hybrid-cert" => {
                    if i < args.len() {
                        config.values.hybrid_cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("hybrid_cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--hybrid-key" => {
                    if i < args.len() {
                        config.values.hybrid_key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("hybrid_key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--pqc-only-cert" => {
                    if i < args.len() {
                        config.values.pqc_only_cert = Some(PathBuf::from(&args[i]));
                        config.sources.insert("pqc_only_cert".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--pqc-only-key" => {
                    if i < args.len() {
                        config.values.pqc_only_key = Some(PathBuf::from(&args[i]));
                        config.sources.insert("pqc_only_key".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--client-ca-cert" => {
                    if i < args.len() {
                        config.values.client_ca_cert_path = Some(PathBuf::from(&args[i]));
                        config.sources.insert("client_ca_cert_path".to_string(), self.source_type());
                        i += 1;
                    }
                }

                "--config-file" => {
                    if i < args.len() {
                        let config_file = PathBuf::from(&args[i]);
                        debug!("Found config file argument: {}", config_file.display());

                        // Check if the file exists
                        if config_file.exists() {
                            debug!("Config file exists: {}", config_file.display());
                        } else {
                            debug!("Config file does not exist: {}", config_file.display());
                        }

                        config.config_file = Some(config_file);
                        i += 1;
                    }
                }

                _ => {
                    // Ignore unknown arguments
                    if arg.starts_with("--") && i < args.len() && !args[i].starts_with("--") {
                        // Skip the value if there is one
                        i += 1;
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
