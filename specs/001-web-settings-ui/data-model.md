# Data Model: Web-Based Settings Management UI

**Feature**: 001-web-settings-ui
**Date**: 2025-12-30
**Purpose**: Define core data structures and their relationships

## Core Entities

### 1. ResolvedConfig

**Purpose**: Single source of truth for runtime configuration, representing the merged result of all configuration sources.

**Structure**:
```rust
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
```

**Relationships**:
- Contains multiple `ResolvedSetting` instances
- Has one `OperationalStatus`
- Source: Derived from `ProxyConfig` (existing type in src/config/types.rs)

**Lifecycle**:
1. Created: On proxy startup or configuration reload
2. Updated: When ConfigManager.update() is called
3. Read: By admin API GET /api/config endpoint

---

### 2. ResolvedSetting

**Purpose**: Individual configuration setting with source tracking and metadata.

**Structure**:
```rust
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
    pub description: Option<String>,

    /// Is this a security-affecting setting?
    pub security_affecting: bool,
}
```

**Field Constraints**:
- `name`: Must be unique within ResolvedConfig
- `value`: Must match expected type for the setting
- `source`: One of CLI, Environment, File, UI, Default
- `hot_reloadable`: Derived from setting metadata (see research.md R4)
- `security_affecting`: True for settings in security warnings list (see research.md R8)

**Validation Rules**:
- Name must exist in ProxyConfig schema
- Value must pass type validation (validator.rs)
- Source must be valid ConfigSource variant

---

### 3. ConfigSource

**Purpose**: Indicates where a configuration value originated.

**Structure**:
```rust
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
```

**Precedence Order**: CommandLine > Environment > UI > File > Default

**Note**: This extends the existing `ValueSource` enum in src/config/types.rs by adding `UI` variant.

---

### 4. SettingCategory

**Purpose**: Groups settings for UI organization.

**Structure**:
```rust
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
```

---

### 5. OperationalStatus

**Purpose**: Provides runtime health and statistics for the proxy.

**Structure**:
```rust
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
```

---

### 6. TlsModeStats

**Purpose**: Tracks cryptographic mode classification (Constitution Principle IV).

**Structure**:
```rust
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
```

**Rationale**: Directly implements Constitution Principle IV (Cryptographic Mode Classification).

---

### 7. HandshakeStats

**Purpose**: Recent TLS handshake success/failure metrics.

**Structure**:
```rust
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
```

**Calculation**: Rolling 5-minute window, updated every 30 seconds.

---

### 8. ConfigurationChange

**Purpose**: Records a configuration modification request and result.

**Structure**:
```rust
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
```

**State Transitions**:
1. Created: When PATCH /api/config is called
2. Validated: After server-side validation
3. Warned: If security warnings detected
4. Confirmed: If operator acknowledges warnings
5. Applied: If hot-reloadable and validated
6. Logged: Written to audit log

---

### 9. SettingChange

**Purpose**: Individual setting modification within a ConfigurationChange.

**Structure**:
```rust
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
```

---

### 10. ValidationResult

**Purpose**: Result of configuration validation.

**Structure**:
```rust
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
```

---

### 11. ValidationError

**Purpose**: Specific validation failure.

**Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Setting that failed validation
    pub setting: String,

    /// Error message
    pub message: String,

    /// Expected value range/format
    pub expected: Option<String>,

    /// Actual value provided
    pub actual: String,
}
```

---

### 12. SecurityWarning

**Purpose**: Warns about security-degrading configuration changes.

**Structure**:
```rust
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
    pub alternative: Option<String>,
}
```

---

### 13. WarningLevel

**Purpose**: Security warning severity.

**Structure**:
```rust
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
```

---

### 14. AuditEntry

**Purpose**: Immutable audit log record of a configuration change.

**Structure**:
```rust
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
    pub confirmation: Option<String>,

    /// Hash of previous audit entry (for tamper detection)
    pub prev_hash: String,

    /// SHA256 hash of this entry
    pub hash: String,
}
```

**Hash Calculation**:
```
hash = SHA256(prev_hash || JSON.stringify(entry without hash field))
```

**First Entry**: Uses empty string as prev_hash.

---

### 15. AuditAction

**Purpose**: Type of audit event.

**Structure**:
```rust
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
```

---

### 16. Role

**Purpose**: RBAC role for admin API access.

**Structure**:
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Read-only access
    Viewer,

    /// Can modify non-security settings
    Operator,

    /// Can modify all settings
    Admin,
}
```

