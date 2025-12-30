# Implementation Tasks: Web-Based Settings Management UI

**Feature**: 001-web-settings-ui
**Branch**: `001-web-settings-ui`
**Generated**: 2025-12-30
**Spec**: [spec.md](./spec.md)
**Plan**: [plan.md](./plan.md)

## Task Legend

- `[P]` - Parallelizable (can run concurrently with other [P] tasks in same phase)
- `[US#]` - User Story reference (US1-US5 corresponding to P1-P5 priorities)
- `T###` - Task ID for dependency tracking
- File paths included in task descriptions for clarity

## Phase 1: Project Setup & Foundation

**Prerequisites**: None
**Deliverable**: Admin API module structure established

- [X] T001 [P] Create admin module structure in `src/admin/mod.rs` with public exports
- [X] T002 [P] Add axum 0.7 and tower dependencies to `Cargo.toml`
- [X] T003 [P] Define ResolvedConfig and ResolvedSetting types in `src/admin/types.rs` derived from existing ConfigValues
- [X] T004 [P] Define AuditEntry and AuditLog types in `src/admin/audit.rs` with JSONL format
- [X] T005 Create error types in `src/admin/error.rs` for admin-specific errors (validation, auth, persistence)

**Dependencies**: None (all tasks in this phase are foundational setup)

## Phase 2: HTTP Server & Authentication Foundation

**Prerequisites**: Phase 1 complete
**Deliverable**: Functioning HTTP server with authentication

- [X] T006 Implement HTTP server setup in `src/admin/server.rs` with axum Router and graceful shutdown
- [X] T007 Integrate admin server spawn in `src/main.rs` with tokio::spawn for concurrent execution
- [X] T008 [P] Implement API key authentication in `src/admin/auth.rs` with bearer token validation
- [X] T009 [P] Define RBAC roles (viewer/operator/admin) in `src/admin/auth.rs` with permission checking
- [X] T010 Create authentication middleware in `src/admin/auth.rs` for protecting admin endpoints
- [X] T061 [P] Implement authentication and authorization event logging in `src/admin/auth.rs` (FR-029: log all auth failures, successful logins, permission denials)

**Dependencies**: T006 blocks T007 (server must exist before integration). T061 can run parallel with other auth tasks.

## Phase 3: User Story 1 - View Effective Configuration [US1]

**Priority**: P1 (Foundation - highest priority)
**Prerequisites**: Phase 2 complete
**Deliverable**: Read-only configuration visibility
**Independent Test**: Access settings page, verify all displayed values match running config

- [X] T011 [US1] Implement GET /api/config handler in `src/admin/handlers.rs` returning ResolvedConfig
- [X] T012 [US1] Add logic to derive ResolvedConfig from ConfigManager in `src/admin/config_resolver.rs`
- [X] T013 [US1] Implement ValueSource tracking for each setting (CLI/Env/File/Default) in ResolvedSetting
- [X] T014 [US1] Add hot_reloadable flag classification logic in `src/admin/config_resolver.rs` (safe: log_level, buffer_size; restart: listen, target)
- [X] T015 [US1] [P] Create OperationalStatus type in `src/admin/types.rs` with TLS mode statistics
- [X] T016 [US1] [P] Implement basic operational status endpoint GET /api/status in `src/admin/handlers.rs`
- [X] T017 [US1] Add read-only mode enforcement in handlers based on user role (viewer role)

**Dependencies**: T011-T014 are sequential (handler → derivation → source tracking → classification). T015-T016 are parallel (status is independent).

## Phase 4: User Story 2 - Modify Non-Security Settings [US2]

**Priority**: P2
**Prerequisites**: Phase 3 complete (must have read capability first)
**Deliverable**: Operational settings modification with validation
**Independent Test**: Modify timeout setting, verify validation, confirm change takes effect

- [X] T018 [US2] Implement PATCH /api/config handler in `src/admin/handlers.rs` with JSON body parsing
- [X] T019 [US2] Integrate existing validator.rs logic for type/range validation in PATCH handler
- [X] T020 [US2] Add hot-reload detection in `src/admin/handlers.rs` to determine if restart required
- [X] T021 [US2] Create ConfigChange type in `src/admin/types.rs` with before/after values and validation result
- [X] T022 [US2] Implement diff generation logic in `src/admin/handlers.rs` comparing current vs. pending config
- [X] T023 [US2] Add integration with ConfigManager hot-reload mechanism (placeholder for actual implementation)
- [X] T024 [US2] Implement rollback capability in `src/admin/handlers.rs` with POST /api/config/rollback endpoint

