//! Environment checking module
//!
//! This module provides functionality to check the environment
//! for OpenSSL and post-quantum cryptography support.

use std::process::Command;
use std::collections::HashMap;

/// Environment issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Informational message
    Info,
    
    /// Warning message
    Warning,
    
    /// Error message
    Error,
}

/// Environment issue
#[derive(Debug, Clone)]
pub struct EnvironmentIssue {
    /// Issue message
    pub message: String,
    
    /// Issue severity
    pub severity: IssueSeverity,
}

/// Environment information
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    /// OpenSSL version
    pub openssl_version: String,
    
    /// Whether post-quantum cryptography is available
    pub pqc_available: bool,
    
    /// Supported key exchange algorithms
    pub key_exchange_algorithms: Vec<String>,
    
    /// Supported signature algorithms
    pub signature_algorithms: Vec<String>,
    
    /// Environment issues
    pub issues: Vec<EnvironmentIssue>,
    
    /// Environment variables
    pub env_vars: HashMap<String, String>,
}

/// Check the environment for OpenSSL and post-quantum cryptography support
///
/// # Returns
///
/// Environment information
pub fn check_environment() -> EnvironmentInfo {
    let mut info = EnvironmentInfo {
        openssl_version: "unknown".to_string(),
        pqc_available: false,
        key_exchange_algorithms: Vec::new(),
        signature_algorithms: Vec::new(),
        issues: Vec::new(),
        env_vars: HashMap::new(),
    };
    
    // Check OpenSSL version
    match Command::new("openssl").arg("version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).to_string();
            info.openssl_version = version.trim().to_string();
            
            // Check if OpenSSL 3.5+
            if version.contains("3.5") {
                info.issues.push(EnvironmentIssue {
                    message: "OpenSSL 3.5+ detected with built-in post-quantum support".to_string(),
                    severity: IssueSeverity::Info,
                });
            } else {
                info.issues.push(EnvironmentIssue {
                    message: format!("OpenSSL version {} does not have built-in post-quantum support", version.trim()),
                    severity: IssueSeverity::Warning,
                });
            }
        },
        _ => {
            info.issues.push(EnvironmentIssue {
                message: "Failed to detect OpenSSL version".to_string(),
                severity: IssueSeverity::Error,
            });
        }
    }
    
    // Check for PQC support
    match Command::new("openssl").args(["list", "-kem-algorithms"]).output() {
        Ok(output) if output.status.success() => {
            let kem_list = String::from_utf8_lossy(&output.stdout);
            if kem_list.contains("ML-KEM") {
                info.pqc_available = true;
                info.issues.push(EnvironmentIssue {
                    message: "Post-quantum key exchange algorithms (ML-KEM) are available".to_string(),
                    severity: IssueSeverity::Info,
                });
                
                // Extract ML-KEM algorithms
                for line in kem_list.lines() {
                    if line.contains("ML-KEM") {
                        let parts: Vec<&str> = line.split('@').collect();
                        if let Some(alg_part) = parts.first() {
                            let alg = alg_part.trim();
                            if alg.contains("ML-KEM") {
                                info.key_exchange_algorithms.push(alg.to_string());
                            }
                        }
                    }
                }
            } else {
                info.issues.push(EnvironmentIssue {
                    message: "Post-quantum key exchange algorithms (ML-KEM) are not available".to_string(),
                    severity: IssueSeverity::Warning,
                });
            }
        },
        _ => {
            info.issues.push(EnvironmentIssue {
                message: "Failed to detect key exchange algorithms".to_string(),
                severity: IssueSeverity::Warning,
            });
        }
    }
    
    // Check for signature algorithms
    match Command::new("openssl").args(["list", "-signature-algorithms"]).output() {
        Ok(output) if output.status.success() => {
            let sig_list = String::from_utf8_lossy(&output.stdout);
            if sig_list.contains("ML-DSA") || sig_list.contains("SLH-DSA") {
                info.issues.push(EnvironmentIssue {
                    message: "Post-quantum signature algorithms (ML-DSA/SLH-DSA) are available".to_string(),
                    severity: IssueSeverity::Info,
                });
                
                // Extract PQ signature algorithms
                for line in sig_list.lines() {
                    if line.contains("ML-DSA") || line.contains("SLH-DSA") {
                        let parts: Vec<&str> = line.split('@').collect();
                        if let Some(alg_part) = parts.first() {
                            let alg = alg_part.trim();
                            if alg.contains("ML-DSA") || alg.contains("SLH-DSA") {
                                info.signature_algorithms.push(alg.to_string());
                            }
                        }
                    }
                }
            } else {
                info.issues.push(EnvironmentIssue {
                    message: "Post-quantum signature algorithms (ML-DSA/SLH-DSA) are not available".to_string(),
                    severity: IssueSeverity::Warning,
                });
            }
        },
        _ => {
            info.issues.push(EnvironmentIssue {
                message: "Failed to detect signature algorithms".to_string(),
                severity: IssueSeverity::Warning,
            });
        }
    }
    
    // Check environment variables
    for var in &["OPENSSL_DIR", "OPENSSL_LIB_DIR", "OPENSSL_INCLUDE_DIR", "LD_LIBRARY_PATH"] {
        if let Ok(value) = std::env::var(var) {
            info.env_vars.insert(var.to_string(), value);
        }
    }
    
    info
}

/// Diagnose environment issues
///
/// # Arguments
///
/// * `info` - Environment information
///
/// # Returns
///
/// A list of environment issues
pub fn diagnose_environment(info: &EnvironmentInfo) -> Vec<EnvironmentIssue> {
    let mut issues = info.issues.clone();
    
    // Check if PQC is available
    if !info.pqc_available {
        issues.push(EnvironmentIssue {
            message: "Post-quantum cryptography is not available. Consider upgrading to OpenSSL 3.5+.".to_string(),
            severity: IssueSeverity::Warning,
        });
    }
    
    // Check OpenSSL version
    if info.openssl_version.contains("unknown") {
        issues.push(EnvironmentIssue {
            message: "OpenSSL version could not be detected".to_string(),
            severity: IssueSeverity::Error,
        });
    } else if !info.openssl_version.contains("3.5") {
        issues.push(EnvironmentIssue {
            message: format!("OpenSSL version {} does not have built-in post-quantum support. Consider upgrading to OpenSSL 3.5+.", info.openssl_version),
            severity: IssueSeverity::Warning,
        });
    }
    
    issues
}
