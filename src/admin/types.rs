//! Admin API Data Types
//!
//! This module defines all data structures used by the admin API,
//! including configuration representation, audit entries, and operational status.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Resolved configuration with source tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConfig {
    /// List of all resolved settings with metadata
    pub settings: Vec<ResolvedSetting>,

    /// Current operational status of the proxy
    pub status: OperationalStatus,

    /// Timestamp when config was resolved
    pub resolved_at: DateTime<Utc>,

    /// Configuration version (increments on changes)
    pub version: u64,
}

/// Individual configuration setting with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSetting {
    /// Setting name (e.g., "log_level", "listen", "target")
    pub name: String,

    /// Current value (JSON for flexibility)
    pub value: serde_json::Value,

    /// Where this value came from
    pub source: ConfigSource,

    /// Can this be changed without restart?
    pub hot_reloadable: bool,

    /// Category for UI grouping
    pub category: SettingCategory,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Is this a security-affecting setting?
    pub security_affecting: bool,
}

/// Configuration value source
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfigSource {
    /// Command-line argument (highest precedence)
    CommandLine,

    /// Environment variable
    Environment,

    /// UI-applied change (new in this feature)
    UI,

    /// Configuration file
    File,

    /// Built-in default (lowest precedence)
    Default,
}

/// Setting category for UI organization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettingCategory {
    /// Network settings (listen, target)
    Network,

    /// TLS/crypto settings (cert paths, cipher config)
    Security,

    /// Operational settings (timeouts, buffer sizes)
    Performance,

    /// Logging and telemetry
    Observability,

    /// Client authentication
    Authentication,
}

/// Operational status and runtime metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalStatus {
    /// Proxy uptime in seconds
    pub uptime_seconds: u64,

    /// Total connections handled since start
    pub total_connections: u64,

    /// Currently active connections
    pub active_connections: u64,

    /// TLS mode breakdown (Principle IV compliance)
    pub tls_mode_stats: TlsModeStats,

    /// Recent handshake metrics
    pub handshake_stats: HandshakeStats,
}

/// Cryptographic mode classification (Constitution Principle IV)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CryptoMode {
    /// Classical TLS (ECDHE, RSA, etc.)
    Classical,
    /// Hybrid TLS (classical + PQC, e.g., X25519MLKEM768)
    Hybrid,
    /// PQC-only TLS (if supported by crypto stack)
    Pqc,
}

/// TLS mode classification statistics (Constitution Principle IV)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsModeStats {
    /// Connections using classical TLS only
    pub classical_count: u64,

    /// Connections using hybrid TLS (classical + PQC)
    pub hybrid_count: u64,

    /// Connections using PQC-only TLS (if supported)
    pub pqc_count: u64,

    /// Last classification update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Recent TLS handshake statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeStats {
    /// Successful handshakes in last 5 minutes
    pub recent_success_count: u64,

    /// Failed handshakes in last 5 minutes
    pub recent_failure_count: u64,

    /// Average handshake duration (milliseconds)
    pub avg_duration_ms: f64,

    /// Handshake success rate (0.0-1.0)
    pub success_rate: f64,
}

/// Configuration modification request and result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationChange {
    /// Unique change ID
    pub id: Uuid,

    /// Who initiated the change
    pub operator: String,

    /// Operator's role
    pub role: Role,

    /// Timestamp of change request
    pub timestamp: DateTime<Utc>,

    /// Settings being modified
    pub changes: Vec<SettingChange>,

    /// Validation result
    pub validation: ValidationResult,

    /// Whether change requires restart
    pub requires_restart: bool,

    /// Whether change was applied
    pub applied: bool,

    /// Security warnings (if any)
    pub warnings: Vec<SecurityWarning>,

    /// Operator's confirmation (for security changes)
    pub confirmed: bool,
}

/// Individual setting modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingChange {
    /// Setting name
    pub name: String,

    /// Value before change
    pub before: serde_json::Value,

    /// Value after change
    pub after: serde_json::Value,

    /// Is this setting security-affecting?
    pub security_affecting: bool,
}

/// Configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation status
    pub valid: bool,

    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,

    /// Warnings (non-blocking)
    pub warnings: Vec<String>,

    /// Constitutional violations (if any)
    pub constitution_violations: Vec<String>,
}

/// Specific validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Setting that failed validation
    pub setting: String,

    /// Error message
    pub message: String,

    /// Expected value range/format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,

    /// Actual value provided
    pub actual: String,
}

/// Security warning for configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    /// Warning severity
    pub level: WarningLevel,

    /// Human-readable warning message
    pub message: String,

    /// Setting triggering the warning
    pub affected_setting: String,

    /// Risk explanation
    pub risk_explanation: String,

    /// Suggested alternative (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative: Option<String>,
}

/// Security warning severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningLevel {
    /// Informational (no risk)
    Info,

    /// Minor security impact
    Low,

    /// Moderate security impact
    Medium,

    /// Significant security impact (requires confirmation)
    High,

    /// Critical security downgrade (requires explicit confirmation)
    Critical,
}

/// Immutable audit log record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: Uuid,

    /// Entry timestamp
    pub timestamp: DateTime<Utc>,

    /// Operator who made the change
    pub operator: String,

    /// Operator's role
    pub role: Role,

    /// Action performed
    pub action: AuditAction,

    /// Settings changed
    pub changes: Vec<SettingChange>,

    /// Whether change was successfully applied
    pub applied: bool,

    /// Security warnings shown (if any)
    pub warnings_shown: Vec<String>,

    /// Operator confirmation (if required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<String>,

    /// Hash of previous audit entry (for tamper detection)
    pub prev_hash: String,

    /// SHA256 hash of this entry
    pub hash: String,
}

/// Type of audit event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    /// Configuration changed via UI
    ConfigChange,

    /// Configuration exported
    ConfigExport,

    /// Configuration imported (preview)
    ConfigImportPreview,

    /// Configuration imported (applied)
    ConfigImportApply,

    /// Configuration rolled back
    ConfigRollback,

    /// Authentication failure
    AuthFailure,

    /// Authorization failure
    AuthzFailure,
}

/// RBAC role for admin API access
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Read-only access
    Viewer,

    /// Can modify non-security settings
    Operator,

    /// Can modify all settings
    Admin,
}

/// API key credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Base64-encoded key value
    pub key: String,

    /// Associated role
    pub role: Role,

    /// Key owner name (for audit logging)
    pub name: String,

    /// Optional expiration timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    /// Settings to change
    pub changes: Vec<SettingUpdateRequest>,

    /// Confirmation flag for security warnings
    #[serde(default)]
    pub confirmed: bool,
}

/// Individual setting update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingUpdateRequest {
    /// Setting name
    pub name: String,

    /// New value
    pub value: serde_json::Value,
}

/// Configuration import preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    /// Validation result
    pub validation: ValidationResult,

    /// Diff comparing imported vs. current config
    pub diff: Vec<SettingChange>,

    /// Whether import requires restart
    pub requires_restart: bool,

    /// Security warnings
    pub warnings: Vec<SecurityWarning>,
}

impl Default for TlsModeStats {
    fn default() -> Self {
        Self {
            classical_count: 0,
            hybrid_count: 0,
            pqc_count: 0,
            last_updated: Utc::now(),
        }
    }
}

impl Default for HandshakeStats {
    fn default() -> Self {
        Self {
            recent_success_count: 0,
            recent_failure_count: 0,
            avg_duration_ms: 0.0,
            success_rate: 1.0,
        }
    }
}

impl Default for OperationalStatus {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            total_connections: 0,
            active_connections: 0,
            tls_mode_stats: TlsModeStats::default(),
            handshake_stats: HandshakeStats::default(),
        }
    }
}

impl ValidationResult {
    /// Create a valid result with no errors
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            constitution_violations: Vec::new(),
        }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
            constitution_violations: Vec::new(),
        }
    }

    /// Check if there are any constitutional violations
    pub fn has_constitution_violations(&self) -> bool {
        !self.constitution_violations.is_empty()
    }
}
