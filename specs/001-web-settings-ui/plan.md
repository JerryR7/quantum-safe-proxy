# Implementation Plan: Web-Based Settings Management UI

**Branch**: `001-web-settings-ui` | **Date**: 2025-12-30 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-web-settings-ui/spec.md`

## Summary

Build a web-based settings management interface for Quantum Safe Proxy that allows administrators to view and modify configuration through a UI instead of manual file editing. The feature provides visibility into effective configuration, supports hot-reload where possible, includes audit logging with tamper-evidence (SHA256 hash chaining), and maintains security constraints including explicit warnings for security-degrading changes (No Silent Downgrade principle). This is an MVP to improve usability of existing configuration, not a complete redesign of the configuration system.

**Technical Approach**:
- Extend existing Rust async proxy daemon with embedded admin API using axum HTTP framework
- Reuse existing ConfigManager for hot-reload integration (no config redesign)
- Simple persistence: existing config files + append-only JSONL audit logs (no database)
- Minimal admin surface: HTTP API + single embedded HTML file (no Node.js, no separate frontend build)
- Security: API key authentication with RBAC (viewer/operator/admin roles)
- Constitution compliance: Explicit security warnings, crypto mode classification telemetry, trust boundary documentation

## Technical Context

**Language/Version**: Rust 1.86.0 (edition 2021)

**Primary Dependencies**:
- tokio 1.44 (existing) - async runtime
- openssl 0.10 (existing) - TLS/crypto
- axum 0.7 (new) - HTTP framework built on hyper + tower
- tower 0.4 (new) - middleware layer
- serde/serde_json 1.0 (existing) - serialization
- clap 4 (existing) - CLI parsing
- config 0.14 (existing) - config file parsing
- uuid 1.0 (new) - audit log entry IDs
- sha2 0.10 (new) - audit log hash chaining

**Storage**:
- Configuration: Existing config files (TOML/JSON) + in-memory ConfigManager
- Audit Log: Append-only JSONL file at `/var/log/quantum-safe-proxy/admin-audit.jsonl`
- API Keys: In main config file under `[admin.api_keys]` section
- No database required

**Testing**: cargo test (existing integration tests in tests/)

**Target Platform**: Linux/Unix (primary), Windows (fallback for hot-reload via polling)

**Project Type**: Single project (Rust daemon with embedded admin API module)

**Performance Goals**:
- Configuration validation: <2 seconds (SC-008)
- Settings page load: <3 seconds (performance constraint)
- View effective config: <5 seconds (SC-001)
- Hot-reload completion: <5 seconds (performance constraint)
- Audit log queries: <1 second for typical date ranges (performance constraint)

**Constraints**:
- No redesign of existing configuration model
- No new databases, queues, or external services
- No Node.js or separate frontend build (single embedded HTML file)
- Reuse existing ConfigManager hot-reload mechanism
- Maintain existing ValueSource precedence: CLI > Env > UI (new) > File > Default
- Support 10 concurrent administrators (SC-007)
- Maintain 90-day audit log retention (FR-020)

**Scale/Scope**: MVP - single proxy instance, admin API for local management (127.0.0.1 default)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Security Is Non-Negotiable

**Status**: ✅ COMPLIANT

- All security-affecting changes require explicit warnings (SecurityWarning type, FR-010, FR-011)
- Validation includes constitutional constraint checking (ValidationResult.constitution_violations)
- Authentication required for all admin API endpoints (FR-027, API key + Bearer token)
- Authorization via RBAC with viewer/operator/admin roles (FR-028)
- No feature reduces security guarantees

**Evidence**: SecurityWarning type enforces explicit risk communication; admin API is authentication-gated.

---

### Principle II: Explicit Trust Boundaries

**Status**: ✅ COMPLIANT

**Trust Boundaries Defined**:

1. **Client ↔ Proxy** (existing TLS termination boundary)
   - Authentication: mTLS with client certificate validation
   - Certificate ownership: Client owns client cert + private key; Proxy owns CA cert for validation
   - Validation responsibility: Proxy validates client cert against configured CA
   - Failure behavior: Reject TLS handshake with alert, log failure, no data processing

2. **Proxy ↔ Upstream** (existing plaintext forwarding boundary)
   - Authentication: None (implicit localhost trust)
   - Certificate ownership: N/A (plaintext)
   - Validation responsibility: N/A
   - Failure behavior: Connection error, log failure, return error to client

3. **Admin API ↔ Proxy Control Plane** (new boundary introduced by this feature)
   - Authentication: API key via HTTP Bearer token
   - Certificate ownership: Admin client owns API key; Proxy owns authorized key database (config file)
   - Validation responsibility: Proxy validates API key against configured authorized keys in constant-time
   - Failure behavior: HTTP 401 Unauthorized, audit log entry, no configuration access

**Evidence**: All three boundaries documented with required elements per Principle II.

---

### Principle III: No Silent Downgrade (NON-NEGOTIABLE)

**Status**: ✅ COMPLIANT

- All security-degrading changes trigger SecurityWarning with level::Critical (FR-010)
- Explicit operator confirmation required before applying security downgrades (FR-011, confirmed flag)
- Audit log records warnings shown and confirmation provided (AuditEntry.warnings_shown, confirmation)
- No auto-apply of security changes without user acknowledgment (FR-012)

**Security-Affecting Changes** (trigger warnings):
- Enabling classical TLS fallback
- Disabling crypto mode classification
- Weakening certificate validation (allow_invalid_certificates)
- Disabling client authentication (client_cert_mode: none)
- Enabling passthrough mode (bypasses crypto classification)

**Evidence**: SecurityWarning.level::Critical requires explicit confirmation; all downgrades logged.

---

### Principle IV: Cryptographic Mode Classification (MANDATORY)

**Status**: ⚠️ REQUIRES IMPLEMENTATION

**Required Tasks** (added as Phase 9 in tasks.md):
- T054: Add crypto_mode field to ConnectionMetrics in telemetry module
- T055: Implement TLS handshake inspection in existing handshake handler
- T056: Classify connections as classical/hybrid/PQC based on cipher suite (OpenSSL inspection)
- T057: Emit telemetry with security.crypto_mode, security.tls.version, security.handshake.result
- T058: Integration test for classical TLS classification
- T059: Integration test for hybrid TLS classification (X25519MLKEM768)

**Classification Logic** (from research.md R6):
```rust
fn classify_crypto_mode(ssl: &SslRef) -> CryptoMode {
    let cipher_name = ssl.current_cipher().unwrap().name();
    if cipher_name.contains("MLKEM") || cipher_name.contains("KYBER") {
        if cipher_name.contains("X25519") || cipher_name.contains("P256") {
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
- TlsModeStats tracks classical_count, hybrid_count, pqc_count
- Displayed in OperationalStatus (GET /api/status)
- Observable via structured logs and metrics

**Evidence**: TlsModeStats type in data model; telemetry tasks in Phase 9.

---

### Principle V: Policy-Driven Behavior Only

**Status**: ✅ COMPLIANT

- All configuration behavior driven by ConfigManager (existing, policy-based)
- No hardcoded assumptions about environment or deployment
- Hot-reload vs. restart determined by setting metadata (not hardcoded)
- Security warnings based on configurable security-affecting settings list

**Evidence**: Reuses existing ConfigManager; no hardcoded security decisions.

---

### Principle VI: Observability Is a Feature

**Status**: ✅ COMPLIANT

- Comprehensive audit logging for all configuration changes (AuditEntry, FR-018, FR-019)
- Structured logs for admin API authentication/authorization events (FR-029)
- TLS mode statistics visible via OperationalStatus (FR-003, TlsModeStats)
- Handshake success/failure metrics (HandshakeStats)
- Error messages are actionable (ValidationError includes expected/actual values)

**Evidence**: AuditEntry provides complete change history; OperationalStatus provides runtime visibility.

---

### Principle VII: Test-Proven Security

**Status**: ✅ COMPLIANT

- Integration tests for all critical paths (Phase 8 tasks):
  - T049: Admin API endpoint tests
  - T050: Configuration validation tests
  - T051: Security warning flow tests
  - T058-T059: Crypto classification tests (Phase 9)
- Tests written before implementation (TDD approach in tasks.md)
- All security-relevant behavior has explicit test coverage

**Evidence**: Integration test tasks in Phase 8 and Phase 9 cover all critical security paths.

---

### Principle VIII: No Overengineering (MANDATORY DISCIPLINE)

**Status**: ✅ COMPLIANT

**Abstractions Justified**:
- axum HTTP framework: 2+ consumers (all handler functions), proven necessity for HTTP API
- ResolvedConfig type: Augments existing ProxyConfig with source tracking (concrete need)
- AuditLog: Required for tamper-evident audit trail (FR-018, FR-020)

**No Speculative Extensibility**:
- No plugin system (explicitly out of scope)
- No generic configuration framework (reuses existing ConfigManager)
- No multi-tenant support (single proxy instance)
- No advanced version control (only single rollback, FR-016)
- Single embedded HTML file (no frontend framework, no build step)

**Complexity Justification**: All components serve specific requirements from spec.md.

**Evidence**: Minimal abstractions; reuses existing ConfigManager; no frameworks beyond necessities.

---

### Summary

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Security Is Non-Negotiable | ✅ COMPLIANT | Security warnings enforced, auth required |
| II. Explicit Trust Boundaries | ✅ COMPLIANT | All three boundaries documented |
| III. No Silent Downgrade | ✅ COMPLIANT | Explicit confirmation for all downgrades |
| IV. Cryptographic Mode Classification | ⚠️ IMPLEMENTATION REQUIRED | Tasks added in Phase 9 |
| V. Policy-Driven Behavior Only | ✅ COMPLIANT | Reuses ConfigManager, no hardcoding |
| VI. Observability Is a Feature | ✅ COMPLIANT | Audit log + telemetry + metrics |
| VII. Test-Proven Security | ✅ COMPLIANT | Integration tests for all critical paths |
| VIII. No Overengineering | ✅ COMPLIANT | Minimal abstractions, no speculative code |

**GATE STATUS**: ✅ PASS (with implementation requirement for Principle IV in Phase 9)

## Project Structure

### Documentation (this feature)

```text
specs/001-web-settings-ui/
├── spec.md                 # Feature specification (completed)
├── plan.md                 # This file (completed)
├── research.md             # Phase 0 output (completed)
├── data-model.md           # Phase 1 output (completed)
├── quickstart.md           # Phase 1 output (completed)
├── contracts/              # Phase 1 output (completed)
│   └── admin-api.yaml      # OpenAPI 3.0 specification
├── checklists/
│   └── requirements.md     # Specification quality checklist (completed)
└── tasks.md                # Phase 2 output (completed - 60 tasks in 10 phases)
```

### Source Code (repository root)

```text
src/
├── main.rs                 # MODIFIED: Spawn admin server with tokio::spawn
├── lib.rs                  # Existing library root
├── common/                 # Existing (buffer_pool, error, log)
├── config/                 # Existing (EXTENDED: add UI variant to ValueSource)
│   ├── manager.rs          # MODIFIED: Extend update() for admin API integration
│   ├── types.rs            # MODIFIED: Extend ProxyConfig with UI source tracking
│   └── validator.rs        # EXTENDED: Add security constraint validation
├── crypto/                 # Existing (capabilities, environment, loader, openssl)
├── protocol/               # Existing (detector)
├── proxy/                  # Existing (forwarder, handler, message, server, service)
├── tls/                    # EXISTING: Will add crypto mode classification in acceptor
│   ├── acceptor.rs         # MODIFIED: Add crypto mode classification after handshake
│   └── ...
└── admin/                  # NEW: Admin API module
    ├── mod.rs              # Public exports
    ├── types.rs            # Data model (ResolvedConfig, AuditEntry, etc.)
    ├── server.rs           # HTTP server setup, routing
    ├── handlers.rs         # Request handlers (GET/PATCH /config, etc.)
    ├── auth.rs             # Authentication/authorization middleware
    ├── audit.rs            # Audit log (append-only JSONL with hash chaining)
    ├── error.rs            # Admin-specific error types
    └── html.rs             # Embedded HTML UI (include_str!())

web/
└── admin-ui.html           # NEW: Single embedded HTML file (no build step)

tests/
├── integration/            # NEW: Admin API integration tests
│   ├── admin_api.rs        # API endpoint tests (T049)
│   ├── config_validation.rs # Validation tests (T050)
│   ├── security_warnings.rs # Security warning tests (T051)
│   └── crypto_classification.rs # Crypto mode tests (T058-T059)
└── ... (existing tests)

Cargo.toml                  # MODIFIED: Add axum, tower, uuid, sha2 dependencies
```

**Structure Decision**: Single project (Option 1). Admin API is a module within the existing Rust daemon, not a separate service. This maintains simplicity and reuses the existing async runtime (tokio) and configuration management (ConfigManager).

---

## Architecture

### Component Diagram

```
┌────────────────────────────────────────────────────────────────┐
│                    Quantum Safe Proxy                           │
│                                                                  │
│  ┌─────────────────┐          ┌──────────────────┐             │
│  │   Main Runtime  │          │   Admin API      │             │
│  │   (existing)    │          │   (new module)   │             │
│  │                 │          │                  │             │
│  │  ┌───────────┐  │          │  ┌────────────┐  │             │
│  │  │ TLS       │  │          │  │ HTTP       │  │             │
│  │  │ Acceptor  │◄─┼──reads───┼──│ Server     │  │             │
│  │  │           │  │  stats   │  │ (axum)     │  │             │
│  │  └─────┬─────┘  │          │  └──────┬─────┘  │             │
│  │        │        │          │         │        │             │
│  │        │ classifies       │         │ auth   │             │
│  │        ▼        │          │         ▼        │             │
│  │  ┌───────────┐  │          │  ┌────────────┐  │             │
│  │  │ Crypto    │  │          │  │ Auth       │  │             │
│  │  │ Mode      │──┼─emits────┼─►│ Middleware │  │             │
│  │  │ Classifier│  │ metrics  │  └──────┬─────┘  │             │
│  │  └───────────┘  │          │         │        │             │
│  │        │        │          │         ▼        │             │
│  │        │        │          │  ┌────────────┐  │             │
│  │        ▼        │          │  │ Handlers   │  │             │
│  │  ┌───────────┐  │          │  │            │  │             │
│  │  │Telemetry  │  │          │  │ GET/PATCH  │  │             │
│  │  │(metrics)  │◄─┼──reads───┼──│ /config    │  │             │
│  │  └───────────┘  │          │  └──────┬─────┘  │             │
│  │                 │          │         │        │             │
│  │  ┌───────────┐  │          │         │        │             │
│  │  │Config     │  │          │         │        │             │
│  │  │Manager    │◄─┼──update──┼─────────┘        │             │
│  │  │(existing) │  │          │                  │             │
│  │  └─────┬─────┘  │          │  ┌────────────┐  │             │
│  │        │        │          │  │ Audit Log  │  │             │
│  │        │ hot    │          │  │ (JSONL +   │  │             │
│  │        │ reload │          │  │ SHA256)    │  │             │
│  │        ▼        │          │  └────────────┘  │             │
│  │  ┌───────────┐  │          │                  │             │
│  │  │Config     │  │          │  ┌────────────┐  │             │
│  │  │Files      │  │          │  │ Embedded   │  │             │
│  │  │(TOML)     │  │          │  │ HTML UI    │  │             │
│  │  └───────────┘  │          │  └────────────┘  │             │
│  └─────────────────┘          └──────────────────┘             │
│                                                                  │
└────────────────────────────────────────────────────────────────┘
           │                              │
           │ TLS                          │ HTTP
           ▼                              ▼
    ┌────────────┐               ┌─────────────────┐
    │  Clients   │               │  Administrators │
    │            │               │  (browser)      │
    └────────────┘               └─────────────────┘
```

### Data Flow

#### 1. Configuration Read Flow (GET /api/config)

```
Browser → HTTP Request (Bearer token)
       → Auth Middleware (validate API key)
       → GET /config Handler
       → ConfigManager.get_config()
       → ProxyConfig → ResolvedConfig (derive with source tracking)
       → JSON Response
```

#### 2. Configuration Modify Flow (PATCH /api/config)

```
Browser → HTTP Request (changes + confirmed flag)
       → Auth Middleware
       → PATCH /config Handler
       → Validate changes (validator.rs)
       → Detect security warnings (is_security_downgrade)
       → IF warnings && !confirmed:
            → Return warnings (require confirmation)
       → IF hot_reloadable:
            → ConfigManager.update()
            → Apply changes immediately
       → ELSE:
            → Mark requires_restart = true
            → Do NOT apply
       → AuditLog.append(entry)
       → Return ConfigurationChange
```

#### 3. Crypto Mode Classification Flow (Constitution Principle IV)

```
TLS Handshake → TLS Acceptor (acceptor.rs)
             → Handshake Complete
             → SSL_get_current_cipher()
             → classify_crypto_mode(cipher_name)
             → CryptoMode { Classical | Hybrid | PQC }
             → Update TlsModeStats counters
             → Emit telemetry (security.crypto_mode, security.tls.version)
             → Log handshake result
```

#### 4. Audit Log Hash Chaining

```
ConfigurationChange → Create AuditEntry
                   → Load last entry hash (prev_hash)
                   → Serialize entry to JSON (without hash field)
                   → Calculate hash = SHA256(prev_hash || json)
                   → Append to audit log file
                   → JSONL format (one entry per line)
```

---

## Phase 0: Research (COMPLETED)

See [research.md](./research.md) for detailed technical decisions.

**Key Decisions**:
1. HTTP Framework: axum 0.7 with tower middleware
2. Authentication: Bearer token with in-memory API key validation
3. Audit Log: JSONL with SHA256 hash chaining (no database)
4. Hot-Reload: Extend existing ConfigManager.update()
5. UI Embedding: include_str!() with single HTML file
6. Crypto Classification: OpenSSL cipher suite inspection
7. Data Model: Augment existing ProxyConfig with source tracking
8. Validation: Extend existing validator.rs with security constraint checking

**All technical unknowns resolved** - no NEEDS CLARIFICATION remaining.

---

## Phase 1: Design & Contracts (COMPLETED)

### Data Model

See [data-model.md](./data-model.md) for complete entity definitions.

**Core Entities**:
- ResolvedConfig (runtime config with source tracking)
- ResolvedSetting (individual setting with metadata)
- ConfigurationChange (change request and result)
- AuditEntry (immutable audit record with hash chaining)
- SecurityWarning (explicit risk communication)
- ValidationResult (validation outcome)
- OperationalStatus (runtime health + TLS mode stats)
- TlsModeStats (classical/hybrid/PQC counts - Principle IV)
- Role (Viewer/Operator/Admin)
- ApiKey (authentication credential)

### API Contracts

See [contracts/admin-api.yaml](./contracts/admin-api.yaml) for complete OpenAPI specification.

**Endpoints**:
- GET /api/config - View effective configuration
- PATCH /api/config - Modify settings with validation
- POST /api/config/rollback - Rollback to previous config
- POST /api/config/export - Export configuration (JSON/YAML)
- POST /api/config/import - Import with preview and validation
- GET /api/status - Operational status (uptime, connections, TLS modes)
- GET /api/audit - Query audit log (filtering + pagination)
- GET /api/audit/:id - Get specific audit entry
- POST /api/audit/export - Export audit log for compliance
- GET / - Serve embedded HTML UI

### Quickstart Guide

See [quickstart.md](./quickstart.md) for developer implementation guide.

---

## Phase 2: Task Breakdown (COMPLETED)

See [tasks.md](./tasks.md) for detailed task list.

**Summary**: 65 tasks organized into 10 phases (updated after specification analysis to address critical gaps):
1. Phase 1: Project Setup (5 tasks)
2. Phase 2: HTTP Server & Authentication (6 tasks - includes T061 for auth/authz logging)
3. Phase 3: User Story 1 - View Config (7 tasks)
4. Phase 4: User Story 2 - Modify Non-Security Settings (7 tasks)
5. Phase 5: User Story 3 - Security Safeguards (8 tasks)
6. Phase 6: User Story 4 - Import/Export (7 tasks)
7. Phase 7: User Story 5 - Audit Trail (6 tasks)
8. Phase 8: UI & Integration (8 tasks)
9. Phase 9: Constitution Compliance - Trust Boundaries (1 task)
10. Phase 10: Constitution Compliance & Production Hardening (10 tasks)
    - Subphase 10A: Crypto Classification (T055-T060) - MANDATORY for constitution compliance
    - Subphase 10B: Security Hardening (T062-T065) - CRITICAL for production (concurrent edits, export sanitization, audit verification, external config detection)

**Critical Path**: Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 8

**Parallel Branches**:
- Phase 3 → Phase 6 (Import/Export independent of modification)
- Phase 5 → Phase 7 (Audit trail depends on audit logging)
- Phase 9-10 (Constitution compliance can be parallel with Phase 8)

---

## Complexity Tracking

No constitutional violations to justify. All design decisions align with constitution principles.

| Principle | Compliance | Justification |
|-----------|------------|---------------|
| I. Security | ✅ | Security warnings enforced, auth required |
| II. Trust Boundaries | ✅ | All three boundaries documented (Phase 9) |
| III. No Silent Downgrade | ✅ | Explicit confirmation for all downgrades |
| IV. Crypto Classification | ⚠️ | Implementation tasks in Phase 10 |
| V. Policy-Driven | ✅ | Reuses ConfigManager, no hardcoding |
| VI. Observability | ✅ | Audit log + telemetry + metrics |
| VII. Test-Proven | ✅ | Integration tests for all critical paths |
| VIII. No Overengineering | ✅ | Minimal abstractions, justified complexity |

---

## Implementation Status

- ✅ Specification Complete (spec.md) - Updated with clarifications after analysis
- ✅ Planning Complete (this file)
- ✅ Research Complete (research.md)
- ✅ Data Model Complete (data-model.md)
- ✅ API Contracts Complete (contracts/admin-api.yaml)
- ✅ Quickstart Guide Complete (quickstart.md)
- ✅ Task Breakdown Complete (tasks.md - 65 tasks after specification analysis)
- ✅ Phases 1-9 Implementation Complete (55/55 tasks)
- ⏸️  Phase 10 Implementation Pending (10 tasks remaining - CRITICAL for production)

**Specification Analysis Results (2025-12-30)**:
- Identified and resolved 2 CRITICAL gaps (concurrent edits, Principle IV compliance)
- Identified and resolved 3 HIGH-priority underspecifications (permissions, export sanitization, audit verification)
- Added 3 new tasks (T063-T065) to address production security requirements
- Updated requirements with clarifications for initial setup, security-affecting settings list, and export sanitization

**Next Steps**:
1. ✅ Review specification consistency (completed via `/speckit.analyze`)
2. ⏸️  Begin Phase 10 implementation (10 tasks remaining)
   - **MANDATORY**: Complete Subphase 10A (T055-T060) for constitution compliance
   - **CRITICAL**: Complete Subphase 10B (T062-T065) for production readiness
3. Follow TDD approach: write tests before implementation
4. Execute tasks sequentially within each subphase
5. Final validation: Run all tests, verify constitution compliance

**Branch**: `001-web-settings-ui`
**Plan Path**: `/mnt/d/Projects/quantum-safe-proxy/specs/001-web-settings-ui/plan.md`

---

**Plan Generation Complete** ✅
