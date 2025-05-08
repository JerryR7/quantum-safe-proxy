//! Configuration module
//!
//! This module handles application configuration, including loading from
//! different sources (files, environment variables, command line arguments)
//! and validating the configuration.

// Submodules
mod types;
mod traits;
mod loader;
mod validator;
mod logger;
pub mod defaults;
pub mod strategy;
pub mod manager;

// Re-export types from types.rs
pub use self::types::{
    ProxyConfig, ClientCertMode, CertStrategyType,
    parse_socket_addr, check_file_exists
};

// Re-export traits from traits.rs
pub use self::traits::{
    ConfigLoader, ConfigValidator, ConfigMerger, ConfigLogger
};

// Re-export functions from logger.rs
pub use self::logger::log_config;

// Re-export traits from strategy.rs
pub use self::strategy::CertificateStrategyBuilder;

// Re-export constants needed externally
pub use defaults::{ENV_PREFIX, DEFAULT_CONFIG_FILE, DEFAULT_CONFIG_DIR};
pub use defaults::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};

// Re-export configuration management functions
pub use manager::{
    initialize, get_config, update_config, reload_config,
    get_buffer_size, get_connection_timeout,
    is_client_cert_required, is_sigalgs_enabled,
    ConfigChangeEvent
};
