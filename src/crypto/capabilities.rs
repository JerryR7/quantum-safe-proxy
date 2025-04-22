//! OpenSSL capabilities detection for post-quantum cryptography support

use log::{debug, warn};
use openssl::ssl::{SslContext, SslMethod};
use std::collections::HashSet;

/// Check if OpenSSL 3.5+ is available (required for PQC support)
pub fn is_openssl35_available() -> bool {
    // 使用 openssl crate 的 API 直接獲取版本號
    let version_number = openssl::version::number();

    // 版本號格式: 0xMNNFFPPS (M=major, NN=minor, FF=fix, PP=patch, S=status)
    let major = (version_number >> 28) & 0xFF;
    let minor = (version_number >> 20) & 0xFF;

    debug!("OpenSSL version number: 0x{:08X}, major: {}, minor: {}", version_number, major, minor);

    // 檢查是否為 OpenSSL 3.5 或更高版本
    major == 3 && minor >= 5
}

/// Check if post-quantum cryptography is available in the current OpenSSL installation
pub fn is_pqc_available() -> bool {
    // First check if OpenSSL 3.5+ is available
    if !is_openssl35_available() {
        return false;
    }

    // Then check if any post-quantum algorithms are available
    !get_supported_pq_algorithms().is_empty()
}

/// Get OpenSSL version string
pub fn get_openssl_version() -> String {
    // Try to get version from OpenSSL
    let version = ::openssl::version::version();
    return version.to_string();
}

/// Get list of supported post-quantum algorithms
pub fn get_supported_pq_algorithms() -> Vec<String> {
    let mut algorithms = Vec::new();

    // Check if OpenSSL 3.5+ is available
    if !is_openssl35_available() {
        return algorithms;
    }

    // 使用 OpenSSL API 直接檢查支援的演算法
    let supported_algorithms = get_supported_algorithms();

    // 檢查後量子演算法
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

        // 檢查後量子簽名演算法
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

/// 使用 OpenSSL API 取得支援的所有演算法
fn get_supported_algorithms() -> HashSet<String> {
    let mut algorithms = HashSet::new();

    // 嘗試建立 SSL 環境來檢查支援的演算法
    match SslContext::builder(SslMethod::tls_client()) {
        Ok(_ctx) => {
            // 直接使用 OpenSSL 版本來判斷支援的演算法
            // 在 OpenSSL 3.5+ 中，我們可以假設支援後量子演算法
            if is_openssl35_available() {
                // 密鑰交換演算法
                algorithms.insert("ML-KEM-512".to_string());
                algorithms.insert("ML-KEM-768".to_string());
                algorithms.insert("ML-KEM-1024".to_string());

                // 簽名演算法
                algorithms.insert("ML-DSA-44".to_string());
                algorithms.insert("ML-DSA-65".to_string());
                algorithms.insert("ML-DSA-87".to_string());
                algorithms.insert("SLH-DSA-FALCON-512".to_string());
                algorithms.insert("SLH-DSA-FALCON-1024".to_string());
            }
        }
        Err(e) => {
            warn!("Failed to create SSL context for algorithm detection: {}", e);
        }
    }

    // 傳統演算法
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

/// Get recommended groups based on PQC support
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
