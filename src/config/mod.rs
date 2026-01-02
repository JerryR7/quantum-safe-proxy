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
//! ## Certificate Strategy
//!
//! The certificate strategy is now automatically determined:
//! - **Single mode**: Only primary certificate (`cert`/`key`) is configured
//! - **Dynamic mode**: Both primary and fallback certificates are configured
//!
//! This eliminates the need for explicit `strategy` configuration.

// Internal modules
mod source;
mod manager;
mod actor;
mod loader;
mod traits;

// Public modules
pub mod types;
pub mod error;
pub mod validator;
pub mod builder;

// Re-export public types and functions
pub use types::{ProxyConfig, ClientCertMode, parse_socket_addr};
pub use manager::{
    initialize, get_config, update_config, reload_config, add_listener,
    ConfigChangeEvent, get_buffer_size, get_connection_timeout,
    is_client_cert_required, is_dynamic_cert_enabled, save_config
};
pub use builder::ConfigBuilder;
pub use error::{ConfigError, Result};
pub use actor::{ConfigActor, ConfigMessage};
pub use traits::{ConfigLoader, ConfigValidator};

// Public constants
pub const ENV_PREFIX: &str = "QUANTUM_SAFE_PROXY_";
pub const DEFAULT_CONFIG_FILE: &str = "config.json";
pub const DEFAULT_CONFIG_DIR: &str = "config";

// Network settings constants
pub const LISTEN_STR: &str = "0.0.0.0:8443";
pub const TARGET_STR: &str = "127.0.0.1:6000";

// Certificate paths constants
pub const CERT_PATH_STR: &str = "certs/server-pqc.crt";
pub const KEY_PATH_STR: &str = "certs/server-pqc.key";
pub const CA_CERT_PATH_STR: &str = "certs/pqc-full-chain.crt";

// Other constants
pub const LOG_LEVEL_STR: &str = "info";
