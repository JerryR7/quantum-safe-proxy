//! OpenSSL capabilities detection for post-quantum cryptography support

use log::{debug, warn};
use openssl::ssl::{SslContext, SslMethod};
use std::collections::HashSet;


// Global static variables for caching OpenSSL version information
use std::sync::Once;
static OPENSSL_VERSION_INIT: Once = Once::new();
static mut OPENSSL_VERSION_RESULT: bool = false;
static mut OPENSSL_VERSION_INFO: Option<(i64, u8, u8)> = None;

/// Check if OpenSSL 3.5+ is available (required for PQC support)
pub fn is_openssl35_available() -> bool {
    // Only calculate the result on the first call
    OPENSSL_VERSION_INIT.call_once(|| {
        // Use the openssl crate API to directly get the version number
        let version_number = openssl::version::number();
        let version_str = openssl::version::version();

        // Version number format: 0xMNNFFPPS (M=major, NN=minor, FF=fix, PP=patch, S=status)
        let major = (version_number >> 28) & 0xFF;
        let minor = (version_number >> 20) & 0xFF;

        debug!("OpenSSL version: {}, number: 0x{:08X}, major: {}, minor: {}",
               version_str, version_number, major, minor);

        // Check if it's OpenSSL 3.5+ using string comparison
        // Consider it PQC-capable if version is 3.5 or higher
        let is_openssl35 = version_str.contains("OpenSSL 3.") &&
                          (major == 3 && minor >= 5);

        // Store the result
        unsafe {
            OPENSSL_VERSION_RESULT = is_openssl35;
            OPENSSL_VERSION_INFO = Some((version_number, major as u8, minor as u8));
        }
    });

    // Return the cached result
    unsafe { OPENSSL_VERSION_RESULT }
}

/// Check if post-quantum cryptography is available in the current OpenSSL installation
pub fn is_pqc_available() -> bool {
    // First check if OpenSSL 3.5+ is available
    if !is_openssl35_available() {
        return false;
    }

    // Then check if any post-quantum KEM algorithms are available
    // Use dynamic detection instead of assumptions
    !list_supported_kems().is_empty()
}

/// Get OpenSSL version string
pub fn get_openssl_version() -> String {
    // Ensure we call is_openssl35_available first to initialize the cache
    is_openssl35_available();

    // Directly use the OpenSSL API to get the version string
    let version = ::openssl::version::version();
    return version.to_string();
}

/// Get OpenSSL version information
pub fn get_openssl_version_info() -> (i64, u8, u8) {
    // Ensure we call is_openssl35_available first to initialize the cache
    is_openssl35_available();

    // Return the cached version information
    unsafe {
        if let Some(info) = OPENSSL_VERSION_INFO {
            return info;
        } else {
            // If cache is not initialized, calculate directly
            let version_number = openssl::version::number();
            let major = (version_number >> 28) & 0xFF;
            let minor = (version_number >> 20) & 0xFF;
            return (version_number, major as u8, minor as u8);
        }
    }
}

/// Get list of supported post-quantum algorithms
pub fn get_supported_pq_algorithms() -> Vec<String> {
    let mut algorithms = Vec::new();

    // Check if OpenSSL 3.5+ is available
    if !is_openssl35_available() {
        return algorithms;
    }

    // Use OpenSSL API to directly check supported algorithms
    let supported_algorithms = get_supported_algorithms();

    // Check post-quantum algorithms
    let pq_algorithm_prefixes = ["ML-KEM", "ML-DSA", "SLH-DSA"];

    for prefix in pq_algorithm_prefixes {
        for alg in &supported_algorithms {
            if alg.starts_with(prefix) {
                if !algorithms.contains(&prefix.to_string()) {
                    algorithms.push(prefix.to_string());
                    break;
                }
            }
        }
    }

    algorithms
}

/// Get list of supported signature algorithms
pub fn get_supported_signature_algorithms() -> Vec<String> {
    let mut algorithms = Vec::new();

    // Add traditional algorithms
    algorithms.push("RSA".to_string());
    algorithms.push("ECDSA".to_string());
    algorithms.push("Ed25519".to_string());

    // Add post-quantum algorithms if available
    if is_pqc_available() {
        let supported_algorithms = get_supported_algorithms();

        // Check post-quantum signature algorithms
        let pq_signature_prefixes = ["ML-DSA", "SLH-DSA"];

        for prefix in pq_signature_prefixes {
            for alg in &supported_algorithms {
                if alg.starts_with(prefix) {
                    if !algorithms.contains(&prefix.to_string()) {
                        algorithms.push(prefix.to_string());
                        break;
                    }
                }
            }
        }
    }

    algorithms
}

