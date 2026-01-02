# Phase 0: Technical Research

**Feature**: Web-Based Settings Management UI
**Date**: 2025-12-30
**Purpose**: Resolve technical unknowns before detailed design

## Research Questions

### R1: HTTP Framework Selection (axum)

**Question**: What is the minimal axum setup for an embedded admin API in an existing tokio application?

**Decision**: Use axum 0.7 with tower middleware

**Rationale**:
- Built on hyper 1.0 and tower (same foundation as tokio)
- Minimal overhead for existing tokio applications
- Excellent ergonomics for handler functions
- Built-in extractors for JSON, query params, path parameters
- Tower middleware for authentication
- Zero-allocation routing

**Alternatives Considered**:
- **actix-web**: Higher performance but incompatible ecosystem (uses its own actor runtime, not tokio)
- **warp**: Older filter-based API, less ergonomic than axum
- **rocket**: Requires nightly Rust (we use stable 1.86)
- **tide**: Async-std based (we use tokio)

**Integration Pattern**:
```rust
// In main.rs
let admin_server = admin::server::start(config.clone(), manager.clone());
tokio::spawn(admin_server);
```

**Dependencies to Add**:
```toml
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
```

---

### R2: API Key Authentication Strategy

**Question**: How should API key authentication be implemented for the admin API?

**Decision**: Bearer token with in-memory key validation

**Rationale**:
- Simple HTTP Bearer authentication (RFC 6750)
- Keys stored in config file (no database needed)
- Role-based access control (viewer/operator/admin) via key metadata
- Constant-time comparison to prevent timing attacks
- Keys can be generated with standard tools (openssl rand -base64 32)

**Implementation**:
```rust
// In config file
[admin]
listen = "127.0.0.1:8443"
api_keys = [
  { key = "base64-encoded-key", role = "admin", name = "admin-user" },
  { key = "base64-encoded-key", role = "viewer", name = "readonly-user" }
]
```

**Tower Middleware**:
```rust
async fn auth_middleware(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = auth.token();
    match validate_api_key(key) {
        Some(role) => {
            req.extensions_mut().insert(role);
            Ok(next.run(req).await)
        }
        None => Err(StatusCode::UNAUTHORIZED)
    }
}
```

**Alternatives Considered**:
- **JWT tokens**: Overengineered for single-instance admin API
- **mTLS client certificates**: Complex setup for MVP
- **Basic auth**: Less secure (password-based)

---

### R3: Audit Log Format and Storage

**Question**: What format should the audit log use for tamper evidence?

**Decision**: JSONL (JSON Lines) with SHA256 hash chaining

**Rationale**:
- JSONL is append-only and streaming-friendly
- Each line is valid JSON (easy parsing, debugging)
- SHA256 hash chaining provides tamper evidence
- No database required (file-based)
- Log rotation via filesystem tools (logrotate)

**Format**:
```json
{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2025-12-30T10:30:00Z","operator":"admin-user","role":"admin","action":"config_change","setting":"log_level","before":"info","after":"debug","applied":true,"prev_hash":"abc123...","hash":"def456..."}
```

**Hash Calculation**:
```
hash = SHA256(prev_hash || json_string)
```

**First entry** uses empty string as prev_hash.

**File Location**: `/var/log/quantum-safe-proxy/admin-audit.jsonl`

**Rotation**: 90-day retention via logrotate or manual rotation task

**Alternatives Considered**:
- **SQLite database**: Adds dependency, overengineered for append-only log
- **Plain JSON array**: Not streaming-friendly, entire file must be rewritten
- **Structured logs only**: No tamper evidence

---

### R4: Hot-Reload Integration with Existing ConfigManager

**Question**: How does the existing ConfigManager support hot-reload, and how can the admin API integrate with it?

**Investigation** (examining existing code):

From `src/config/manager.rs`:
- ConfigManager wraps ProxyConfig with Arc<RwLock<ProxyConfig>>
- Provides `reload()` method for hot-reloading config from file
- Uses SIGHUP signal handler for Unix platforms
- Windows uses file polling (fallback)

**Decision**: Admin API will call ConfigManager::update() to trigger hot-reload

