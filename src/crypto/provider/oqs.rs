//! OQS-OpenSSL provider implementation
//!
//! This module implements the CryptoProvider trait using OQS-OpenSSL,
//! which supports post-quantum cryptography algorithms.

use std::path::Path;
use openssl::x509::X509;
use openssl::hash::MessageDigest;
use log::debug;

use crate::common::{ProxyError, Result, read_file};
use super::{CryptoProvider, CryptoCapabilities, CertificateType, DetectedCapabilities};

/// OQS-OpenSSL provider with post-quantum support
///
/// This provider uses the OQS-OpenSSL library with post-quantum support.
/// It can handle hybrid and pure post-quantum certificates.
#[derive(Debug, Default, Clone)]
pub struct OqsProvider;

impl OqsProvider {
    /// Create a new OQS-OpenSSL provider
    pub fn new() -> Self {
        Self
    }

    /// Extract post-quantum algorithms from certificate
    fn extract_pqc_algorithms(&self, cert: &X509) -> Vec<String> {
        let mut algorithms = Vec::new();

        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object().to_string();

        // Check for known PQC algorithms
        if signature_algorithm.contains("Kyber") {
            algorithms.push("Kyber".to_string());
        }

        if signature_algorithm.contains("Dilithium") {
            algorithms.push("Dilithium".to_string());
        }

        if signature_algorithm.contains("Falcon") {
            algorithms.push("Falcon".to_string());
        }

        if signature_algorithm.contains("SPHINCS") {
            algorithms.push("SPHINCS+".to_string());
        }

        // Try to extract more detailed information from extensions
        // This is a simplified approach; a real implementation would parse X.509 extensions

        algorithms
    }

    /// Determine certificate type
    fn determine_certificate_type(&self, cert: &X509) -> CertificateType {
        // Get signature algorithm
        let signature_algorithm = cert.signature_algorithm().object().to_string();
        debug!("Raw signature algorithm string: {}", signature_algorithm);

        // For p384_dilithium3 style hybrid certificates
        if signature_algorithm.contains("p256_") ||
           signature_algorithm.contains("p384_") ||
           signature_algorithm.contains("p521_") ||
           signature_algorithm.contains("_p256") ||
           signature_algorithm.contains("_p384") ||
           signature_algorithm.contains("_p521") {
            // This is definitely a hybrid certificate
            debug!("Detected hybrid certificate with signature algorithm: {}", signature_algorithm);
            return CertificateType::Hybrid;
        }

        // Check for known PQC algorithm indicators
        if signature_algorithm.contains("Kyber") ||
           signature_algorithm.contains("kyber") ||
           signature_algorithm.contains("Dilithium") ||
           signature_algorithm.contains("dilithium") ||
           signature_algorithm.contains("Falcon") ||
           signature_algorithm.contains("falcon") ||
           signature_algorithm.contains("SPHINCS") ||
           signature_algorithm.contains("sphincs") {
            // If it contains both classical and PQC algorithms, it's hybrid
            if signature_algorithm.contains("RSA") ||
               signature_algorithm.contains("rsa") ||
               signature_algorithm.contains("ECDSA") ||
               signature_algorithm.contains("ecdsa") ||
               signature_algorithm.contains("DSA") ||
               signature_algorithm.contains("dsa") ||
               signature_algorithm.contains("P-256") ||
               signature_algorithm.contains("P-384") ||
               signature_algorithm.contains("P-521") ||
               signature_algorithm.contains("prime256v1") ||
               signature_algorithm.contains("secp384r1") ||
               signature_algorithm.contains("secp521r1") {
                debug!("Detected hybrid certificate with classical and PQC algorithms: {}", signature_algorithm);
                CertificateType::Hybrid
            } else {
                debug!("Detected pure post-quantum certificate: {}", signature_algorithm);
                CertificateType::PurePostQuantum
            }
        } else if signature_algorithm.contains("oqs") ||
                  signature_algorithm.contains("OQS") ||
                  signature_algorithm.contains("hybrid") {
            // Generic indicators for hybrid certificates
            debug!("Detected hybrid certificate with generic indicators: {}", signature_algorithm);
            CertificateType::Hybrid
        } else {
            // No PQC indicators found
            debug!("Detected classical certificate: {}", signature_algorithm);
            CertificateType::Classical
        }
    }
}

