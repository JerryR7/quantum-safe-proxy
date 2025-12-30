//! Configuration validator
//!
//! This module provides functionality for validating configuration.

use std::path::Path;
use log::warn;

use crate::config::types::{ProxyConfig, check_file_exists};
use crate::config::error::{ConfigError, Result};

/// Validate the configuration
pub fn validate_config(config: &ProxyConfig) -> Result<()> {
    // Validate network settings
    validate_network_settings(config)?;

    // Validate certificate settings
    validate_certificate_settings(config)?;

    // Validate general settings
    validate_general_settings(config)?;

    Ok(())
}

/// Validate network settings
fn validate_network_settings(config: &ProxyConfig) -> Result<()> {
    // Check that listen and target addresses are different
    if config.listen() == config.target() {
        return Err(ConfigError::InvalidCombination(
            "Listen and target addresses must be different".to_string()
        ));
    }

    Ok(())
}

/// Validate certificate settings
fn validate_certificate_settings(config: &ProxyConfig) -> Result<()> {
    // Primary certificate is always required
    validate_file_exists(config.cert(), "Primary certificate")?;
    validate_file_exists(config.key(), "Primary private key")?;

    // If fallback is configured, both cert and key must exist
    if config.has_fallback() {
        if let Some(cert) = config.fallback_cert() {
            validate_file_exists(cert, "Fallback certificate")?;
        }
        if let Some(key) = config.fallback_key() {
            validate_file_exists(key, "Fallback private key")?;
        }
    }

    // Validate client CA certificate if client certificate verification is enabled
    if config.client_cert_mode().to_string() != "none" {
        validate_file_exists(config.client_ca_cert(), "Client CA certificate")?;
    }

    Ok(())
}

/// Validate general settings
fn validate_general_settings(config: &ProxyConfig) -> Result<()> {
    // Validate log level
    match config.log_level() {
        "error" | "warn" | "info" | "debug" | "trace" => {}
        level => {
            warn!("Invalid log level: {}. Using default: info", level);
        }
    }

    // Validate buffer size
    if config.buffer_size() == 0 {
        return Err(ConfigError::InvalidValue(
            "buffer_size".to_string(),
            "Buffer size must be greater than 0".to_string()
        ));
    }

    // Validate connection timeout
    if config.connection_timeout() == 0 {
        return Err(ConfigError::InvalidValue(
            "connection_timeout".to_string(),
            "Connection timeout must be greater than 0".to_string()
        ));
    }

    // Validate OpenSSL directory if specified
    if let Some(dir) = config.openssl_dir() {
        if !dir.exists() || !dir.is_dir() {
            return Err(ConfigError::InvalidValue(
                "openssl_dir".to_string(),
                format!("OpenSSL directory does not exist or is not a directory: {}", dir.display())
            ));
        }
    }

    Ok(())
}

/// Validate that a file exists
fn validate_file_exists(path: &Path, _description: &str) -> Result<()> {
    if !check_file_exists(path) {
        return Err(ConfigError::FileNotFound(path.to_path_buf()));
    }

    Ok(())
}

/// Configuration validator trait
pub trait ConfigValidator {
    /// Check configuration for warnings
    fn check_warnings(&self) -> Vec<String>;
}

impl ConfigValidator for ProxyConfig {
    fn check_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check log level
        match self.log_level() {
            "error" | "warn" | "info" | "debug" | "trace" => {}
            level => {
                warnings.push(format!("Invalid log level '{}', using default 'info'", level));
            }
        }

        // Check if primary certificate files exist
        if !check_file_exists(self.cert()) {
            warnings.push(format!(
                "Primary certificate file not found: {}",
                self.cert().display()
            ));
        }

        if !check_file_exists(self.key()) {
            warnings.push(format!(
                "Primary key file not found: {}",
                self.key().display()
            ));
        }

        // Check fallback certificates if configured
        if let Some(cert) = self.fallback_cert() {
            if !check_file_exists(cert) {
                warnings.push(format!(
                    "Fallback certificate file not found: {}",
                    cert.display()
                ));
            }
        }

        if let Some(key) = self.fallback_key() {
            if !check_file_exists(key) {
                warnings.push(format!(
                    "Fallback key file not found: {}",
                    key.display()
                ));
            }
        }

        warnings
    }
}

/// Check configuration for warnings (standalone function for backward compatibility)
pub fn check_warnings(config: &ProxyConfig) -> Vec<String> {
    ConfigValidator::check_warnings(config)
}
