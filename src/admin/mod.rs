//! Admin API Module
//!
//! This module provides a web-based settings management interface for Quantum Safe Proxy.
//! It includes:
//! - HTTP API for configuration management
//! - Authentication and authorization (RBAC)
//! - Audit logging with tamper-evidence
//! - Embedded HTML UI
//!
//! # Architecture
//!
//! The admin API is embedded in the main proxy process and runs as a separate
//! tokio task. It uses axum for HTTP handling and tower for middleware.
//!
//! # Security
//!
//! - API key authentication (Bearer token)
//! - Role-based access control (Viewer/Operator/Admin)
//! - Audit logging of all configuration changes
//! - SHA256 hash chaining for tamper detection

pub mod types;
pub mod server;
pub mod handlers;
pub mod auth;
pub mod audit;
pub mod error;
pub mod html;
pub mod config_resolver;

// Re-exports for convenience
pub use types::{
    ResolvedConfig, ResolvedSetting, ConfigSource, SettingCategory,
    OperationalStatus, TlsModeStats, HandshakeStats, CryptoMode,
    ConfigurationChange, SettingChange, ValidationResult, ValidationError,
    SecurityWarning, WarningLevel, AuditEntry, AuditAction, Role, ApiKey,
};

pub use server::start_admin_server;
pub use error::{AdminError, AdminResult};
