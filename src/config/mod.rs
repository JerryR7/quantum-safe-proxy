//! Configuration module
//!
//! This module handles application configuration, including command-line arguments,
//! environment variables, and configuration files.

mod config;
mod defaults;

pub use config::{ProxyConfig, ClientCertMode};

// Export specific items from defaults that are needed externally
pub use defaults::{ENV_PREFIX, DEFAULT_CONFIG_FILE, DEFAULT_CONFIG_DIR};

// Export constants needed for main.rs
pub use defaults::{LISTEN_STR, TARGET_STR, CERT_PATH_STR, KEY_PATH_STR, CA_CERT_PATH_STR, LOG_LEVEL_STR};

// Note: parse_socket_addr is now exported from common::net module
