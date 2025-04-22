//! Environment checks and diagnostics for OpenSSL and PQC support


use std::env;

use super::capabilities::is_openssl35_available;

/// Environment issue severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Informational issue
    Info,

    /// Warning issue
    Warning,

    /// Error issue
    Error,
}

/// Environment issue with message, severity and resolution
#[derive(Debug, Clone)]
pub struct EnvironmentIssue {
    /// Issue message
    pub message: String,

    /// Issue severity
    pub severity: IssueSeverity,

    /// Suggested resolution
    pub resolution: Option<String>,
}

/// Environment information about OpenSSL and PQC capabilities
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    /// OpenSSL version
    pub openssl_version: String,

    /// Whether OpenSSL 3.5+ is available
    pub openssl35_available: bool,

    /// Whether post-quantum cryptography is available
    pub pqc_available: bool,

    /// Supported post-quantum algorithms
    pub supported_pq_algorithms: Vec<String>,

    /// Environment variables
    pub environment_variables: Vec<(String, String)>,

    /// Environment issues
    pub issues: Vec<EnvironmentIssue>,
}

/// Check OpenSSL and PQC environment using the global provider instance
pub fn check_environment() -> EnvironmentInfo {
    // Get the global provider to ensure it's initialized only once
    let provider = super::get_provider();
    let capabilities = provider.capabilities();

    // Get OpenSSL version
    let openssl_version = capabilities.openssl_version.clone();

    // Check if OpenSSL 3.5+ is available
    let openssl35_available = is_openssl35_available();

    // Check if post-quantum cryptography is available
    let pqc_available = capabilities.supports_pqc;

    // Get supported post-quantum algorithms
    let supported_pq_algorithms = capabilities.supported_pq_algorithms.clone();

    // Get relevant environment variables
    let environment_variables = get_environment_variables();

    // Detect issues
    let issues = detect_issues(&openssl_version, openssl35_available, pqc_available);

    EnvironmentInfo {
        openssl_version,
        openssl35_available,
        pqc_available,
        supported_pq_algorithms,
        environment_variables,
        issues,
    }
}

/// Diagnose environment issues and return suggested resolutions
pub fn diagnose_environment() -> Vec<EnvironmentIssue> {
    // Check environment
    let env_info = check_environment();

    // Return issues
    env_info.issues
}

/// Get OpenSSL-related environment variables
fn get_environment_variables() -> Vec<(String, String)> {
    let mut variables = Vec::new();

    // Check OpenSSL environment variables
    for var in &["OPENSSL_DIR", "OPENSSL_LIB_DIR", "OPENSSL_INCLUDE_DIR", "OPENSSL_ENGINES_DIR"] {
        if let Ok(value) = env::var(var) {
            variables.push((var.to_string(), value));
        }
    }

    variables
}

/// Detect OpenSSL and PQC environment issues
fn detect_issues(openssl_version: &str, openssl35_available: bool, pqc_available: bool) -> Vec<EnvironmentIssue> {
    let mut issues = Vec::new();

    // Check OpenSSL version
    if !openssl35_available {
        issues.push(EnvironmentIssue {
            message: format!("OpenSSL 3.5+ is required for post-quantum cryptography, but found {}", openssl_version),
            severity: IssueSeverity::Error,
            resolution: Some("Upgrade to OpenSSL 3.5 or later".to_string()),
        });
    }

    // Check post-quantum cryptography support
    if openssl35_available && !pqc_available {
        issues.push(EnvironmentIssue {
            message: "OpenSSL 3.5+ is available, but post-quantum cryptography is not enabled".to_string(),
            severity: IssueSeverity::Warning,
            resolution: Some("Ensure OpenSSL is compiled with post-quantum cryptography support".to_string()),
        });
    }

    issues
}
