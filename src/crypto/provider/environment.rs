//! Environment detection and diagnostics
//!
//! This module provides functionality for detecting and diagnosing
//! the cryptographic environment, including OpenSSL and OQS-OpenSSL.

use std::path::PathBuf;
use std::process::Command;

use super::{ProviderType, is_oqs_available};

/// Environment information
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    /// OpenSSL version
    pub openssl_version: String,
    /// Whether OQS-OpenSSL is available
    pub oqs_available: bool,
    /// Path to OQS-OpenSSL installation (if available)
    pub oqs_path: Option<PathBuf>,
    /// Supported provider types
    pub supported_providers: Vec<ProviderType>,
    /// Available post-quantum algorithms
    pub available_pqc_algorithms: Vec<String>,
}

/// Environment issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Informational message
    Info,
    /// Warning (non-critical issue)
    Warning,
    /// Error (critical issue)
    Error,
}

/// Environment issue
#[derive(Debug, Clone)]
pub struct EnvironmentIssue {
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue message
    pub message: String,
    /// Suggested resolution
    pub resolution: Option<String>,
}

/// Check the cryptographic environment
///
/// This function checks the cryptographic environment and returns
/// information about the available providers and algorithms.
///
/// # Returns
///
/// Environment information
pub fn check_environment() -> EnvironmentInfo {
    // Check if OQS is available
    let oqs_available = is_oqs_available();

    // Get OpenSSL version
    let openssl_version = get_openssl_version();

    // Determine supported providers
    let mut supported_providers = vec![ProviderType::Standard];
    if oqs_available {
        supported_providers.push(ProviderType::Oqs);
    }
    supported_providers.push(ProviderType::Auto);

    // Get available PQC algorithms
    let available_pqc_algorithms = if oqs_available {
        get_available_pqc_algorithms()
    } else {
        Vec::new()
    };

    // Get OQS path
    let oqs_path = if oqs_available {
        // Access OQS_PATH through a public function
        super::factory::get_oqs_path()
    } else {
        None
    };

    EnvironmentInfo {
        openssl_version,
        oqs_available,
        oqs_path,
        supported_providers,
        available_pqc_algorithms,
    }
}

/// Diagnose the cryptographic environment
///
/// This function diagnoses the cryptographic environment and returns
/// a list of issues that may affect functionality.
///
/// # Returns
///
/// A list of environment issues
pub fn diagnose_environment() -> Vec<EnvironmentIssue> {
    let mut issues = Vec::new();

    // Check OpenSSL
    let openssl_version = get_openssl_version();
    if openssl_version.is_empty() {
        issues.push(EnvironmentIssue {
            severity: IssueSeverity::Error,
            message: "OpenSSL not found".to_string(),
            resolution: Some("Install OpenSSL development libraries".to_string()),
        });
    } else {
        issues.push(EnvironmentIssue {
            severity: IssueSeverity::Info,
            message: format!("OpenSSL version: {}", openssl_version),
            resolution: None,
        });
    }

    // Check OQS-OpenSSL
    if is_oqs_available() {
        issues.push(EnvironmentIssue {
            severity: IssueSeverity::Info,
            message: "OQS-OpenSSL is available".to_string(),
            resolution: None,
        });

        // Check available PQC algorithms
        let algorithms = get_available_pqc_algorithms();
        if algorithms.is_empty() {
            issues.push(EnvironmentIssue {
                severity: IssueSeverity::Warning,
                message: "No post-quantum algorithms detected".to_string(),
                resolution: Some("Check OQS-OpenSSL installation".to_string()),
            });
        } else {
            issues.push(EnvironmentIssue {
                severity: IssueSeverity::Info,
                message: format!("Available PQC algorithms: {}", algorithms.join(", ")),
                resolution: None,
            });
        }
    } else {
        issues.push(EnvironmentIssue {
            severity: IssueSeverity::Warning,
            message: "OQS-OpenSSL not available".to_string(),
            resolution: Some("Install OQS-OpenSSL for post-quantum support".to_string()),
        });
    }

    // Check environment variables
    if std::env::var("OQS_OPENSSL_PATH").is_err() {
        issues.push(EnvironmentIssue {
            severity: IssueSeverity::Info,
            message: "OQS_OPENSSL_PATH environment variable not set".to_string(),
            resolution: Some("Set OQS_OPENSSL_PATH to the OQS-OpenSSL installation directory".to_string()),
        });
    }

    issues
}

/// Get OpenSSL version
///
/// # Returns
///
/// OpenSSL version string
fn get_openssl_version() -> String {
    // Try to get OpenSSL version using the openssl command
    let output = Command::new("openssl")
        .arg("version")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        },
        _ => {
            // Try to get version from OpenSSL library
            openssl::version::version().to_string()
        }
    }
}

/// Get available post-quantum algorithms
///
/// # Returns
///
/// A list of available post-quantum algorithms
fn get_available_pqc_algorithms() -> Vec<String> {
    let mut algorithms = Vec::new();

    // This is a simplified implementation
    // A real implementation would query OQS-OpenSSL for available algorithms

    // Check for common PQC algorithms
    let common_algorithms = [
        "Kyber512", "Kyber768", "Kyber1024",
        "Dilithium2", "Dilithium3", "Dilithium5",
        "Falcon512", "Falcon1024",
        "SPHINCS+-SHA256-128s", "SPHINCS+-SHA256-192s", "SPHINCS+-SHA256-256s",
    ];

    // For now, just return common algorithms if OQS is available
    if is_oqs_available() {
        algorithms.extend(common_algorithms.iter().map(|s| s.to_string()));
    }

    algorithms
}
