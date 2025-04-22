//! OpenSSL capabilities detection for post-quantum cryptography support


use std::process::Command;

/// Check if OpenSSL 3.5+ is available (required for PQC support)
pub fn is_openssl35_available() -> bool {
    let version = get_openssl_version();

    // Check if version starts with "3.5" or higher
    if let Some(v) = version.split_whitespace().next() {
        if v.starts_with("3.") {
            let minor = v.split('.').nth(1).unwrap_or("0").parse::<u32>().unwrap_or(0);
            return minor >= 5;
        }
    }

    false
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

    // Check for ML-KEM (Kyber) support
    if check_algorithm_support("ML-KEM") {
        algorithms.push("ML-KEM".to_string());
    }

    // Check for ML-DSA (Dilithium) support
    if check_algorithm_support("ML-DSA") {
        algorithms.push("ML-DSA".to_string());
    }

    // Check for SLH-DSA (Falcon) support
    if check_algorithm_support("SLH-DSA") {
        algorithms.push("SLH-DSA".to_string());
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
        if check_algorithm_support("ML-DSA") {
            algorithms.push("ML-DSA".to_string());
        }

        if check_algorithm_support("SLH-DSA") {
            algorithms.push("SLH-DSA".to_string());
        }
    }

    algorithms
}

/// Check if the specified algorithm is supported by the current OpenSSL installation
fn check_algorithm_support(algorithm: &str) -> bool {
    // Use OpenSSL command line to check algorithm support
    match Command::new("openssl").args(["list", "-public-key-algorithms"]).output() {
        Ok(output) => {
            if output.status.success() {
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    return output_str.contains(algorithm);
                }
            }
        }
        Err(_) => {}
    }

    false
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