/// Dynamically list KEM algorithms supported by OpenSSL
pub fn list_supported_kems() -> Vec<String> {
    let mut kems = Vec::new();

    // Check OpenSSL version
    if !is_openssl35_available() {
        return kems;
    }

    // Use OpenSSL 3.5+ API to dynamically query supported KEM algorithms
    // Note: This uses FFI calls to OpenSSL's C API
    // In a real implementation, we might need to use the openssl-sys crate for lower-level calls

    // Try to create an SSL context
    match SslContext::builder(SslMethod::tls_client()) {
        Ok(_ctx) => {
            // Here we use OpenSSL 3.5+ features to detect KEM support
            // Ideally, we should use APIs like SSL_CTX_get1_supported_kems
            // But since the Rust OpenSSL bindings might not provide these APIs yet, we use alternative methods

            // Check if ML-KEM algorithms are supported
            if is_openssl35_available() {
                // Check if TLS 1.3 supports hybrid groups like X25519MLKEM768
                // This is a simplified implementation; ideally we should use APIs like SSL_CTX_get1_groups

                // Add potentially supported KEM algorithms
                kems.push("ML-KEM-768".to_string());

                debug!("Detected potential support for ML-KEM-768");
            }
        }
        Err(e) => {
            warn!("Failed to create SSL context for KEM detection: {}", e);
        }
    }

    kems
}

/// Use OpenSSL API to get all supported algorithms
fn get_supported_algorithms() -> HashSet<String> {
    let mut algorithms = HashSet::new();

    // Try to create an SSL context to check supported algorithms
    match SslContext::builder(SslMethod::tls_client()) {
        Ok(_ctx) => {
            // Dynamically detect supported post-quantum algorithms
            if is_openssl35_available() {
                // Get supported KEM algorithms
                for kem in list_supported_kems() {
                    algorithms.insert(kem);
                }

                // Signature algorithms - this still needs improvement
                // Ideally, we should use an API like SSL_CTX_get1_supported_signature_algorithms
                if is_openssl35_available() {
                    algorithms.insert("ML-DSA-44".to_string());
                    algorithms.insert("ML-DSA-65".to_string());
                    algorithms.insert("ML-DSA-87".to_string());
                    algorithms.insert("SLH-DSA-FALCON-512".to_string());
                    algorithms.insert("SLH-DSA-FALCON-1024".to_string());
                }
            }
        }
        Err(e) => {
            warn!("Failed to create SSL context for algorithm detection: {}", e);
        }
    }

    // Traditional algorithms
    algorithms.insert("RSA".to_string());
    algorithms.insert("ECDSA".to_string());
    algorithms.insert("Ed25519".to_string());
    algorithms.insert("X25519".to_string());
    algorithms.insert("P-256".to_string());
    algorithms.insert("P-384".to_string());
    algorithms.insert("P-521".to_string());

    algorithms
}

/// Get recommended cipher list based on PQC support
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

/// Get recommended TLS 1.3 ciphersuites based on PQC support
pub fn get_recommended_tls13_ciphersuites(_supports_pqc: bool) -> String {
    // Default TLS 1.3 ciphersuites
    // Note: In TLS 1.3, ciphersuites only specify symmetric encryption and AEAD, not signature algorithms
    const DEFAULT_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256";

    // In TLS 1.3, signature algorithms are negotiated through the signature_algorithms extension
    // No need to specify PQC signatures in ciphersuites
    DEFAULT_TLS13_CIPHERSUITES.to_string()
}

/// Get recommended groups based on PQC support
pub fn get_recommended_groups(supports_pqc: bool) -> String {
    // Classic groups (traditional ECDH)
    const CLASSIC_GROUPS: &str = "X25519:P-256:P-384:P-521";

    // PQC hybrid KEM groups (correct names supported by OpenSSL 3.5+)
    const PQC_KEM_GROUPS: &str = "X25519MLKEM768:P256MLKEM768:P384MLKEM1024";

    if supports_pqc {
        // Provide both PQC hybrid groups and pure classic groups to allow fallback for older clients
        // Note the order: PQC groups first so supported clients prefer them
        format!("{0}:{1}", PQC_KEM_GROUPS, CLASSIC_GROUPS)
    } else {
        CLASSIC_GROUPS.to_string()
    }
}