**Dependencies**: T018-T020 are sequential (handler → validation → reload detection). T021-T022 parallel (types can be defined independently). T023 requires T020 (reload integration needs reload detection).

## Phase 5: User Story 3 - Security-Critical Settings with Safeguards [US3]

**Priority**: P3
**Prerequisites**: Phase 4 complete (builds on modification capability)
**Deliverable**: Security warnings and audit logging for critical changes
**Independent Test**: Attempt to enable classical TLS fallback, verify explicit warning, confirm logged

- [X] T025 [US3] Define security-affecting settings list in `src/admin/config_resolver.rs`
- [X] T026 [US3] Implement security warning detection in `src/admin/handlers.rs` for PATCH requests
- [X] T027 [US3] Add SecurityWarning type in `src/admin/types.rs` with risk level and message
- [X] T028 [US3] Create confirmation flow for security changes in PATCH handler requiring explicit acknowledgment
- [X] T029 [US3] Implement audit log writing in `src/admin/audit.rs` with append-only JSONL format
- [X] T030 [US3] Add SHA256 hash chaining in `src/admin/audit.rs` for tamper evidence
- [X] T031 [US3] Log all security-affecting changes with operator identity, timestamp, before/after values in `src/admin/handlers.rs`
- [X] T032 [US3] Enforce "No Silent Downgrade" principle by making all security reductions require explicit confirmation

**Dependencies**: T025-T027 are parallel (definitions). T028-T029 sequential (confirmation → audit). T030 can be parallel with T029. T031-T032 require all previous tasks.

## Phase 6: User Story 4 - Export and Import Configuration [US4]

**Priority**: P4
**Prerequisites**: Phase 3 complete (needs config read capability)
**Deliverable**: Configuration import/export with validation
**Independent Test**: Export config, modify externally, import for preview, verify no auto-apply

- [X] T033 [US4] [P] Implement POST /api/config/export handler in `src/admin/handlers.rs` returning JSON/YAML
- [X] T034 [US4] [P] Add format parameter to export endpoint supporting both JSON and YAML formats
- [X] T035 [US4] Implement POST /api/config/import handler in `src/admin/handlers.rs` with file upload
- [X] T036 [US4] Add import validation in `src/admin/handlers.rs` checking compatibility with current QSP version
- [X] T037 [US4] Implement diff generation for import preview comparing imported vs. current config
- [X] T038 [US4] Add dry-run mode to import endpoint preventing auto-application in `src/admin/handlers.rs`
- [X] T039 [US4] Create import confirmation flow requiring explicit operator approval before applying

**Dependencies**: T033-T034 parallel (export). T035-T039 sequential (import → validate → diff → dry-run → confirm).

## Phase 7: User Story 5 - View Configuration Audit Trail [US5]

**Priority**: P5
**Prerequisites**: Phase 5 complete (audit logging must be implemented)
**Deliverable**: Audit log query and filtering
**Independent Test**: Make several config changes, verify all appear in audit log with metadata

- [X] T040 [US5] Implement GET /api/audit handler in `src/admin/handlers.rs` reading JSONL audit log
- [X] T041 [US5] Add query parameters for audit log filtering (setting_name, operator, date_range) in handler
- [X] T042 [US5] Implement pagination for audit log results with limit/offset parameters
- [X] T043 [US5] Add audit log entry detail endpoint GET /api/audit/:id in `src/admin/handlers.rs`
- [X] T044 [US5] Implement audit log export functionality POST /api/audit/export for compliance reporting
- [X] T045 [US5] Add audit log rotation logic in `src/admin/audit.rs` maintaining 90-day retention

**Dependencies**: T040-T042 sequential (read → filter → paginate). T043-T044 parallel. T045 can be independent.

## Phase 8: UI & Integration

**Prerequisites**: Phase 3-7 complete (all API endpoints functional)
**Deliverable**: Complete web-based settings UI
**Independent Test**: Full end-to-end workflow via browser

- [X] T046 [P] Create embedded HTML UI in `src/admin/html.rs` using include_str!() macro
- [X] T047 [P] Add static file serving for HTML UI via GET / endpoint in `src/admin/server.rs`
- [X] T048 [P] Implement JavaScript client for API interaction in embedded HTML
- [X] T049 Add integration tests in `tests/integration/admin_api.rs` for all endpoints
- [X] T050 Add configuration change validation tests in `tests/integration/config_validation.rs`
- [X] T051 Add security warning flow tests in `tests/integration/security_warnings.rs`
- [X] T052 Update README.md with admin API documentation and usage examples
- [X] T053 Add admin API configuration section to existing config.toml example