impl CryptoProvider for OqsProvider {
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
        let cert_type = self.determine_certificate_type(&cert);

        Ok(cert_type == CertificateType::Hybrid)
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

        // Add PQC algorithm information if available
        let pqc_algorithms = self.extract_pqc_algorithms(&cert);
        if !pqc_algorithms.is_empty() {
            let pqc_info = format!(" (PQC: {})", pqc_algorithms.join(", "));
            return Ok(format!("{}{}", subject_str, pqc_info));
        }

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
        // OQS-OpenSSL 提供者的能力
        // 支援後量子密碼學，包括 Kyber 密鑰交換和各種後量子簽名算法

        // 定義常量以提高可維護性
        const OQS_CIPHER_LIST: &str = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";
        const OQS_TLS13_CIPHERSUITES: &str = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
        const OQS_GROUPS: &str = "X25519:P-256:P-384:P-521:kyber768:p384_kyber768:kyber512:p256_kyber512:kyber1024:p521_kyber1024";

        // 嘗試檢測 OQS-OpenSSL 能力
        let detected_capabilities = self.detect_oqs_capabilities();

        // 根據檢測結果构建能力信息
        CryptoCapabilities {
            supports_pqc: true,
            supported_key_exchange: self.detect_supported_key_exchange(),
            supported_signatures: self.detect_supported_signatures(),
            // 使用檢測到的能力或預設值
            recommended_cipher_list: detected_capabilities.cipher_list.unwrap_or_else(|| OQS_CIPHER_LIST.to_string()),
            recommended_tls13_ciphersuites: detected_capabilities.tls13_ciphersuites.unwrap_or_else(|| OQS_TLS13_CIPHERSUITES.to_string()),
            recommended_groups: detected_capabilities.groups.unwrap_or_else(|| OQS_GROUPS.to_string()),
        }
    }

    fn name(&self) -> &'static str {
        "OQS-OpenSSL (Post-Quantum)"
    }
}