**Integration Pattern**:
```rust
// In admin handlers
async fn patch_config(
    State(manager): State<Arc<ConfigManager>>,
    Json(changes): Json<ConfigUpdate>,
) -> Result<Json<ConfigChange>, AdminError> {
    // Validate changes
    let validated = validate_changes(&changes)?;

    // Check if hot-reloadable
    if validated.requires_restart() {
        return Ok(Json(ConfigChange {
            requires_restart: true,
            applied: false,
            // ...
        }));
    }

    // Apply hot-reload
    manager.update(validated.into_config()).await?;

    Ok(Json(ConfigChange {
        requires_restart: false,
        applied: true,
        // ...
    }))
}
```

**Hot-Reloadable Settings**:
- log_level (env_logger can be reconfigured)
- buffer_size (new connections use new value)
- connection_timeout (new connections use new value)
- client_cert_mode (new connections use new validation mode)
- certificates (reload_certificates() already exists)

**Restart-Required Settings**:
- listen (binding address)
- target (upstream address)

---

### R5: Embedded HTML UI Pattern

**Question**: How should static HTML/JS/CSS be embedded and served?

**Decision**: Use include_str!() macro with inline CSS/JS (single HTML file)

**Rationale**:
- No build step required (no webpack, vite, etc.)
- Single HTML file with embedded CSS and vanilla JavaScript
- include_str!() compiles HTML into binary at build time
- Zero runtime dependencies
- Fast page load (no external assets)

**Implementation**:
```rust
// src/admin/html.rs
pub fn ui_html() -> &'static str {
    include_str!("../../web/admin-ui.html")
}

// In server.rs
async fn serve_ui() -> Html<&'static str> {
    Html(html::ui_html())
}
```

**HTML Structure**:
```html
<!DOCTYPE html>
<html>
<head>
    <title>QSP Admin</title>
    <style>/* Embedded CSS */</style>
</head>
<body>
    <div id="app"><!-- Vue.js or vanilla JS app --></div>
    <script>/* Embedded JavaScript */</script>
</body>
</html>
```

**JavaScript Approach**: Vanilla JS or lightweight framework (Alpine.js, Petite Vue) - no build step

**Alternatives Considered**:
- **Separate frontend build**: Violates "no Node.js" constraint
- **include_bytes!() with mime types**: More complex than single HTML
- **Rust templating (askama, tera)**: Overengineered for MVP

---

### R6: Cryptographic Mode Classification

**Question**: How can we detect classical/hybrid/PQC TLS connections using OpenSSL?

**Decision**: Inspect cipher suite negotiation via OpenSSL SSL_get_current_cipher()

**Rationale**:
- OpenSSL exposes negotiated cipher suite after handshake
- Cipher suite string indicates key exchange algorithm
- Hybrid ciphers have specific names (e.g., X25519MLKEM768)
- Can classify deterministically based on cipher suite name

**Classification Logic**:
```rust
fn classify_crypto_mode(ssl: &SslRef) -> CryptoMode {
    let cipher = ssl.current_cipher().unwrap();
    let name = cipher.name();

    if name.contains("MLKEM") || name.contains("KYBER") {
        if name.contains("X25519") || name.contains("P256") || name.contains("P384") {
            CryptoMode::Hybrid  // e.g., X25519MLKEM768
        } else {
            CryptoMode::PQC  // Pure PQC (if ever supported)
        }
    } else {
        CryptoMode::Classical  // Standard ECDHE, RSA, etc.
    }
}
```

**Telemetry Emission**:
```rust
metrics::counter!("tls.connections.total", 1, "mode" => mode.as_str());
metrics::histogram!("tls.handshake.duration", duration);
log::info!(
    target: "security",
    "TLS handshake complete: mode={}, cipher={}, version={}",
    mode, cipher_name, ssl.version_str()
);
```

**Integration Point**: After TLS handshake in `src/tls/acceptor.rs`

**Alternatives Considered**:
- **Manual inspection**: Inspect certificates for PQC signatures - less reliable
- **BoringSSL APIs**: We use OpenSSL, not BoringSSL
- **Application-layer tagging**: Not deterministic, requires client cooperation

---

### R7: ResolvedConfig Data Model

**Question**: How should ResolvedConfig be derived from existing ProxyConfig?

**Decision**: Reflection-like traversal of ProxyConfig fields with source tracking

**Rationale**:
- ProxyConfig already exists in src/config/types.rs
- Need to augment with source information (CLI/Env/File/Default)
- Use serde introspection to iterate fields
- ConfigManager tracks sources via ValueSource enum

