//! Shared types module
//!
//! This module contains shared data types and structures used throughout the application.

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Source address
    pub source: String,
    /// Target address
    pub target: String,
    /// Connection timestamp
    pub timestamp: std::time::SystemTime,
}

/// Certificate information
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    /// Certificate subject
    pub subject: String,
    /// Certificate fingerprint
    pub fingerprint: Option<String>,
    /// Whether the certificate is hybrid
    pub is_hybrid: bool,
}
