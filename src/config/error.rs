//! Configuration errors
//!
//! This module defines error types for the configuration module.

use std::fmt;
use std::error::Error;
use std::path::PathBuf;

/// Configuration error type
#[derive(Debug)]
pub enum ConfigError {
    /// File not found
    FileNotFound(PathBuf),
    
    /// Permission denied when accessing file
    FilePermissionDenied(PathBuf),
    
    /// Error reading file
    FileReadError(PathBuf, String),
    
    /// Error parsing configuration
    ParseError(String),
    
    /// Invalid value for configuration option
    InvalidValue(String, String),
    
    /// Missing required configuration value
    MissingRequiredValue(String),
    
    /// Invalid combination of configuration options
    InvalidCombination(String),
    
    /// Other error
    Other(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => 
                write!(f, "Configuration file not found: {}", path.display()),
            
            ConfigError::FilePermissionDenied(path) => 
                write!(f, "Permission denied when accessing configuration file: {}", path.display()),
            
            ConfigError::FileReadError(path, err) => 
                write!(f, "Error reading configuration file {}: {}", path.display(), err),
            
            ConfigError::ParseError(msg) => 
                write!(f, "Error parsing configuration: {}", msg),
            
            ConfigError::InvalidValue(name, msg) => 
                write!(f, "Invalid value for '{}': {}", name, msg),
            
            ConfigError::MissingRequiredValue(name) => 
                write!(f, "Missing required configuration value: {}", name),
            
            ConfigError::InvalidCombination(msg) => 
                write!(f, "Invalid combination of configuration options: {}", msg),
            
            ConfigError::Other(msg) => 
                write!(f, "Configuration error: {}", msg),
        }
    }
}

impl Error for ConfigError {}

/// Result type alias for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;

// Convert from other error types
impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => 
                ConfigError::FileNotFound(PathBuf::from("unknown")),
            
            std::io::ErrorKind::PermissionDenied => 
                ConfigError::FilePermissionDenied(PathBuf::from("unknown")),
            
            _ => ConfigError::Other(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::ParseError(err.to_string())
    }
}

impl From<std::net::AddrParseError> for ConfigError {
    fn from(err: std::net::AddrParseError) -> Self {
        ConfigError::ParseError(format!("Invalid socket address: {}", err))
    }
}

// Convert to crate's common error type
impl From<ConfigError> for crate::common::ProxyError {
    fn from(err: ConfigError) -> Self {
        crate::common::ProxyError::Config(err.to_string())
    }
}
