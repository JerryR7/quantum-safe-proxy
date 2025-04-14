//! Default configuration values
//!
//! This module centralizes all default configuration values in one place.

use std::path::PathBuf;
use std::net::SocketAddr;
use std::str::FromStr;

use crate::config::ClientCertMode;

/// Environment variable prefix for all configuration options
pub const ENV_PREFIX: &str = "QUANTUM_SAFE_PROXY_";

// Default values as strings for use with clap

/// Default listen address as string
pub fn listen_str() -> String {
    "0.0.0.0:8443".to_string()
}

/// Default target address as string
pub fn target_str() -> String {
    "127.0.0.1:6000".to_string()
}

/// Default certificate path as string
pub fn cert_path_str() -> String {
    "certs/hybrid/dilithium3/server.crt".to_string()
}

/// Default private key path as string
pub fn key_path_str() -> String {
    "certs/hybrid/dilithium3/server.key".to_string()
}

/// Default CA certificate path as string
pub fn ca_cert_path_str() -> String {
    "certs/hybrid/dilithium3/ca.crt".to_string()
}

/// Default log level as string
pub fn log_level_str() -> String {
    "info".to_string()
}

/// Default environment as string
pub fn environment_str() -> String {
    "production".to_string()
}

/// Default listen address
pub fn listen() -> SocketAddr {
    SocketAddr::from_str(&listen_str())
        .expect("Default listen address should be valid")
}

/// Default target address
pub fn target() -> SocketAddr {
    SocketAddr::from_str(&target_str())
        .expect("Default target address should be valid")
}

/// Default certificate path
pub fn cert_path() -> PathBuf {
    PathBuf::from(cert_path_str())
}

/// Default private key path
pub fn key_path() -> PathBuf {
    PathBuf::from(key_path_str())
}

/// Default CA certificate path
pub fn ca_cert_path() -> PathBuf {
    PathBuf::from(ca_cert_path_str())
}

/// Default hybrid mode setting
pub fn hybrid_mode() -> bool {
    true
}

/// Default log level
pub fn log_level() -> String {
    log_level_str()
}

/// Default client certificate mode
pub fn client_cert_mode() -> ClientCertMode {
    ClientCertMode::Required
}

/// Default configuration environment
pub fn environment() -> String {
    environment_str()
}
