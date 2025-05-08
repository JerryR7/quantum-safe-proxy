//! Configuration module
//!
//! This module handles application configuration, including loading from
//! different sources (files, environment variables, command line arguments)
//! and validating the configuration.
//!
//! The configuration system follows a clear priority order:
//! 1. Command-line arguments (highest priority)
//! 2. Environment variables
//! 3. Configuration file
//! 4. Default values (lowest priority)
//!
//! Only explicitly specified values from higher priority sources will override
//! values from lower priority sources.

// Internal modules
mod types;
mod source;
mod manager;
mod actor;
mod loader;
mod traits;

// Public modules
pub mod error;
pub mod validator;
pub mod builder;

// Re-export public types and functions
pub use types::{ProxyConfig, ClientCertMode, CertStrategyType, parse_socket_addr};
pub use manager::{
    initialize, get_config, update_config, reload_config, add_listener,
    ConfigChangeEvent, get_buffer_size, get_connection_timeout,
    is_client_cert_required, is_sigalgs_enabled
};
pub use builder::ConfigBuilder;
pub use error::{ConfigError, Result};
pub use actor::{ConfigActor, ConfigMessage};
pub use traits::{ConfigLoader, ConfigValidator};

// No compatibility layer needed

// Public constants
pub const ENV_PREFIX: &str = "QUANTUM_SAFE_PROXY_";
pub const DEFAULT_CONFIG_FILE: &str = "config.json";
pub const DEFAULT_CONFIG_DIR: &str = "config";

// Network settings constants
pub const LISTEN_STR: &str = "0.0.0.0:8443";
pub const TARGET_STR: &str = "127.0.0.1:6000";

// Certificate paths constants
pub const CERT_PATH_STR: &str = "certs/hybrid/ml-dsa-87/server.crt";
pub const KEY_PATH_STR: &str = "certs/hybrid/ml-dsa-87/server.key";
pub const CA_CERT_PATH_STR: &str = "certs/hybrid/ml-dsa-87/ca.crt";

// Other constants
pub const LOG_LEVEL_STR: &str = "info";

// Public utility functions