**Dependencies**: T046-T048 are parallel (UI components). T049-T051 parallel (tests). T052-T053 parallel (docs).

## Phase 9: Trust Boundary Documentation

**Prerequisites**: Phase 8 complete
**Deliverable**: Security documentation
**Independent Test**: Review docs for completeness

- [X] T054 Document all three trust boundaries in `docs/admin-api-trust-boundaries.md`

## Phase 10: Constitution Compliance & Production Hardening

**Prerequisites**: Phase 8 complete
**Deliverable**: Full constitution compliance, production-ready security features
**Independent Test**: Verify all constitution principles satisfied, edge cases handled

### Subphase 10A: Cryptographic Mode Classification (Constitution Principle IV - MANDATORY)

- [X] T055 Add crypto mode classification in `src/proxy/handler.rs` based on cipher suite inspection (classical/hybrid/PQC)
- [X] T056 Implement telemetry emission for TLS modes (security.crypto_mode, security.tls.version, security.handshake.result)
- [X] T057 Add integration tests for crypto classification in `tests/integration/crypto_classification.rs`
- [X] T058 Update OperationalStatus with actual TLS metrics from classified connections
- [X] T059 Document crypto classification in `docs/crypto-mode-classification.md`
- [X] T060 Add observability hooks for metrics collection (structured logs + metrics)

### Subphase 10B: Production Security Hardening

- [ ] T062 [Edge Case] Implement external configuration file change detection in `src/admin/handlers.rs` (detect manual file edits, offer UI reload with conflict resolution)
- [ ] T063 [Edge Case - CRITICAL] Implement optimistic locking for concurrent administrator edits in `src/admin/handlers.rs` using ETag-based versioning or timestamp comparison to prevent lost updates
- [ ] T064 [Security - CRITICAL] Implement export sanitization in `src/admin/handlers.rs` to remove/redact sensitive credentials (API keys, private key paths, certificate passphrases) from exported configuration (FR-security-constraint: no secrets in plaintext exports)
- [ ] T065 [Audit Integrity] Implement audit log hash chain verification endpoint GET /api/audit/verify in `src/admin/handlers.rs` to validate tamper-evidence via SHA256 chain validation (FR-192: tamper-evident audit logs)

**Dependencies**:
- Subphase 10A (T055-T060) are sequential for crypto classification
- Subphase 10B tasks (T062-T065) are parallel and can run concurrently with 10A

## Dependency Graph

### Critical Path (Blocking Dependencies)

```
Phase 1 (Setup) → Phase 2 (HTTP Server) → Phase 3 (US1: View Config)
                                         ↓
                                    Phase 4 (US2: Modify Config)
                                         ↓
                                    Phase 5 (US3: Security Safeguards)
                                         ↓
                                    Phase 7 (US5: Audit Trail)
                                         ↓
                                    Phase 8 (UI & Integration)
```

### Parallel Branches

```
Phase 3 (US1: View Config) → Phase 6 (US4: Import/Export)
                                         ↓
                                    Phase 8 (UI & Integration)

Phase 5 (US3: Security Safeguards) → Phase 7 (US5: Audit Trail)
                                         ↓
                                    Phase 8 (UI & Integration)
```

## Parallel Execution Examples

### Setup Phase Parallelism
After T001 creates module structure, the following can run concurrently:
- T002 (add dependencies) || T003 (define types) || T004 (audit types)

### Authentication Parallelism
After T007 integrates server, the following can run concurrently:
- T008 (API key auth) || T009 (RBAC roles)

### View Config Parallelism
After T014 completes hot-reload classification:
- T015 (OperationalStatus type) || T016 (status endpoint)

### Modification Parallelism
After T020 completes hot-reload detection:
- T021 (ConfigChange type) || T022 (diff generation)

### Export Parallelism
Within Phase 6:
- T033 (export handler) || T034 (format parameter)

### UI Phase Parallelism
All UI tasks can run concurrently:
- T046 (HTML creation) || T047 (static serving) || T048 (JS client)

All test tasks can run concurrently:
- T049 (API tests) || T050 (validation tests) || T051 (security tests)

All documentation tasks can run concurrently:
- T052 (README) || T053 (config example)

## Implementation Notes

### Hot-Reload Classification
Based on existing ConfigManager capabilities and safety analysis:
- **Hot-reloadable (safe)**: log_level, buffer_size, connection_timeout, client_cert_mode, certificates (via reload_certificates)
- **Requires restart**: listen (binding address), target (upstream address)

