//! OpenSSL API implementation
//!
//! This module provides an implementation of the OpenSSLApiStrategy
//! using the OpenSSL API.

#[cfg(feature = "openssl")]
use log::debug;
use super::OpenSSLApiStrategy;
use std::sync::atomic::{AtomicBool, Ordering};

// Import OpenSSL at module level with conditional compilation
#[cfg(feature = "openssl")]
use openssl;

/// OpenSSL API implementation
///
/// This struct implements the OpenSSLApiStrategy trait
/// using the OpenSSL API.
pub struct OpenSSLApiImpl;

impl OpenSSLApiStrategy for OpenSSLApiImpl {
    fn check_version(&self) -> (bool, String) {
        // Try to use the OpenSSL API
        #[cfg(feature = "openssl")]
        {
            let version_str = openssl::version::version();
            let ver_num = openssl::version::number();

            // Extract major and minor version
            let major = (ver_num >> 28) & 0xFF;
            let minor = (ver_num >> 20) & 0xFF;

            // Check if OpenSSL 3.5+
            let is_35_plus = (major == 3 && minor >= 5) || major > 3;

            // Use a static variable to track if we've already logged this
            static LOGGED: AtomicBool = AtomicBool::new(false);
            if !LOGGED.load(Ordering::Relaxed) {
                debug!("Detected OpenSSL version via API: {}", version_str);
                LOGGED.store(true, Ordering::Relaxed);
            }
            (is_35_plus, version_str.to_string())
        }
        #[cfg(not(feature = "openssl"))]
        {
            // If OpenSSL API is not available, return false
            (false, "unknown (OpenSSL API not available)".to_string())
        }
    }

    fn check_pqc_support(&self) -> bool {
        // Try to use the OpenSSL API
        #[cfg(feature = "openssl")]
        {
            // This would need to be implemented based on the actual OpenSSL 3.5 API
            // For now, we'll assume that if OpenSSL 3.5+ is available, it supports PQC
            // In a real implementation, we would check for specific PQC algorithms

            // Example implementation (would need to be updated for actual OpenSSL 3.5 API):
            // use openssl::kem::Kem;
            // Kem::fetch(None, "ML-KEM-768", None).is_some()

            // For now, we'll just return true if OpenSSL 3.5+ is available
            let (is_35_plus, _) = self.check_version();
            is_35_plus
        }
        #[cfg(not(feature = "openssl"))]
        {
            // If OpenSSL API is not available, return false
            false
        }
    }

    fn get_pq_algorithms(&self) -> (Vec<String>, Vec<String>) {
        let mut key_exchange = Vec::new();
        let mut signatures = Vec::new();

        // Try to use the OpenSSL API
        #[cfg(feature = "openssl")]
        {
            // This would need to be implemented based on the actual OpenSSL 3.5 API
            // For now, we'll add some known algorithms

            // Add known KEM algorithms
            key_exchange.extend(vec![
                "ML-KEM-512".to_string(),
                "ML-KEM-768".to_string(),
                "ML-KEM-1024".to_string(),
            ]);

            // Add known signature algorithms
            signatures.extend(vec![
                "ML-DSA-44".to_string(),
                "ML-DSA-65".to_string(),
                "ML-DSA-87".to_string(),
                "SLH-DSA-SHAKE-128S".to_string(),
                "SLH-DSA-SHAKE-256S".to_string(),
            ]);
        }

        (key_exchange, signatures)
    }

    fn get_recommended_ciphers(&self, supports_pqc: bool) -> String {
        // Default cipher list
        const DEFAULT_CIPHER_LIST: &str = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";

        if supports_pqc {
            // Add PQC ciphers if available
            format!("{}:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256", DEFAULT_CIPHER_LIST)
        } else {
            DEFAULT_CIPHER_LIST.to_string()
        }
    }

    fn get_recommended_tls13_ciphersuites(&self, supports_pqc: bool) -> String {
        // Default TLS 1.3 ciphersuites
        const DEFAULT_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";

        if supports_pqc {
            // Add PQC ciphersuites if available
            format!("{}:{}", DEFAULT_TLS13_CIPHERSUITES, "TLS_MLDSA87_WITH_AES_256_GCM_SHA384")
        } else {
            DEFAULT_TLS13_CIPHERSUITES.to_string()
        }
    }

    fn get_recommended_groups(&self, supports_pqc: bool) -> String {
        // Default groups
        const DEFAULT_GROUPS: &str = "X25519:P-256:P-384:P-521";

        // PQC groups
        const PQC_GROUPS: &str = "X25519MLKEM768:P384MLDSA65:P256MLDSA44";

        if supports_pqc {
            // Add PQC groups if available
            format!("{}:{}", DEFAULT_GROUPS, PQC_GROUPS)
        } else {
            DEFAULT_GROUPS.to_string()
        }
    }
}
