//! Standard OpenSSL provider implementation
//!
//! This module implements the CryptoProvider trait using standard OpenSSL.
//! It does not support post-quantum cryptography but provides a fallback
//! when OQS-OpenSSL is not available.

use std::path::Path;
use openssl::x509::X509;
use openssl::hash::MessageDigest;
use log::debug;

use crate::common::{ProxyError, Result, read_file};
use super::{CryptoProvider, CryptoCapabilities, DetectedCapabilities};

/// Standard OpenSSL provider
///
/// This provider uses the standard OpenSSL library without post-quantum support.
/// It serves as a fallback when OQS-OpenSSL is not available.
#[derive(Debug, Default, Clone)]
pub struct StandardProvider;

impl StandardProvider {
    /// Create a new standard OpenSSL provider
    pub fn new() -> Self {
        Self
    }
}

impl CryptoProvider for StandardProvider {
    fn is_hybrid_cert(&self, cert_path: &Path) -> Result<bool> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object();

        // Get signature algorithm name
        let algorithm_name = signature_algorithm.to_string();
        debug!("Certificate signature algorithm: {}", algorithm_name);

        // Check if it's a hybrid certificate
        // Note: This detection logic should be adjusted based on the actual PQC algorithms in use
        // Currently we're simply checking if the algorithm name contains specific strings
        let is_hybrid = algorithm_name.contains("Kyber") ||
                       algorithm_name.contains("Dilithium") ||
                       algorithm_name.contains("oqs") ||
                       algorithm_name.contains("hybrid");

        Ok(is_hybrid)
    }

    fn get_cert_subject(&self, cert_path: &Path) -> Result<String> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        // Get subject
        let subject = cert.subject_name();
        // Convert X509NameRef to string
        let subject_str = format!("{:?}", subject);

        Ok(subject_str)
    }

    fn get_cert_fingerprint(&self, cert_path: &Path) -> Result<String> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        // Calculate SHA-256 fingerprint
        let fingerprint = cert.digest(MessageDigest::sha256())
            .map_err(|e| ProxyError::Certificate(format!("Failed to calculate certificate fingerprint: {}", e)))?;

        // Convert to hexadecimal string
        let fingerprint_hex = fingerprint.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join(":");

        Ok(fingerprint_hex)
    }

    fn load_cert(&self, cert_path: &Path) -> Result<X509> {
        // Read certificate file
        let cert_data = read_file(cert_path)
            .map_err(|e| ProxyError::Certificate(format!("Failed to read certificate file: {}", e)))?;

        // Parse certificate
        let cert = X509::from_pem(&cert_data)
            .map_err(|e| ProxyError::Certificate(format!("Failed to parse certificate: {}", e)))?;

        Ok(cert)
    }

    fn capabilities(&self) -> CryptoCapabilities {
        // 標準 OpenSSL 提供者的能力
        // 不支援後量子密碼學，只提供傳統的密碼套件和群組

        // 定義常量以提高可維護性
        const STANDARD_CIPHER_LIST: &str = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";
        const STANDARD_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
        const STANDARD_GROUPS: &str = "X25519:P-256:P-384:P-521";

        // 嘗試檢測系統能力（簡化版）
        let detected_capabilities = self.detect_openssl_capabilities();

        CryptoCapabilities {
            supports_pqc: false,
            supported_key_exchange: vec![
                "RSA".to_string(),
                "ECDHE".to_string(),
                "DHE".to_string(),
            ],
            supported_signatures: vec![
                "RSA".to_string(),
                "ECDSA".to_string(),
                "DSA".to_string(),
            ],
            // 使用檢測到的能力或預設值
            recommended_cipher_list: detected_capabilities.cipher_list.unwrap_or_else(|| STANDARD_CIPHER_LIST.to_string()),
            recommended_tls13_ciphersuites: detected_capabilities.tls13_ciphersuites.unwrap_or_else(|| STANDARD_TLS13_CIPHERSUITES.to_string()),
            recommended_groups: detected_capabilities.groups.unwrap_or_else(|| STANDARD_GROUPS.to_string()),
        }
    }

    fn name(&self) -> &'static str {
        "Standard OpenSSL"
    }
}

