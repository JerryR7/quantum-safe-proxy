//! Configuration validation functionality
//!
//! This module provides functionality for validating configuration.

use std::path::Path;

use crate::common::{ProxyError, Result};
use crate::config::{ProxyConfig, CertStrategyType, check_file_exists};

/// Trait for validating configuration
pub trait ConfigValidator {
    /// Validate configuration
    ///
    /// Checks if certificate files exist and other configuration is valid.
    fn validate(&self) -> Result<()>;

    /// Check configuration for potential issues
    ///
    /// This method checks the configuration for potential issues and returns
    /// a list of warnings. Unlike `validate()`, this method does not return
    /// an error if issues are found.
    fn check(&self) -> Vec<String>;
}

impl ConfigValidator for ProxyConfig {
    fn validate(&self) -> Result<()> {
        // Validate network settings
        if self.listen.port() == 0 {
            return Err(ProxyError::Config("Invalid listen port: 0 (random port not supported)".to_string()));
        }

        // Parse target string to check port
        if let Ok(addr) = self.resolve_target() {
            if addr.port() == 0 {
                return Err(ProxyError::Config("Invalid target port: 0 (random port not supported)".to_string()));
            }
        } else {
            return Err(ProxyError::Config(format!("Invalid target address: {}", self.target)));
        }

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => {
                return Err(ProxyError::Config(format!(
                    "Invalid log level: {}. Valid values are: debug, info, warn, error",
                    self.log_level
                )));
            },
        }

        // Validate certificate files based on strategy
        self.validate_certificate_files()
    }

    fn check(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Helper function to check if a file exists and add a warning if it doesn't
        let check_file = |path: &Path, file_type: &str, warnings: &mut Vec<String>| {
            if !path.exists() {
                warnings.push(format!("{} file does not exist: {}", file_type, path.display()));
            }
        };

        // Check certificate files based on strategy
        match self.strategy {
            CertStrategyType::Single => {
                // Single strategy only requires hybrid certificate
                check_file(&self.hybrid_cert, "Hybrid certificate", &mut warnings);
                check_file(&self.hybrid_key, "Hybrid key", &mut warnings);
            },
            CertStrategyType::SigAlgs | CertStrategyType::Dynamic => {
                // Both SigAlgs and Dynamic strategies require traditional and hybrid certificates
                check_file(&self.traditional_cert, "Traditional certificate", &mut warnings);
                check_file(&self.traditional_key, "Traditional key", &mut warnings);
                check_file(&self.hybrid_cert, "Hybrid certificate", &mut warnings);
                check_file(&self.hybrid_key, "Hybrid key", &mut warnings);

                // Check PQC-only certificates if specified (only relevant for Dynamic strategy)
                if let Some(cert) = &self.pqc_only_cert {
                    check_file(cert, "PQC-only certificate", &mut warnings);
                }
                if let Some(key) = &self.pqc_only_key {
                    check_file(key, "PQC-only key", &mut warnings);
                }
            },
        }

        // Always check client CA certificate
        check_file(&self.client_ca_cert_path, "Client CA certificate", &mut warnings);

        // Check network settings
        if self.listen.port() == 0 {
            warnings.push(format!(
                "Listen address has port 0, which will use a random port: {}",
                self.listen
            ));
        }

        // Parse target string to check port
        if let Ok(addr) = self.resolve_target() {
            if addr.port() == 0 {
                warnings.push(format!(
                    "Target address has port 0, which may not work as expected: {}",
                    self.target
                ));
            }
        } else {
            warnings.push(format!("Invalid target address: {}", self.target));
        }

        // Check log level
        match self.log_level.to_lowercase().as_str() {
            "debug" | "info" | "warn" | "error" => {},
            _ => warnings.push(format!("Unknown log level: {}", self.log_level)),
        }

        warnings
    }
}

// Private implementation details
impl ProxyConfig {
    /// Validate certificate files based on the selected strategy
    fn validate_certificate_files(&self) -> Result<()> {
        // Helper function to validate a certificate file with a custom error message
        fn validate_cert_file(path: &Path, file_type: &str) -> Result<()> {
            check_file_exists(path).map_err(|_| {
                ProxyError::Config(format!(
                    "{} file does not exist or is invalid: {}",
                    file_type,
                    path.display()
                ))
            })
        }

        // Always validate client CA certificate
        validate_cert_file(&self.client_ca_cert_path, "Client CA certificate")?;

        // Validate required certificates based on strategy
        match self.strategy {
            CertStrategyType::Single => {
                // Single strategy only requires hybrid certificate
                validate_cert_file(&self.hybrid_cert, "Hybrid certificate")?;
                validate_cert_file(&self.hybrid_key, "Hybrid key")?;
            },
            CertStrategyType::SigAlgs => {
                // SigAlgs strategy requires both traditional and hybrid certificates
                validate_cert_file(&self.traditional_cert, "Traditional certificate")?;
                validate_cert_file(&self.traditional_key, "Traditional key")?;
                validate_cert_file(&self.hybrid_cert, "Hybrid certificate")?;
                validate_cert_file(&self.hybrid_key, "Hybrid key")?;
            },
            CertStrategyType::Dynamic => {
                // Dynamic strategy requires traditional and hybrid certificates
                // and optionally PQC-only certificates
                validate_cert_file(&self.traditional_cert, "Traditional certificate")?;
                validate_cert_file(&self.traditional_key, "Traditional key")?;
                validate_cert_file(&self.hybrid_cert, "Hybrid certificate")?;
                validate_cert_file(&self.hybrid_key, "Hybrid key")?;

                // Check PQC-only certificates if specified
                if let Some(cert) = &self.pqc_only_cert {
                    validate_cert_file(cert, "PQC-only certificate")?;
                }
                if let Some(key) = &self.pqc_only_key {
                    validate_cert_file(key, "PQC-only key")?;
                }
            },
        }

        Ok(())
    }
}