**Implementation**:
```rust
#[derive(Serialize)]
pub struct ResolvedConfig {
    pub settings: Vec<ResolvedSetting>,
    pub status: OperationalStatus,
}

#[derive(Serialize)]
pub struct ResolvedSetting {
    pub name: String,
    pub value: serde_json::Value,
    pub source: String,  // "cli" | "environment" | "file" | "default"
    pub hot_reloadable: bool,
    pub category: SettingCategory,
    pub description: Option<String>,
}

impl From<&ProxyConfig> for ResolvedConfig {
    fn from(config: &ProxyConfig) -> Self {
        // Use serde_json to convert to Value, then introspect
        let json_value = serde_json::to_value(config).unwrap();
        // Build ResolvedSetting for each field...
    }
}
```

**Source Tracking**: ConfigManager already has ValueSource - extend to track per-field

---

### R8: Configuration Validation Strategy

**Question**: How should configuration validation be structured?

**Decision**: Extend existing validator.rs with security constraint validation

**Rationale**:
- src/config/validator.rs already exists
- Add security-specific validation (e.g., detect downgrades)
- Use validator pattern (fn validate_X(value) -> Result<(), Error>)
- Validation runs before applying changes

**Security Validation**:
```rust
pub fn is_security_downgrade(before: &ProxyConfig, after: &ProxyConfig) -> Option<SecurityWarning> {
    // Check for classical fallback enablement
    if !before.allow_classical_fallback && after.allow_classical_fallback {
        return Some(SecurityWarning {
            level: WarningLevel::Critical,
            message: "Enabling classical TLS fallback reduces security to pre-PQC levels",
            affected_setting: "allow_classical_fallback",
        });
    }
    // Check for passthrough mode
    // Check for cert validation weakening
    // ...
    None
}
```

---

### R9: Secret Redaction Strategy

**Question**: How should sensitive secrets be redacted when exporting configuration?

**Decision**: Define explicit secret field list and replace with "[REDACTED]" placeholder

**Rationale**:
- Security constraint (spec.md L192): "Exported configuration files must not contain sensitive secrets in plaintext"
- Need deterministic list of secret fields to redact
- Simple string replacement maintains JSON/YAML structure
- Clear indication to operators that secrets were removed

**Secret Fields Definition**:
```rust
const SECRET_FIELDS: &[&str] = &[
    "api_key",
    "api_keys",  // Array of API keys in admin config
    "password",
    "secret",
    "private_key",
    "certificate_key",  // Private key portion of certificates
    "token",
    "bearer_token",
];
```

**Redaction Implementation**:
```rust
pub fn redact_secrets(config: &serde_json::Value) -> serde_json::Value {
    match config {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                if SECRET_FIELDS.iter().any(|&secret| key.contains(secret)) {
                    redacted.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    redacted.insert(key.clone(), redact_secrets(value));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(redact_secrets).collect())
        }
        _ => config.clone(),
    }
}
```

**Application Points**:
- Export handler (T033, T034): Apply before returning config
- Audit log export (T044): API keys already redacted (only operator name shown)
- ResolvedConfig display: Show redacted values for secret settings

**Edge Cases**:
- Nested secrets: Recursive traversal handles nested objects
- Array of secrets: Each array element checked
- Partial matches: Field name contains secret keyword (e.g., "admin_api_key_salt")

**Alternatives Considered**:
- **Complete omission**: Less clear to operators that secrets existed
- **Hash display**: Confusing and not reversible for restore
- **Encryption**: Overengineered for export use case

---

## Summary of Technical Decisions

| Area | Decision | Key Rationale |
|------|----------|---------------|
| HTTP Framework | axum 0.7 + tower | Tokio-native, ergonomic, minimal overhead |
| Authentication | Bearer token + API keys | Simple, no database, RBAC support |
| Audit Log | JSONL + SHA256 chaining | Append-only, tamper-evident, no database |
| Hot-Reload | Extend ConfigManager::update() | Reuse existing hot-reload mechanism |
| UI Embedding | include_str!() + single HTML | No build step, zero runtime deps |
| Crypto Classification | OpenSSL cipher suite inspection | Deterministic, handshake-based detection |
| Data Model | Augment ProxyConfig with source | Derive from existing config types |
| Validation | Extend validator.rs | Reuse existing validation patterns |

## Open Questions for Phase 1

None - all technical unknowns resolved.

## Dependencies to Add

```toml
[dependencies]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
http = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
sha2 = "0.10"  # For audit log hash chaining
```

---

**Research Complete**: Ready for Phase 1 (Data Model & Contracts)