**Permissions**:
- **Viewer**: GET /api/config, GET /api/status, GET /api/audit
- **Operator**: Viewer + PATCH /api/config (non-security settings only)
- **Admin**: Operator + PATCH /api/config (all settings) + import/export

---

### 17. ApiKey

**Purpose**: Authentication credential for admin API.

**Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Base64-encoded key value
    pub key: String,

    /// Associated role
    pub role: Role,

    /// Key owner name (for audit logging)
    pub name: String,

    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
}
```

**Storage**: In config file under `[admin]` section (see research.md R2).

---

## Data Relationships

```
ResolvedConfig
├── settings: Vec<ResolvedSetting>
│   └── source: ConfigSource
│   └── category: SettingCategory
└── status: OperationalStatus
    ├── tls_mode_stats: TlsModeStats
    └── handshake_stats: HandshakeStats

ConfigurationChange
├── operator: String
├── role: Role
├── changes: Vec<SettingChange>
├── validation: ValidationResult
│   └── errors: Vec<ValidationError>
└── warnings: Vec<SecurityWarning>
    └── level: WarningLevel

AuditEntry
├── operator: String
├── role: Role
├── action: AuditAction
├── changes: Vec<SettingChange>
└── hash linkage (prev_hash → hash)
```

---

## Persistence Strategy

### ResolvedConfig
- **In-Memory**: Derived on-demand from ConfigManager
- **Not Persisted**: Reconstructed from ProxyConfig on each request

### ConfigurationChange
- **Transient**: Exists during request/response cycle
- **Logged**: Written to AuditEntry after completion

### AuditEntry
- **Persisted**: Append-only JSONL file at `/var/log/quantum-safe-proxy/admin-audit.jsonl`
- **Retention**: 90 days (rotated via logrotate or manual task)
- **Immutable**: Never modified after writing

### ApiKey
- **Persisted**: In main config file under `[admin.api_keys]` section
- **Reloaded**: On configuration hot-reload

---

## Validation Rules Summary

### ResolvedSetting
- Name must exist in ProxyConfig schema
- Value must match expected type (string, int, bool, array, etc.)
- Source must be valid ConfigSource
- Security-affecting flag must match predefined list

### ConfigurationChange
- At least one SettingChange must be present
- All changes must pass validation
- Security-affecting changes must have warnings
- Critical warnings require operator confirmation

### AuditEntry
- ID must be valid UUIDv4
- Timestamp must be valid ISO8601
- Hash must match SHA256(prev_hash || entry_json)
- Previous hash must match last entry's hash

### ApiKey
- Key must be base64-encoded, at least 32 bytes
- Role must be valid Role variant
- Name must be non-empty
- Expired keys must be rejected

---

## Constitution Compliance

### Principle I (Security Is Non-Negotiable)
- SecurityWarning type enforces explicit warnings
- ValidationResult includes constitution_violations field
- Security-affecting settings explicitly flagged

### Principle II (Explicit Trust Boundaries)
- ApiKey defines Admin API ↔ Proxy authentication
- Role defines authorization boundaries

### Principle III (No Silent Downgrade)
- SecurityWarning.level::Critical requires explicit confirmation
- AuditEntry.warnings_shown records all warnings displayed

### Principle IV (Cryptographic Mode Classification)
- TlsModeStats tracks classical/hybrid/pqc counts
- Directly maps to telemetry requirements

### Principle VI (Observability Is a Feature)
- OperationalStatus provides runtime visibility
- AuditEntry provides complete change history

### Principle VIII (No Overengineering)
- All types serve specific requirements
- No speculative extensibility (e.g., no plugin system)
- Reuses existing ProxyConfig structure

---

**Data Model Complete**: Ready for API contract definition.
