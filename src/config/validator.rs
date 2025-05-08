//! Configuration validator
//!
//! This module provides functionality for validating configuration.

use std::path::Path;
use log::warn;

use crate::config::types::{ProxyConfig, CertStrategyType, check_file_exists};
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
    // Listen and target addresses are already validated during parsing

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
    match config.strategy() {
        CertStrategyType::Single => {
            // For Single strategy, we need either traditional or hybrid certificate
            let has_traditional = check_file_exists(config.traditional_cert()) && check_file_exists(config.traditional_key());
            let has_hybrid = check_file_exists(config.hybrid_cert()) && check_file_exists(config.hybrid_key());

            if !has_traditional && !has_hybrid {
                return Err(ConfigError::InvalidCombination(
                    "Single strategy requires either traditional or hybrid certificate and key".to_string()
                ));
            }
        }

        CertStrategyType::SigAlgs | CertStrategyType::Dynamic => {
            // For SigAlgs and Dynamic strategies, we need both traditional and hybrid certificates
            validate_file_exists(config.traditional_cert(), "Traditional certificate")?;
            validate_file_exists(config.traditional_key(), "Traditional private key")?;
            validate_file_exists(config.hybrid_cert(), "Hybrid certificate")?;
            validate_file_exists(config.hybrid_key(), "Hybrid private key")?;

            // For Dynamic strategy, PQC-only certificate is optional but if specified, both cert and key must exist
            if config.strategy() == CertStrategyType::Dynamic {
                if let Some(cert_path) = config.pqc_only_cert() {
                    validate_file_exists(cert_path, "PQC-only certificate")?;

                    if let Some(key_path) = config.pqc_only_key() {
                        validate_file_exists(key_path, "PQC-only private key")?;
                    } else {
                        return Err(ConfigError::MissingRequiredValue(
                            "PQC-only private key is required when PQC-only certificate is specified".to_string()
                        ));
                    }
                } else if config.pqc_only_key().is_some() {
                    return Err(ConfigError::MissingRequiredValue(
                        "PQC-only certificate is required when PQC-only private key is specified".to_string()
                    ));
                }
            }
        }
    }

    // Validate client CA certificate if client certificate verification is enabled
    if config.client_cert_mode().to_string() != "none" {
        validate_file_exists(config.client_ca_cert_path(), "Client CA certificate")?;
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
    ///
    /// This method checks the configuration and returns a list of warnings.
    /// Unlike validate_config, this method does not return an error if the configuration is invalid.
    fn check_warnings(&self) -> Vec<String>;
}

impl ConfigValidator for ProxyConfig {
    fn check_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check certificate paths
        if !Path::new(self.traditional_cert()).exists() {
            warnings.push(format!("Traditional certificate file not found: {}", self.traditional_cert().display()));
        }

        if !Path::new(self.traditional_key()).exists() {
            warnings.push(format!("Traditional key file not found: {}", self.traditional_key().display()));
        }

        if !Path::new(self.hybrid_cert()).exists() {
            warnings.push(format!("Hybrid certificate file not found: {}", self.hybrid_cert().display()));
        }

        if !Path::new(self.hybrid_key()).exists() {
            warnings.push(format!("Hybrid key file not found: {}", self.hybrid_key().display()));
        }

        // Check if client CA certificate exists when client cert mode is not None
        if self.client_cert_mode().to_string() != "none" && !Path::new(self.client_ca_cert_path()).exists() {
            warnings.push(format!("Client CA certificate file not found: {}", self.client_ca_cert_path().display()));
        }

        warnings
    }
}

/// Backward compatibility function for check_warnings
pub fn check_warnings(config: &ProxyConfig) -> Vec<String> {
    ConfigValidator::check_warnings(config)
}
