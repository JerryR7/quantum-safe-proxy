//! Configuration module
//!
//! This module handles application configuration, including command-line arguments,
//! environment variables, and configuration files.

mod config;
mod defaults;

pub use config::{ProxyConfig, ClientCertMode};

// Export specific items from defaults that are needed externally
pub use defaults::ENV_PREFIX;

// Note: parse_socket_addr is now exported from common::net module
