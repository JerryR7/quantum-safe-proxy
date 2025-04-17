//! OpenSSL capabilities detection
//!
//! This module provides functionality to detect the capabilities of the
//! OpenSSL installation, including post-quantum cryptography support.

use std::process::Command;
use log::{debug, info};
use once_cell::sync::OnceCell;

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
    /// Detect OpenSSL capabilities
    ///
    /// This function detects the capabilities of the OpenSSL installation,
    /// including whether it supports post-quantum cryptography.
    pub fn detect() -> Self {
        // Use OnceCell to cache the detection result
        static CAPABILITIES: OnceCell<OpenSSLCapabilities> = OnceCell::new();

        // Return cached detection result if available
        if let Some(capabilities) = CAPABILITIES.get() {
            return capabilities.clone();
        }

        // Default values
        const DEFAULT_CIPHER_LIST: &str = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";
        const DEFAULT_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
        const DEFAULT_GROUPS: &str = "X25519:P-256:P-384:P-521";

        // OpenSSL 3.5 specific values
        const OPENSSL35_GROUPS: &str = "X25519:P-256:P-384:P-521:X25519MLKEM768:P384MLDSA65:P256MLDSA44";
        // OpenSSL 3.5 specific values
        const HYBRID_GROUPS: &str = "X25519:P-256:P-384:P-521:X25519MLKEM768:X25519MLKEM1024:P256MLDSA44:P384MLDSA65";
        // Detect OpenSSL version and capabilities
        let openssl_version = Self::detect_openssl_version();
        let is_openssl35 = openssl_version.starts_with("3.5");

        // Log OpenSSL version
        if is_openssl35 {
            info!("Detected OpenSSL 3.5+ with built-in post-quantum support: {}", openssl_version);
        } else {
            debug!("Detected OpenSSL version: {}", openssl_version);
        }

        // Check for PQC support
        let supports_pqc = is_openssl35 && Self::has_pqc_support();

        // Detect available algorithms
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

        let groups = if supports_pqc {
            // If OpenSSL 3.5+ with PQC support, add ML-KEM algorithms
            key_exchange.push("ML-KEM".to_string());

            // Add ML-DSA and SLH-DSA algorithms
            signatures.push("ML-DSA".to_string());
            signatures.push("SLH-DSA".to_string());

            // Use PQC groups
            OPENSSL35_GROUPS.to_string()
        } else {
            // Standard OpenSSL without PQC support
            DEFAULT_GROUPS.to_string()
        };

        // Detect specific algorithms if OpenSSL 3.5+ is available
        if supports_pqc {
            Self::detect_pqc_algorithms(&mut key_exchange, &mut signatures);
        }

        let recommended_cipher_list = if supports_pqc {
            // 傳統 + 可用的 PQC cipher
            format!("{}:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256", DEFAULT_CIPHER_LIST)
        } else {
            DEFAULT_CIPHER_LIST.to_string()
        };

        let recommended_tls13_ciphersuites = if supports_pqc {
            format!(
                "{}:{}",
                DEFAULT_TLS13_CIPHERSUITES,
                "TLS_MLDSA87_WITH_AES_256_GCM_SHA384" // 假設有定義這樣的 cipher suite
            )
        } else {
            DEFAULT_TLS13_CIPHERSUITES.to_string()
        };

        let recommended_groups = if supports_pqc {
            // 混合傳統與 PQC groups（讓老 client fallback）
            format!("{}:{}", DEFAULT_GROUPS, HYBRID_GROUPS)
        } else {
            DEFAULT_GROUPS.to_string()
        };
        
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

        capabilities
    }

    /// Detect OpenSSL version
    ///
    /// This function detects the version of the OpenSSL installation.
    fn detect_openssl_version() -> String {
        // Try to run openssl version command
        match Command::new("openssl").arg("version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                // Extract version number (e.g., "OpenSSL 3.5.0 8 Apr 2025" -> "3.5.0")
                if let Some(version_str) = version.split_whitespace().nth(1) {
                    version_str.to_string()
                } else {
                    "unknown".to_string()
                }
            },
            _ => "unknown".to_string()
        }
    }

    /// Check if OpenSSL has post-quantum support
    ///
    /// This function checks if the OpenSSL installation supports
    /// post-quantum cryptography by looking for ML-KEM algorithms.
    fn has_pqc_support() -> bool {
        // Try to run openssl list -kem-algorithms command
        match Command::new("openssl").args(["list", "-kem-algorithms"]).output() {
            Ok(output) if output.status.success() => {
                let kem_list = String::from_utf8_lossy(&output.stdout);
                kem_list.contains("ML-KEM")
            },
            _ => false
        }
    }

    /// Detect post-quantum algorithms
    ///
    /// This function detects the available post-quantum algorithms
    /// in the OpenSSL installation.
    ///
    /// # Arguments
    ///
    /// * `key_exchange` - Vector to store detected key exchange algorithms
    /// * `signatures` - Vector to store detected signature algorithms
    fn detect_pqc_algorithms(key_exchange: &mut Vec<String>, signatures: &mut Vec<String>) {
        // Detect ML-KEM algorithms
        if let Ok(output) = Command::new("openssl").args(["list", "-kem-algorithms"]).output() {
            if output.status.success() {
                let kem_list = String::from_utf8_lossy(&output.stdout);

                // Look for specific ML-KEM variants
                for kem in ["ML-KEM-512", "ML-KEM-768", "ML-KEM-1024"] {
                    if kem_list.contains(kem) {
                        key_exchange.push(kem.to_string());
                        debug!("Detected post-quantum key exchange: {}", kem);
                    }
                }
            }
        }

        // Detect ML-DSA and SLH-DSA algorithms
        if let Ok(output) = Command::new("openssl").args(["list", "-signature-algorithms"]).output() {
            if output.status.success() {
                let sig_list = String::from_utf8_lossy(&output.stdout);

                // Look for specific ML-DSA variants
                for sig in ["ML-DSA-44", "ML-DSA-65", "ML-DSA-87"] {
                    if sig_list.contains(sig) {
                        signatures.push(sig.to_string());
                        debug!("Detected post-quantum signature: {}", sig);
                    }
                }

                // Look for specific SLH-DSA variants
                for sig in ["SLH-DSA-SHAKE-128S", "SLH-DSA-SHAKE-256S"] {
                    if sig_list.contains(sig) {
                        signatures.push(sig.to_string());
                        debug!("Detected post-quantum signature: {}", sig);
                    }
                }
            }
        }
    }
}
