//! Command-line implementation
//!
//! This module provides an implementation of the OpenSSLApiStrategy
//! using command-line tools.

use std::process::Command;
use log::{debug, warn};
use super::OpenSSLApiStrategy;

/// Command-line implementation
///
/// This struct implements the OpenSSLApiStrategy trait
/// using command-line tools.
pub struct CommandLineImpl;

impl OpenSSLApiStrategy for CommandLineImpl {
    fn check_version(&self) -> (bool, String) {
        // Try to run openssl version command
        match Command::new("openssl").arg("version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).to_string();
                let is_35_plus = version.contains("3.5");
                
                debug!("Detected OpenSSL version via command: {}", version.trim());
                (is_35_plus, version.trim().to_string())
            },
            Err(e) => {
                warn!("Failed to run openssl version command: {}", e);
                (false, "unknown (command failed)".to_string())
            },
            _ => {
                warn!("Failed to detect OpenSSL version");
                (false, "unknown (command failed)".to_string())
            }
        }
    }
    
    fn check_pqc_support(&self) -> bool {
        // Try to run openssl list -kem-algorithms command
        match Command::new("openssl").args(["list", "-kem-algorithms"]).output() {
            Ok(output) if output.status.success() => {
                let kem_list = String::from_utf8_lossy(&output.stdout);
                let has_pqc = kem_list.contains("ML-KEM");
                
                debug!("Detected PQC support via command: {}", has_pqc);
                has_pqc
            },
            Err(e) => {
                warn!("Failed to run openssl list -kem-algorithms command: {}", e);
                false
            },
            _ => {
                warn!("Failed to detect PQC support");
                false
            }
        }
    }
    
    fn get_pq_algorithms(&self) -> (Vec<String>, Vec<String>) {
        let mut key_exchange = Vec::new();
        let mut signatures = Vec::new();
        
        // Check KEM algorithms
        if let Ok(output) = Command::new("openssl").args(["list", "-kem-algorithms"]).output() {
            if output.status.success() {
                let kem_list = String::from_utf8_lossy(&output.stdout);
                
                for kem in ["ML-KEM-512", "ML-KEM-768", "ML-KEM-1024"] {
                    if kem_list.contains(kem) {
                        key_exchange.push(kem.to_string());
                        debug!("Detected post-quantum key exchange: {}", kem);
                    }
                }
            }
        }
        
        // Check signature algorithms
        if let Ok(output) = Command::new("openssl").args(["list", "-signature-algorithms"]).output() {
            if output.status.success() {
                let sig_list = String::from_utf8_lossy(&output.stdout);
                
                for sig in ["ML-DSA-44", "ML-DSA-65", "ML-DSA-87", "SLH-DSA-SHAKE-128S", "SLH-DSA-SHAKE-256S"] {
                    if sig_list.contains(sig) {
                        signatures.push(sig.to_string());
                        debug!("Detected post-quantum signature: {}", sig);
                    }
                }
            }
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
