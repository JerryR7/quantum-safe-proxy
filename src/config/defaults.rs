//! Default configuration values
//!
//! This module provides default values for configuration options.
//! It is designed to be a single source of truth for defaults,
//! making it easier to maintain consistent defaults across the application.

use std::path::PathBuf;
use std::net::SocketAddr;
use std::str::FromStr;

use super::config::ClientCertMode;

/// Environment variable prefix for all configuration options
pub const ENV_PREFIX: &str = "QUANTUM_SAFE_PROXY_";

/// Default configuration file name
pub const DEFAULT_CONFIG_FILE: &str = "config.json";

/// Default configuration directory
pub const DEFAULT_CONFIG_DIR: &str = "config";

// String constants for default values

/// Default listen address as string
pub const LISTEN_STR: &str = "0.0.0.0:8443";

/// Default target address as string
pub const TARGET_STR: &str = "127.0.0.1:6000";

/// Default certificate path as string
pub const CERT_PATH_STR: &str = "certs/hybrid/dilithium3/server.crt";

/// Default private key path as string
pub const KEY_PATH_STR: &str = "certs/hybrid/dilithium3/server.key";

/// Default CA certificate path as string
pub const CA_CERT_PATH_STR: &str = "certs/hybrid/dilithium3/ca.crt";

/// Default log level as string
pub const LOG_LEVEL_STR: &str = "info";

// Note: Environment string is now handled directly in the environment() function

// Functions for default values

/// Default listen address
pub fn listen() -> SocketAddr {
    SocketAddr::from_str(LISTEN_STR)
        .expect("Default listen address should be valid")
}

/// Default target address
pub fn target() -> SocketAddr {
    SocketAddr::from_str(TARGET_STR)
        .expect("Default target address should be valid")
}

/// Default certificate path
pub fn cert_path() -> PathBuf {
    PathBuf::from(CERT_PATH_STR)
}

/// Default private key path
pub fn key_path() -> PathBuf {
    PathBuf::from(KEY_PATH_STR)
}

/// Default CA certificate path
pub fn ca_cert_path() -> PathBuf {
    PathBuf::from(CA_CERT_PATH_STR)
}

/// Default log level
pub fn log_level() -> String {
    LOG_LEVEL_STR.to_string()
}

/// Default client certificate mode
pub fn client_cert_mode() -> ClientCertMode {
    ClientCertMode::Optional
}

/// Default buffer size (8KB)
pub fn buffer_size() -> usize {
    8192
}

/// Default connection timeout in seconds
pub fn connection_timeout() -> u64 {
    30
}

/// Default environment
pub fn environment() -> String {
    "production".to_string()
}

// Note: Command line argument names and environment variable names
// are now handled directly in the config.rs file
