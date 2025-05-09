//! Error handling module
//!
//! This module defines the error types and result type aliases used in the application.

use thiserror::Error;
use std::io;

/// Quantum Safe Proxy error type
#[derive(Error, Debug)]
pub enum ProxyError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// OpenSSL error
    #[error("OpenSSL error: {0}")]
    Ssl(#[from] openssl::error::ErrorStack),

    /// TLS handshake error
    #[error("TLS handshake error: {0}")]
    TlsHandshake(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Certificate error
    #[error("Certificate error: {0}")]
    Certificate(String),

    /// File not found error
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Permission denied error
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Connection timeout error
    #[error("Connection timeout after {0} seconds")]
    ConnectionTimeout(u64),

    /// Non-TLS connection error
    #[error("Non-TLS connection detected: {0}")]
    NonTlsConnection(String),

    /// Buffer pool error
    #[error("Buffer pool error: {0}")]
    BufferPool(String),

    /// Task join error
    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type alias
///
/// This is a `Result` type alias that uses our custom `ProxyError`.
pub type Result<T> = std::result::Result<T, ProxyError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        // Test IO error conversion
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let proxy_err: ProxyError = io_err.into();

        match proxy_err {
            ProxyError::Io(_) => assert!(true),
            _ => panic!("Should convert to IO error"),
        }
    }

    #[test]
    fn test_error_display() {
        // Test error display
        let err = ProxyError::Config("Invalid configuration".to_string());
        let err_str = format!("{}", err);
        assert!(err_str.contains("Invalid configuration"));
    }
}