### Security-Affecting Settings
Settings that trigger security warnings:
- Enabling classical TLS fallback
- Disabling crypto mode classification
- Weakening certificate validation (allow_invalid_certificates)
- Disabling client authentication (client_cert_mode: none)
- Enabling passthrough mode (bypasses crypto classification)

### Audit Log Format
JSONL with SHA256 hash chaining:
```json
{"id":"uuid","timestamp":"ISO8601","operator":"username","role":"admin","action":"config_change","setting":"log_level","before":"info","after":"debug","applied":true,"hash":"sha256(prev_hash + current_entry)"}
```

### Constitution Compliance
- **Principle I (Security)**: All security changes require explicit warnings and confirmation
- **Principle III (No Silent Downgrade)**: Security warnings mandatory for downgrade actions
- **Principle VI (Observability)**: All changes logged with audit trail
- **Principle VII (Test-Proven)**: Integration tests required for all critical paths
- **Principle VIII (No Overengineering)**: Using existing ConfigManager, no new frameworks

## Success Criteria Mapping

Each user story maps to specific success criteria from spec.md:

- **US1 (View Config)**: Satisfies SC-001 (view in <5s), SC-007 (10 concurrent users)
- **US2 (Modify Config)**: Satisfies SC-002 (modify in <30s), SC-008 (validation <2s), SC-009 (rollback <10s)
- **US3 (Security Safeguards)**: Satisfies SC-003 (100% warnings), SC-006 (zero silent downgrades)
- **US4 (Import/Export)**: Satisfies SC-005 (reject invalid configs)
- **US5 (Audit Trail)**: Satisfies SC-004 (all changes recorded)

Overall: SC-010 (90% self-service success rate) requires completing all phases through Phase 8.

## Estimated Task Count by Phase

- Phase 1 (Setup): 5 tasks [X] COMPLETE
- Phase 2 (HTTP/Auth): 6 tasks [X] COMPLETE
- Phase 3 (US1): 7 tasks [X] COMPLETE
- Phase 4 (US2): 7 tasks [X] COMPLETE
- Phase 5 (US3): 8 tasks [X] COMPLETE
- Phase 6 (US4): 7 tasks [X] COMPLETE
- Phase 7 (US5): 6 tasks [X] COMPLETE
- Phase 8 (UI): 8 tasks [X] COMPLETE
- Phase 9 (Docs): 1 task [X] COMPLETE
- Phase 10 (Constitution & Hardening): 10 tasks [ ] PENDING (T055-T060: Crypto Classification, T062-T065: Security Hardening)

**Total**: 65 tasks - 55 completed, 10 remaining in Phase 10

## Current Status

**Phases 1-9 Implementation Complete** as of 2025-12-30:
- ✅ All HTTP endpoints functional (12 REST endpoints)
- ✅ Authentication and RBAC working (Bearer token + 3 roles)
- ✅ Configuration read/write with validation
- ✅ Security warnings and confirmation flow
- ✅ Import/export with preview (JSON/YAML)
- ✅ Audit logging with hash chaining (SHA256, 90-day rotation)
- ✅ Audit query and export endpoints
- ✅ Full JavaScript client implementation (700+ lines)
- ✅ Integration test framework (3 test files, 60+ test cases)
- ✅ Comprehensive documentation (README, config example, trust boundaries)

**Phase 10 Status (as of 2025-12-30)**:

**Subphase 10A - Constitution Compliance (MANDATORY)** - ✅ COMPLETE:
- ✅ T055-T060: Crypto mode classification and telemetry implemented
- ✅ Constitution Principle IV fully satisfied
- ✅ Telemetry emitting for all TLS connections
- ✅ Integration tests created
- ✅ Comprehensive documentation written

**Subphase 10B - Production Security Hardening** - ⏸️ PENDING:
- ⏸️ T062: External config file change detection (Medium priority)
- ⏸️ T063: Concurrent administrator edit protection (CRITICAL - optimistic locking)
- ⏸️ T064: Export sanitization (CRITICAL - remove secrets from exported configs)
- ⏸️ T065: Audit log verification endpoint (High priority - validate hash chain integrity)

**Production Readiness**:
- **Constitutional Compliance**: ✅ Achieved (Principle IV implemented)
- **Production Deployment**: ⚠️ Requires T063 and T064 completion (estimated 9-12 hours)
- **Full Feature Complete**: ⏸️ Requires all Subphase 10B tasks (estimated 16-22 hours)

**See**: `docs/phase-10-implementation-notes.md` for detailed implementation guidance