// Private implementation methods for StandardProvider
impl StandardProvider {
    /// 檢測 OpenSSL 能力
    ///
    /// 使用 OpenSSL 命令行工具檢測系統支持的 TLS 設置
    fn detect_openssl_capabilities(&self) -> DetectedCapabilities {
        use std::process::Command;
        use once_cell::sync::OnceCell;
        use log::{debug, warn};

        // 使用 OnceCell 緩存檢測結果，避免重複檢測
        static DETECTED_CAPABILITIES: OnceCell<DetectedCapabilities> = OnceCell::new();

        // 如果已經檢測過，直接返回緩存的結果
        if let Some(capabilities) = DETECTED_CAPABILITIES.get() {
            return capabilities.clone();
        }

        // 初始化結果
        let mut capabilities = DetectedCapabilities::default();

        // 檢測支持的密碼套件
        match Command::new("openssl").args(["ciphers", "-v"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let ciphers = stdout.lines()
                    .filter_map(|line| line.split_whitespace().next())
                    .filter(|cipher| !cipher.is_empty())
                    .collect::<Vec<&str>>();

                if !ciphers.is_empty() {
                    let cipher_list = "HIGH:MEDIUM:!aNULL:!MD5:!RC4".to_string();
                    capabilities.cipher_list = Some(cipher_list);
                    debug!("Detected {} cipher suites", ciphers.len());
                }
            },
            _ => warn!("Failed to detect OpenSSL cipher suites, using defaults")
        }

        // 檢測支持的 TLS 1.3 密碼套件
        // 注意：此命令可能不適用於所有 OpenSSL 版本
        match Command::new("openssl").args(["ciphers", "-tls1_3"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let ciphersuites = stdout.trim();

                if !ciphersuites.is_empty() {
                    capabilities.tls13_ciphersuites = Some(ciphersuites.to_string());
                    debug!("Detected TLS 1.3 ciphersuites: {}", ciphersuites);
                }
            },
            _ => {
                // 如果無法直接檢測 TLS 1.3 密碼套件，使用預設值
                let default_tls13 = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
                capabilities.tls13_ciphersuites = Some(default_tls13.to_string());
                debug!("Using default TLS 1.3 ciphersuites: {}", default_tls13);
            }
        }

        // 檢測支持的群組
        // 注意：此命令可能不適用於所有 OpenSSL 版本
        match Command::new("openssl").args(["ecparam", "-list_curves"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let curves = stdout.lines()
                    .filter_map(|line| line.split(':').next())
                    .map(|curve| curve.trim())
                    .filter(|curve| !curve.is_empty())
                    .collect::<Vec<&str>>();

                if !curves.is_empty() {
                    // 從檢測到的曲線中選擇常用的曲線
                    let mut selected_curves = Vec::new();

                    // 優先選擇這些常用曲線
                    for curve in ["X25519", "P-256", "P-384", "P-521"].iter() {
                        if curves.iter().any(|c| c.contains(curve)) {
                            selected_curves.push(*curve);
                        }
                    }

                    if !selected_curves.is_empty() {
                        let groups = selected_curves.join(":");
                        capabilities.groups = Some(groups.clone());
                        debug!("Detected elliptic curves: {}", groups);
                    }
                }
            },
            _ => warn!("Failed to detect OpenSSL curves, using defaults")
        }

        // 如果沒有檢測到群組，使用預設值
        if capabilities.groups.is_none() {
            let default_groups = "X25519:P-256:P-384:P-521";
            capabilities.groups = Some(default_groups.to_string());
            debug!("Using default groups: {}", default_groups);
        }

        // 緩存檢測結果
        let _ = DETECTED_CAPABILITIES.set(capabilities.clone());

        capabilities
    }
}
