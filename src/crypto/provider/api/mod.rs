//! OpenSSL API strategy module
//!
//! This module provides strategies for interacting with OpenSSL,
//! either through direct API calls or command-line tools.

mod openssl_api;
mod command_line;
mod factory;

use super::environment;

pub use factory::get_api_strategy;
use openssl_api::OpenSSLApiImpl;
#[cfg(not(feature = "openssl"))]
use command_line::CommandLineImpl;

/// OpenSSL API strategy trait
///
/// This trait defines the interface for interacting with OpenSSL.
pub trait OpenSSLApiStrategy: Send + Sync {
    /// Check OpenSSL version
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - A boolean indicating if OpenSSL 3.5+ is available
    /// - The OpenSSL version string
    fn check_version(&self) -> (bool, String);

    /// Check if post-quantum cryptography is supported
    ///
    /// # Returns
    ///
    /// `true` if post-quantum cryptography is supported, `false` otherwise
    fn check_pqc_support(&self) -> bool;

    /// Get supported post-quantum algorithms
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - A vector of supported key exchange algorithms
    /// - A vector of supported signature algorithms
    fn get_pq_algorithms(&self) -> (Vec<String>, Vec<String>);

    /// Get recommended cipher list
    ///
    /// # Arguments
    ///
    /// * `supports_pqc` - Whether post-quantum cryptography is supported
    ///
    /// # Returns
    ///
    /// The recommended cipher list
    fn get_recommended_ciphers(&self, supports_pqc: bool) -> String;

    /// Get recommended TLS 1.3 ciphersuites
    ///
    /// # Arguments
    ///
    /// * `supports_pqc` - Whether post-quantum cryptography is supported
    ///
    /// # Returns
    ///
    /// The recommended TLS 1.3 ciphersuites
    fn get_recommended_tls13_ciphersuites(&self, supports_pqc: bool) -> String;

    /// Get recommended groups
    ///
    /// # Arguments
    ///
    /// * `supports_pqc` - Whether post-quantum cryptography is supported
    ///
    /// # Returns
    ///
    /// The recommended groups
    fn get_recommended_groups(&self, supports_pqc: bool) -> String;
}

// No longer needed, using environment info instead

/// Check if OpenSSL 3.5+ is available
///
/// This function checks if OpenSSL 3.5+ is available in the system.
/// The result is cached to avoid repeated checks.
///
/// # Returns
///
/// `true` if OpenSSL 3.5+ is available, `false` otherwise
pub fn is_openssl35_available() -> bool {
    // Use the environment info to check OpenSSL version
    let env_info = environment::initialize_environment();

    // Check if OpenSSL 3.5+ is available
    env_info.openssl_version.contains("3.5")
}

/// Get OpenSSL version string
///
/// This function returns the OpenSSL version string.
/// The result is cached to avoid repeated checks.
///
/// # Returns
///
/// The OpenSSL version string
pub fn get_openssl_version() -> &'static str {
    // Use the environment info to get OpenSSL version
    let env_info = environment::initialize_environment();

    // Return the OpenSSL version
    &env_info.openssl_version
}

// No longer needed, using environment info instead

/// Check if post-quantum cryptography is available
///
/// This function checks if post-quantum cryptography is available
/// in the OpenSSL installation. The result is cached to avoid repeated checks.
///
/// # Returns
///
/// `true` if post-quantum cryptography is available, `false` otherwise
pub fn is_pqc_available() -> bool {
    // Use the environment info to check PQC support
    let env_info = environment::initialize_environment();

    // Check if PQC is available
    env_info.pqc_available
}

// No longer needed, using environment info instead

/// Get supported post-quantum algorithms
///
/// This function returns the supported post-quantum algorithms.
/// The result is cached to avoid repeated checks.
///
/// # Returns
///
/// A tuple containing:
/// - A vector of supported key exchange algorithms
/// - A vector of supported signature algorithms
pub fn get_supported_pq_algorithms() -> (Vec<String>, Vec<String>) {
    // Use the environment info to get PQC algorithms
    let env_info = environment::initialize_environment();

    // Return the PQC algorithms from the environment info
    (env_info.key_exchange_algorithms.clone(), env_info.signature_algorithms.clone())
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
