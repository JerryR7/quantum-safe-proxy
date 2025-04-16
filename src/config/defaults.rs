//! Default configuration values
//!
//! This module centralizes all default configuration values in one place.
//! Uses constants for better performance and clarity.

use std::path::PathBuf;
use std::net::SocketAddr;
use std::str::FromStr;
use once_cell::sync::Lazy;

use crate::config::ClientCertMode;

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

/// Default environment as string
pub const ENVIRONMENT_STR: &str = "production";

// TLS 相關設定會根據系統中的 OpenSSL 能力動態決定
// 這些設定完全由系統檢測決定，不在設定檔中提供

// Lazy-initialized complex default values

/// Default listen address
pub static LISTEN: Lazy<SocketAddr> = Lazy::new(|| {
    SocketAddr::from_str(LISTEN_STR)
        .expect("Default listen address should be valid")
});

/// Default target address
pub static TARGET: Lazy<SocketAddr> = Lazy::new(|| {
    SocketAddr::from_str(TARGET_STR)
        .expect("Default target address should be valid")
});

// Functions for backward compatibility and for use with serde defaults

/// Default listen address
pub fn listen() -> SocketAddr {
    *LISTEN
}

/// Default target address
pub fn target() -> SocketAddr {
    *TARGET
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

/// Default hybrid mode setting
pub fn hybrid_mode() -> bool {
    true
}

/// Default log level
pub fn log_level() -> String {
    LOG_LEVEL_STR.to_string()
}

/// Default client certificate mode
pub fn client_cert_mode() -> ClientCertMode {
    ClientCertMode::Required
}

/// Default configuration environment
pub fn environment() -> String {
    ENVIRONMENT_STR.to_string()
}

// TLS 相關設定完全由系統檢測決定，不再提供預設值函數
