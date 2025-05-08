//! Configuration logger implementation
//!
//! This module provides functionality for logging configuration.

use log::info;
use crate::config::traits::ConfigLogger;
use crate::config::types::ProxyConfig;

impl ConfigLogger for ProxyConfig {
    /// Log the configuration with source information
    fn log(&self) {
        // Only log in info level or below
        if !log::log_enabled!(log::Level::Info) {
            return;
        }

        info!("=== Final Configuration ===");

        // Network settings
        info!("Network Settings:");
        info!("  Listen address: {}", self.listen);
        info!("  Target address: {}", self.target);

        // General settings
        info!("General Settings:");
        info!("  Log level: {}", self.log_level);
        info!("  Client certificate mode: {}", self.client_cert_mode);
        info!("  Buffer size: {} bytes", self.buffer_size);
        info!("  Connection timeout: {} seconds", self.connection_timeout);

        // Certificate strategy settings
        info!("Certificate Strategy Settings:");
        info!("  Strategy: {:?}", self.strategy);
        info!("  Traditional certificate: {}", self.traditional_cert.display());
        info!("  Traditional key: {}", self.traditional_key.display());
        info!("  Hybrid certificate: {}", self.hybrid_cert.display());
        info!("  Hybrid key: {}", self.hybrid_key.display());

        if let Some(ref cert) = self.pqc_only_cert {
            info!("  PQC-only certificate: {}", cert.display());
        }

        if let Some(ref key) = self.pqc_only_key {
            info!("  PQC-only key: {}", key.display());
        }

        info!("  Client CA certificate: {}", self.client_ca_cert_path.display());

        // OpenSSL directory
        if let Some(ref dir) = self.openssl_dir {
            info!("  OpenSSL directory: {}", dir.display());
        }

        info!("=========================");
    }
}

/// Log the configuration with source information
///
/// This is a convenience function that calls the `log` method on the configuration.
pub fn log_config(config: &ProxyConfig) {
    config.log();
}
