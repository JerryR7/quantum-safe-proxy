//! OpenSSL capabilities' detection
//!
//! This module provides functionality to detect the capabilities of the
//! OpenSSL installation, including post-quantum cryptography support.
//! It also provides utility functions for working with OpenSSL.

use log::{debug, info};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};

use super::environment;

/// Get OpenSSL version
///
/// # Returns
///
/// The OpenSSL version string
pub fn get_openssl_version() -> String {
    let env_info = environment::initialize_environment();
    env_info.openssl_version.clone()
}

/// Check if OpenSSL 3.5+ is available
///
/// # Returns
///
/// `true` if OpenSSL 3.5+ is available, `false` otherwise
pub fn is_openssl35_available() -> bool {
    let version = get_openssl_version();
    version.contains("3.5") || version.contains("3.6")
}

/// Check if post-quantum cryptography is available
///
/// # Returns
///
/// `true` if post-quantum cryptography is available, `false` otherwise
pub fn is_pqc_available() -> bool {
    let env_info = environment::initialize_environment();
    env_info.pqc_available
}

/// Get supported post-quantum algorithms
///
/// # Returns
///
/// A tuple containing:
/// - A vector of supported key exchange algorithms
/// - A vector of supported signature algorithms
pub fn get_supported_pq_algorithms() -> (Vec<String>, Vec<String>) {
    let env_info = environment::initialize_environment();
    (
        env_info.key_exchange_algorithms.clone(),
        env_info.signature_algorithms.clone(),
    )
}

/// Get recommended cipher list
///
/// This function returns the recommended cipher list based on
/// whether post-quantum cryptography is supported.
///
/// # Arguments
///
/// * `supports_pqc` - Whether post-quantum cryptography is supported
///
/// # Returns
///
/// The recommended cipher list
pub fn get_recommended_cipher_list(supports_pqc: bool) -> String {
    // Default cipher list
    const DEFAULT_CIPHER_LIST: &str = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";

    if supports_pqc {
        // Add PQC ciphers if available
        format!("{0}:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256", DEFAULT_CIPHER_LIST)
    } else {
        DEFAULT_CIPHER_LIST.to_string()
    }
}

/// Get recommended TLS 1.3 ciphersuites
///
/// This function returns the recommended TLS 1.3 ciphersuites based on
/// whether post-quantum cryptography is supported.
///
/// # Arguments
///
/// * `supports_pqc` - Whether post-quantum cryptography is supported
///
/// # Returns
///
/// The recommended TLS 1.3 ciphersuites
pub fn get_recommended_tls13_ciphersuites(supports_pqc: bool) -> String {
    // Default TLS 1.3 ciphersuites
    const DEFAULT_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";

    if supports_pqc {
        // Add PQC ciphersuites if available
        format!("{0}:{1}", DEFAULT_TLS13_CIPHERSUITES, "TLS_MLDSA87_WITH_AES_256_GCM_SHA384")
    } else {
        DEFAULT_TLS13_CIPHERSUITES.to_string()
    }
}

/// Get recommended groups
///
/// This function returns the recommended groups based on
/// whether post-quantum cryptography is supported.
///
/// # Arguments
///
/// * `supports_pqc` - Whether post-quantum cryptography is supported
///
/// # Returns
///
/// The recommended groups
pub fn get_recommended_groups(supports_pqc: bool) -> String {
    // Default groups
    const DEFAULT_GROUPS: &str = "X25519:P-256:P-384:P-521";

    // PQC groups
    const PQC_GROUPS: &str = "X25519MLKEM768:P384MLDSA65:P256MLDSA44";

    if supports_pqc {
        // Add PQC groups if available
        format!("{0}:{1}", DEFAULT_GROUPS, PQC_GROUPS)
    } else {
        DEFAULT_GROUPS.to_string()
    }
}

/// Get supported signature algorithms
///
/// # Returns
///
/// The supported signature algorithms
pub fn get_supported_signature_algorithms() -> Vec<String> {
    let mut algorithms = vec![
        "RSA-PSS+SHA256".to_string(),
        "ECDSA+SHA256".to_string(),
        "RSA+SHA256".to_string(),
    ];

    if is_pqc_available() {
        algorithms.push("dilithium3".to_string());
        algorithms.push("falcon512".to_string());
        algorithms.push("p384_dilithium3".to_string());
    }

    algorithms
}

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
        let recommended_cipher_list = get_recommended_cipher_list(supports_pqc);
        let recommended_tls13_ciphersuites = get_recommended_tls13_ciphersuites(supports_pqc);
        let recommended_groups = get_recommended_groups(supports_pqc);

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
