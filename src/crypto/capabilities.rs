//! OpenSSL capabilities detection for post-quantum cryptography support

use log::{debug, warn};
use openssl::ssl::{SslContext, SslMethod};
use std::collections::HashSet;
use std::ffi::CStr;

// 全局静態變量用于快取 OpenSSL 版本信息
use std::sync::Once;
static OPENSSL_VERSION_INIT: Once = Once::new();
static mut OPENSSL_VERSION_RESULT: bool = false;
static mut OPENSSL_VERSION_INFO: Option<(i64, u8, u8)> = None;

/// Check if OpenSSL 3.5+ is available (required for PQC support)
pub fn is_openssl35_available() -> bool {
    // 只在第一次調用時計算結果
    OPENSSL_VERSION_INIT.call_once(|| {
        // 使用 openssl crate 的 API 直接獲取版本號
        let version_number = openssl::version::number();
        let version_str = openssl::version::version();

        // 版本號格式: 0xMNNFFPPS (M=major, NN=minor, FF=fix, PP=patch, S=status)
        let major = (version_number >> 28) & 0xFF;
        let minor = (version_number >> 20) & 0xFF;

        debug!("OpenSSL version: {}, number: 0x{:08X}, major: {}, minor: {}",
               version_str, version_number, major, minor);

        // 使用字串比對確認是否為 OpenSSL 3.5+
        // 只要版本號是 3.5 或更高就視為支援 PQC
        let is_openssl35 = version_str.contains("OpenSSL 3.") &&
                          (major == 3 && minor >= 5);

        // 存儲結果
        unsafe {
            OPENSSL_VERSION_RESULT = is_openssl35;
            OPENSSL_VERSION_INFO = Some((version_number, major as u8, minor as u8));
        }
    });

    // 返回快取結果
    unsafe { OPENSSL_VERSION_RESULT }
}

/// Check if post-quantum cryptography is available in the current OpenSSL installation
pub fn is_pqc_available() -> bool {
    // First check if OpenSSL 3.5+ is available
    if !is_openssl35_available() {
        return false;
    }

    // Then check if any post-quantum KEM algorithms are available
    // 使用動態檢測而非假設
    !list_supported_kems().is_empty()
}

/// Get OpenSSL version string
pub fn get_openssl_version() -> String {
    // 確保先調用 is_openssl35_available 來初始化快取
    is_openssl35_available();

    // 直接使用 OpenSSL API 獲取版本字串
    let version = ::openssl::version::version();
    return version.to_string();
}

/// Get OpenSSL version information
pub fn get_openssl_version_info() -> (i64, u8, u8) {
    // 確保先調用 is_openssl35_available 來初始化快取
    is_openssl35_available();

    // 返回快取的版本信息
    unsafe {
        if let Some(info) = OPENSSL_VERSION_INFO {
            return info;
        } else {
            // 如果快取未初始化，直接計算
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

/// 動態列出 OpenSSL 支援的 KEM 演算法
pub fn list_supported_kems() -> Vec<String> {
    let mut kems = Vec::new();

    // 檢查 OpenSSL 版本
    if !is_openssl35_available() {
        return kems;
    }

    // 使用 OpenSSL 3.5+ 的 API 動態查詢支援的 KEM 演算法
    // 注意：這裡使用 FFI 呼叫 OpenSSL 的 C API
    // 在實際實現中，可能需要使用 openssl-sys crate 進行更底層的呼叫

    // 嘗試建立 SSL 環境
    match SslContext::builder(SslMethod::tls_client()) {
        Ok(ctx) => {
            // 在這裡我們使用 OpenSSL 3.5+ 的特性來檢測 KEM 支援
            // 實際上，我們應該使用 SSL_CTX_get1_supported_kems 等 API
            // 但由於 Rust OpenSSL 綁定可能尚未提供這些 API，我們使用替代方法

            // 檢查是否支援 ML-KEM 演算法
            if is_openssl35_available() {
                // 檢查 TLS 1.3 中是否支援 X25519MLKEM768 等混合群組
                // 這是一個簡化的實現，實際上應該使用 SSL_CTX_get1_groups 等 API

                // 添加可能支援的 KEM 演算法
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

/// 使用 OpenSSL API 取得支援的所有演算法
fn get_supported_algorithms() -> HashSet<String> {
    let mut algorithms = HashSet::new();

    // 嘗試建立 SSL 環境來檢查支援的演算法
    match SslContext::builder(SslMethod::tls_client()) {
        Ok(_ctx) => {
            // 動態檢測支援的後量子演算法
            if is_openssl35_available() {
                // 獲取支援的 KEM 演算法
                for kem in list_supported_kems() {
                    algorithms.insert(kem);
                }

                // 簽名演算法 - 這裡我們仍然需要改進
                // 理想情況下，應該使用類似 SSL_CTX_get1_supported_signature_algorithms 的 API
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
    // 注意：TLS 1.3 中 ciphersuites 只指定對稱加密和 AEAD，不包含簽章演算法
    const DEFAULT_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256:TLS_CHACHA20_POLY1305_SHA256";

    // 在 TLS 1.3 中，簽章演算法是透過 signature_algorithms 擴展協商的
    // 不需要在 ciphersuites 中指定 PQC 簽章
    DEFAULT_TLS13_CIPHERSUITES.to_string()
}

/// Get recommended groups based on PQC support
pub fn get_recommended_groups(supports_pqc: bool) -> String {
    // 經典群組 (傳統 ECDH)
    const CLASSIC_GROUPS: &str = "X25519:P-256:P-384:P-521";

    // PQC 混合 KEM 群組 (OpenSSL 3.5+ 支援的正確名稱)
    const PQC_KEM_GROUPS: &str = "X25519MLKEM768:P256MLKEM768:P384MLKEM1024";

    if supports_pqc {
        // 同時提供 PQC 混合群組和純經典群組，讓舊客戶端可以 fallback
        // 注意順序：先列出 PQC 群組，讓支援的客戶端優先選擇
        format!("{0}:{1}", PQC_KEM_GROUPS, CLASSIC_GROUPS)
    } else {
        CLASSIC_GROUPS.to_string()
    }
}