// Private implementation methods for OqsProvider
impl OqsProvider {
    /// 檢測 OQS-OpenSSL 能力
    fn detect_oqs_capabilities(&self) -> DetectedCapabilities {
        use std::process::Command;
        use once_cell::sync::OnceCell;
        use log::{debug, warn, info};
        use super::factory::get_oqs_path;

        // 使用 OnceCell 緩存檢測結果，避免重複檢測
        static DETECTED_CAPABILITIES: OnceCell<DetectedCapabilities> = OnceCell::new();

        // 如果已經檢測過，直接返回緩存的結果
        if let Some(capabilities) = DETECTED_CAPABILITIES.get() {
            return capabilities.clone();
        }

        // 初始化結果
        let mut capabilities = DetectedCapabilities::default();

        // 從 factory 模組獲取 OQS-OpenSSL 路徑
        let oqs_path = get_oqs_path();

        // 如果找不到 OQS-OpenSSL，使用預設值
        if oqs_path.is_none() {
            warn!("OQS-OpenSSL path not found, using default TLS settings");
            let default_cipher_list = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";
            let default_tls13 = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
            let default_groups = "X25519:P-256:P-384:P-521:kyber768:p384_kyber768";

            capabilities.cipher_list = Some(default_cipher_list.to_string());
            capabilities.tls13_ciphersuites = Some(default_tls13.to_string());
            capabilities.groups = Some(default_groups.to_string());

            // 緩存檢測結果
            let _ = DETECTED_CAPABILITIES.set(capabilities.clone());
            return capabilities;
        }

        let oqs_path = oqs_path.unwrap();
        info!("Using OQS-OpenSSL at: {}", oqs_path.display());

        // 檢測 OQS-OpenSSL 支持的密碼套件
        let openssl_bin = oqs_path.join("bin").join("openssl");

        // 檢測支持的密碼套件
        match Command::new(&openssl_bin).args(["ciphers", "-v"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let ciphers = stdout.lines()
                    .filter_map(|line| line.split_whitespace().next())
                    .filter(|cipher| !cipher.is_empty())
                    .collect::<Vec<&str>>();

                if !ciphers.is_empty() {
                    let cipher_list = "HIGH:MEDIUM:!aNULL:!MD5:!RC4".to_string();
                    capabilities.cipher_list = Some(cipher_list);
                    debug!("Detected {} cipher suites in OQS-OpenSSL", ciphers.len());
                }
            },
            _ => warn!("Failed to detect OQS-OpenSSL cipher suites, using defaults")
        }

        // 檢測支持的 TLS 1.3 密碼套件
        match Command::new(&openssl_bin).args(["ciphers", "-tls1_3"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let ciphersuites = stdout.trim();

                if !ciphersuites.is_empty() {
                    capabilities.tls13_ciphersuites = Some(ciphersuites.to_string());
                    debug!("Detected TLS 1.3 ciphersuites in OQS-OpenSSL: {}", ciphersuites);
                }
            },
            _ => {
                // 如果無法直接檢測 TLS 1.3 密碼套件，使用預設值
                let default_tls13 = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
                capabilities.tls13_ciphersuites = Some(default_tls13.to_string());
                debug!("Using default TLS 1.3 ciphersuites: {}", default_tls13);
            }
        }

        // 檢測支持的群組，包括後量子群組
        // 先檢測傳統的橘圈曲線
        let mut detected_groups = Vec::new();

        match Command::new(&openssl_bin).args(["ecparam", "-list_curves"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let curves = stdout.lines()
                    .filter_map(|line| line.split(':').next())
                    .map(|curve| curve.trim())
                    .filter(|curve| !curve.is_empty())
                    .collect::<Vec<&str>>();

                if !curves.is_empty() {
                    // 從檢測到的曲線中選擇常用的曲線
                    for curve in ["X25519", "P-256", "P-384", "P-521"].iter() {
                        if curves.iter().any(|c| c.contains(curve)) {
                            detected_groups.push((*curve).to_string());
                        }
                    }

                    debug!("Detected elliptic curves in OQS-OpenSSL: {}", detected_groups.join(":"));
                }
            },
            _ => warn!("Failed to detect OQS-OpenSSL curves, using defaults")
        }

        // 檢測後量子群組
        // 我們使用 openssl list -kem-algorithms 來檢測支持的後量子密鑰交換算法
        match Command::new(&openssl_bin).args(["list", "-kem-algorithms"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // 檢測常見的後量子群組
                for kem in ["kyber768", "p384_kyber768", "kyber512", "p256_kyber512", "kyber1024", "p521_kyber1024"].iter() {
                    if stdout.contains(kem) {
                        detected_groups.push((*kem).to_string());
                        debug!("Detected post-quantum KEM: {}", kem);
                    }
                }
            },
            _ => {
                // 如果無法直接檢測，則假設支持常見的後量子群組
                for kem in ["kyber768", "p384_kyber768"].iter() {
                    detected_groups.push((*kem).to_string());
                }
                debug!("Assuming support for common post-quantum KEMs");
            }
        }

        // 如果檢測到了群組，則使用檢測到的群組
        if !detected_groups.is_empty() {
            let groups = detected_groups.join(":");
            capabilities.groups = Some(groups.clone());
            info!("Using detected groups: {}", groups);
        } else {
            // 如果沒有檢測到群組，使用預設值
            let default_groups = "X25519:P-256:P-384:P-521:kyber768:p384_kyber768";
            capabilities.groups = Some(default_groups.to_string());
            warn!("No groups detected, using default groups: {}", default_groups);
        }

        // 如果沒有檢測到密碼套件，使用預設值
        if capabilities.cipher_list.is_none() {
            let default_cipher_list = "HIGH:MEDIUM:!aNULL:!MD5:!RC4";
            capabilities.cipher_list = Some(default_cipher_list.to_string());
            debug!("Using default cipher list: {}", default_cipher_list);
        }

        // 如果沒有檢測到 TLS 1.3 密碼套件，使用預設值
        if capabilities.tls13_ciphersuites.is_none() {
            let default_tls13 = "TLS_AES_256_GCM_SHA384:TLS_AES_128_GCM_SHA256";
            capabilities.tls13_ciphersuites = Some(default_tls13.to_string());
            debug!("Using default TLS 1.3 ciphersuites: {}", default_tls13);
        }

        // 緩存檢測結果
        let _ = DETECTED_CAPABILITIES.set(capabilities.clone());

        capabilities
    }

