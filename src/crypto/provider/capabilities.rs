//! OpenSSL capabilities detection
//!
//! This module provides functionality to detect the capabilities of the
//! OpenSSL installation, including post-quantum cryptography support.

use log::{debug, info};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};

use super::api;
use super::environment;

/// OpenSSL capabilities
///
/// This structure holds the capabilities of the OpenSSL installation,
/// including whether it supports post-quantum cryptography.
#[derive(Debug, Clone)]
pub struct OpenSSLCapabilities {
    /// Whether OpenSSL supports post-quantum cryptography
    pub supports_pqc: bool,

    /// Supported key exchange algorithms
    pub supported_key_exchange: Vec<String>,

    /// Supported signature algorithms
    pub supported_signatures: Vec<String>,

    /// Recommended TLS cipher list
    pub recommended_cipher_list: String,

    /// Recommended TLS 1.3 ciphersuites
    pub recommended_tls13_ciphersuites: String,

    /// Recommended TLS groups
    pub recommended_groups: String,
}

impl OpenSSLCapabilities {
    /// Create a new OpenSSL capabilities object
    ///
    /// This function creates a new OpenSSL capabilities object
    /// with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `supports_pqc` - Whether OpenSSL supports post-quantum cryptography
    /// * `key_exchange` - Supported key exchange algorithms
    /// * `signatures` - Supported signature algorithms
    /// * `cipher_list` - Recommended TLS cipher list
    /// * `tls13_ciphersuites` - Recommended TLS 1.3 ciphersuites
    /// * `groups` - Recommended TLS groups
    #[allow(dead_code)]
    pub fn new(
        supports_pqc: bool,
        key_exchange: Vec<String>,
        signatures: Vec<String>,
        cipher_list: String,
        tls13_ciphersuites: String,
        groups: String,
    ) -> Self {
        Self {
            supports_pqc,
            supported_key_exchange: key_exchange,
            supported_signatures: signatures,
            recommended_cipher_list: cipher_list,
            recommended_tls13_ciphersuites: tls13_ciphersuites,
            recommended_groups: groups,
        }
    }

    /// Detect OpenSSL capabilities
    ///
    /// This function detects the capabilities of the OpenSSL installation,
    /// including whether it supports post-quantum cryptography.
    pub fn detect() -> Self {
        // Use OnceCell to cache the detection result
        static CAPABILITIES: OnceCell<OpenSSLCapabilities> = OnceCell::new();
        static INITIALIZED: AtomicBool = AtomicBool::new(false);

        // Return cached detection result if available
        if INITIALIZED.load(Ordering::Relaxed) {
            debug!("Using cached OpenSSLCapabilities");
            return CAPABILITIES.get().unwrap().clone();
        }

        debug!("Detecting OpenSSL capabilities");
        INITIALIZED.store(true, Ordering::Relaxed);

        // Get environment info
        let env_info = environment::initialize_environment();

        // Log OpenSSL version
        if env_info.openssl_version.contains("3.5") {
            info!("Detected OpenSSL 3.5+ with built-in post-quantum support: {}", env_info.openssl_version);
        } else {
            debug!("Detected OpenSSL version: {}", env_info.openssl_version);
        }

        // Check for PQC support
        let supports_pqc = env_info.pqc_available;

        // Get basic algorithms
        let mut key_exchange = vec![
            "RSA".to_string(),
            "ECDHE".to_string(),
            "DHE".to_string(),
        ];

        let mut signatures = vec![
            "RSA".to_string(),
            "ECDSA".to_string(),
            "DSA".to_string(),
        ];

        // Add post-quantum algorithms if available
        if supports_pqc {
            key_exchange.extend(env_info.key_exchange_algorithms.clone());
            signatures.extend(env_info.signature_algorithms.clone());
        }

        // Get recommended cipher list, ciphersuites, and groups
        let recommended_cipher_list = api::get_recommended_cipher_list(supports_pqc);
        let recommended_tls13_ciphersuites = api::get_recommended_tls13_ciphersuites(supports_pqc);
        let recommended_groups = api::get_recommended_groups(supports_pqc);

        // Create capabilities
        let capabilities = Self {
            supports_pqc,
            supported_key_exchange: key_exchange,
            supported_signatures: signatures,
            recommended_cipher_list,
            recommended_tls13_ciphersuites,
            recommended_groups,
        };

        // Cache the result
        let _ = CAPABILITIES.set(capabilities.clone());

        // Log capabilities
        if supports_pqc {
            info!("OpenSSL supports post-quantum cryptography");
        } else if env_info.openssl_version.contains("3.5") {
            info!("OpenSSL 3.5+ detected but post-quantum cryptography not available");
        } else {
            info!("OpenSSL does not support post-quantum cryptography");
        }

        capabilities
    }
}