    /// 檢測支援的密鑰交換算法
    fn detect_supported_key_exchange(&self) -> Vec<String> {
        use std::process::Command;
        use super::factory::get_oqs_path;
        use log::debug;

        // 初始化基本的密鑰交換算法
        let mut key_exchange = vec![
            "RSA".to_string(),
            "ECDHE".to_string(),
            "DHE".to_string(),
        ];

        // 從 factory 模組獲取 OQS-OpenSSL 路徑
        let oqs_path = get_oqs_path();

        // 如果找不到 OQS-OpenSSL，使用預設值
        if oqs_path.is_none() {
            key_exchange.push("Kyber".to_string());
            return key_exchange;
        }

        let oqs_path = oqs_path.unwrap();
        let openssl_bin = oqs_path.join("bin").join("openssl");

        // 檢測支持的後量子密鑰交換算法
        match Command::new(&openssl_bin).args(["list", "-kem-algorithms"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // 檢測常見的後量子密鑰交換算法
                for kem in ["Kyber", "NTRU", "SIKE"].iter() {
                    if stdout.contains(kem) {
                        key_exchange.push((*kem).to_string());
                        debug!("Detected post-quantum key exchange: {}", kem);
                    }
                }
            },
            _ => {
                // 如果無法直接檢測，則假設支持 Kyber
                key_exchange.push("Kyber".to_string());
                debug!("Assuming support for Kyber key exchange");
            }
        }

        key_exchange
    }

    /// 檢測支援的簽名算法
    fn detect_supported_signatures(&self) -> Vec<String> {
        use std::process::Command;
        use super::factory::get_oqs_path;
        use log::debug;

        // 初始化基本的簽名算法
        let mut signatures = vec![
            "RSA".to_string(),
            "ECDSA".to_string(),
            "DSA".to_string(),
        ];

        // 從 factory 模組獲取 OQS-OpenSSL 路徑
        let oqs_path = get_oqs_path();

        // 如果找不到 OQS-OpenSSL，使用預設值
        if oqs_path.is_none() {
            signatures.extend(vec![
                "Dilithium".to_string(),
                "Falcon".to_string(),
                "SPHINCS+".to_string(),
            ]);
            return signatures;
        }

        let oqs_path = oqs_path.unwrap();
        let openssl_bin = oqs_path.join("bin").join("openssl");

        // 檢測支持的後量子簽名算法
        match Command::new(&openssl_bin).args(["list", "-signature-algorithms"]).output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // 檢測常見的後量子簽名算法
                for sig in ["Dilithium", "Falcon", "SPHINCS", "Rainbow"].iter() {
                    if stdout.contains(sig) {
                        signatures.push((*sig).to_string());
                        debug!("Detected post-quantum signature: {}", sig);
                    }
                }
            },
            _ => {
                // 如果無法直接檢測，則假設支持常見的後量子簽名算法
                signatures.extend(vec![
                    "Dilithium".to_string(),
                    "Falcon".to_string(),
                    "SPHINCS+".to_string(),
                ]);
                debug!("Assuming support for common post-quantum signatures");
            }
        }

        signatures
    }
}
