# Feature Specification: Web-Based Settings Management UI

**Feature Branch**: `001-web-settings-ui`
**Created**: 2025-12-30
**Status**: Draft
**Input**: User description: "Create a web-based settings management page for Quantum Safe Proxy that allows administrators to view and modify configuration through a UI instead of manual file editing. The feature should provide visibility into effective configuration, support hot-reload where possible, include audit logging, and maintain security constraints like no silent downgrades. This is an MVP to improve usability of existing configuration, not a complete redesign of the configuration system."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View Effective Configuration (Priority: P1)

As a QSP administrator, I need to view the currently active configuration in a web interface so I can quickly understand the system's current state without parsing configuration files.

**Why this priority**: This is the foundation for all other features. Administrators must be able to see what's currently running before they can confidently make changes. This delivers immediate value by providing visibility and serves as a read-only audit tool.

**Independent Test**: Can be fully tested by accessing the settings page and verifying that all displayed values match the actual running configuration. Delivers value by eliminating the need to SSH into servers and manually inspect config files.

**Acceptance Scenarios**:

1. **Given** the proxy is running with configuration from multiple sources (CLI, env vars, config file), **When** administrator opens the settings page, **Then** the page displays the resolved/effective configuration with each setting's source clearly indicated
2. **Given** the administrator has read-only permissions, **When** they access the settings page, **Then** all settings are displayed but edit controls are disabled
3. **Given** the proxy is operational, **When** the settings page loads, **Then** basic operational status is displayed (TLS mode statistics, recent connection metrics)

---

### User Story 2 - Modify Non-Security Settings (Priority: P2)

As a QSP administrator, I need to modify operational settings (timeouts, buffer sizes, logging levels) through the UI so I can tune performance without editing configuration files.

**Why this priority**: Once visibility is established, the next most valuable capability is modifying low-risk operational settings. These changes improve day-to-day operations without introducing security concerns.

**Independent Test**: Can be fully tested by modifying a timeout setting through the UI, verifying validation, and confirming the change takes effect (with or without restart as appropriate). Delivers value by enabling quick operational adjustments.

**Acceptance Scenarios**:

1. **Given** administrator is on the settings page, **When** they modify a hot-reloadable setting (e.g., log level), **Then** the system validates the input, shows a confirmation dialog indicating it can be applied immediately, and applies the change without restart
2. **Given** administrator modifies a setting requiring restart (e.g., listen address), **When** they click save, **Then** the system clearly indicates restart is required and does not auto-apply the change
3. **Given** administrator enters an invalid value, **When** they attempt to save, **Then** the system displays a clear validation error with acceptable value range

---

### User Story 3 - Modify Security-Critical Settings with Safeguards (Priority: P3)

As a security engineer, I need to modify PQC/TLS/Hybrid mode settings through the UI with explicit warnings and audit logging so I can adjust security posture while maintaining accountability and preventing silent downgrades.

**Why this priority**: Security settings changes are higher risk and require more safeguards. This builds on the modification capability with additional safety mechanisms specifically for security-affecting changes.

**Independent Test**: Can be fully tested by attempting to enable classical TLS fallback, verifying that explicit risk warnings appear, requiring confirmation, and confirming the change is logged in audit trail. Delivers value by making security changes traceable and intentional.

**Acceptance Scenarios**:

1. **Given** administrator attempts to allow classical TLS fallback, **When** they toggle the setting, **Then** a prominent warning displays explaining the security downgrade risk and requires explicit confirmation before proceeding
2. **Given** administrator confirms a security-affecting change, **When** the change is saved, **Then** the audit log records the operator identity, timestamp, before/after values, and confirmation acknowledgment
3. **Given** passthrough mode is enabled (bypassing crypto classification), **When** administrator views settings, **Then** the UI clearly indicates which security requirements are bypassed in this mode

---

### User Story 4 - Export and Import Configuration (Priority: P4)

As an operations engineer, I need to export the current configuration and import configurations for validation so I can maintain configuration-as-code and safely test configuration changes.

**Why this priority**: This enables infrastructure-as-code workflows and disaster recovery. While valuable, it's less urgent than the core read/write functionality.

**Independent Test**: Can be fully tested by exporting current config, modifying it externally, importing for preview, and verifying no auto-apply occurs. Delivers value by supporting GitOps workflows.

**Acceptance Scenarios**:

1. **Given** administrator is on the settings page, **When** they click "Export Configuration", **Then** the system downloads a JSON/YAML file containing the current resolved configuration
2. **Given** administrator uploads a configuration file, **When** import is triggered, **Then** the system validates the file and displays a preview with diff comparing current vs. imported settings, without auto-applying
3. **Given** administrator reviews import preview, **When** they confirm application, **Then** the imported settings are validated against current constraints and applied following the same rules as UI edits (security warnings, restart requirements, etc.)

---

### User Story 5 - View Configuration Audit Trail (Priority: P5)

As a compliance auditor, I need to view a history of configuration changes so I can verify who changed what and when for security compliance purposes.

**Why this priority**: Audit trail is critical for compliance but can be delivered after core modification features are working. It's primarily a reporting/visibility feature.

**Independent Test**: Can be fully tested by making several configuration changes and verifying they all appear in the audit log with correct metadata. Delivers value by providing accountability and forensic capability.

**Acceptance Scenarios**:

1. **Given** multiple configuration changes have been made, **When** administrator accesses the audit log view, **Then** all changes are listed chronologically with operator, timestamp, changed fields, before/after values, and apply status
2. **Given** a security-affecting change was made, **When** viewing the audit entry, **Then** the entry includes the security warning that was displayed and confirmation that was acknowledged
3. **Given** administrator wants to review changes to a specific setting, **When** they filter the audit log by setting name, **Then** only changes to that setting are displayed

---

### Edge Cases

- **External File Modification**: What happens when configuration file is manually edited while UI is open? The UI must detect the external change (via file modification time or content hash) and offer to reload with conflict resolution options
- **Concurrent Administrator Edits**: What happens when two administrators edit settings simultaneously? The system must implement optimistic locking (ETag-based versioning or timestamp comparison) to prevent lost updates; second editor receives conflict error with option to view changes and retry
- **Hot-Reload Failure**: What happens when hot-reload fails? The system must roll back to previous configuration automatically and display error with recovery instructions (validate configuration, check logs, manual restart if needed)
- **Incompatible Import**: What happens when imported configuration contains settings incompatible with current QSP version? The system must reject with clear version compatibility error listing specific incompatible settings
- **Audit Log Storage Full**: What happens when audit log storage fills up? The system must implement rotation/archival (FR-020: 90-day retention) and warn administrators before reaching capacity (90% full threshold)

## Requirements *(mandatory)*

### Functional Requirements

#### Configuration Visibility
- **FR-001**: System MUST display the resolved/effective configuration combining values from all sources (CLI args, environment variables, config file, defaults)
- **FR-002**: System MUST indicate the source of each configuration value (e.g., "from config file", "from environment", "default")
- **FR-003**: System MUST display basic operational status including TLS mode statistics (classical/hybrid/PQC counts) and recent handshake success rates
- **FR-004**: System MUST support read-only mode for users without edit permissions
  - **Initial Setup**: API keys and roles are defined in the main configuration file under `[admin.api_keys]` section
  - **Bootstrap Admin**: At least one admin-role API key must be created manually in the configuration file before first use
  - **Role Assignment**: Each API key is assigned exactly one role (viewer/operator/admin) in the configuration
  - **Key Generation**: Administrators must generate secure random API keys (recommended: 32+ character alphanumeric strings) and add them to config file with role assignment

#### Configuration Modification
- **FR-005**: System MUST allow modification of all configurable proxy settings including: crypto modes (classical/hybrid/PQC), fallback policies, passthrough mode, timeouts, buffer sizes, logging configuration
- **FR-006**: System MUST perform server-side validation of all configuration changes including: type checking, range validation, compatibility validation, security constraint enforcement
- **FR-007**: System MUST clearly distinguish settings that support hot-reload from those requiring restart
- **FR-008**: System MUST NOT auto-apply changes requiring restart without explicit administrator confirmation
- **FR-009**: System MUST NOT auto-restart the proxy service under any circumstances

#### Security Safeguards
- **FR-010**: System MUST display explicit risk warnings for security-degrading changes. The following configuration changes are classified as security-affecting and MUST trigger warnings:
  - Enabling classical TLS fallback (allows downgrade from PQC/hybrid to classical TLS)
  - Disabling crypto mode classification (removes visibility into connection security level)
  - Weakening certificate validation via `allow_invalid_certificates: true`
  - Disabling client authentication by setting `client_cert_mode: none`
  - Enabling passthrough mode (bypasses all crypto classification and security inspection)
  - Any change that reduces TLS version requirements or weakens cipher suite restrictions
- **FR-011**: System MUST require explicit administrator confirmation for all security-affecting changes
- **FR-012**: System MUST enforce the "No Silent Downgrade" principle by making all security reductions visible and opt-in
- **FR-013**: System MUST prohibit configuration changes that violate absolute security constraints defined in the constitution

#### Configuration Persistence
- **FR-014**: System MUST maintain a single source of truth for resolved configuration accessible to both UI and proxy runtime
- **FR-015**: System MUST preserve clear precedence order when UI and config file both exist: CLI arguments > Environment variables > UI changes > Config file > Defaults
- **FR-016**: System MUST support configuration rollback to the previous version
- **FR-017**: System MUST display diff (before/after) for pending changes before applying

#### Audit and Compliance
- **FR-018**: System MUST log all configuration changes including: operator identity, timestamp, changed fields with before/after values, whether change was applied successfully
- **FR-019**: System MUST log all security-affecting changes with additional context: risk warning displayed, confirmation acknowledgment, justification if provided
- **FR-020**: System MUST maintain audit log for minimum 90 days
- **FR-021**: System MUST support audit log export for compliance reporting
- **FR-021a**: System MUST provide audit log integrity verification capability
  - **Hash Chain Verification**: Audit log uses SHA256 hash chaining for tamper detection; system must provide verification endpoint to validate chain integrity
  - **Verification Scope**: Verification must check that each entry's hash correctly incorporates the previous entry's hash
  - **Verification Result**: Return verification status (valid/invalid), total entries checked, and location of first hash mismatch if tampering detected

#### Import/Export
- **FR-022**: System MUST support exporting current configuration in JSON and YAML formats
  - **Export Sanitization**: Exported configuration MUST NOT contain sensitive credentials in plaintext including: API keys, private key file contents, certificate passphrases, authentication tokens
  - **Sanitization Method**: Sensitive fields must be either omitted or replaced with placeholder values (e.g., `<REDACTED>`, `<path/to/file>`)
  - **Export Documentation**: Export must include comments indicating which sensitive fields were sanitized and require manual restoration
- **FR-023**: System MUST support importing configuration files for validation and preview
- **FR-024**: System MUST display diff comparing imported configuration with current configuration
- **FR-025**: System MUST NOT auto-apply imported configuration without explicit administrator confirmation
- **FR-026**: System MUST validate imported configuration against current QSP version compatibility

#### Authentication and Authorization
- **FR-027**: System MUST authenticate all settings API requests
- **FR-028**: System MUST enforce role-based access control with minimum roles: read-only viewer, operator (can modify non-security settings), security admin (can modify all settings)
- **FR-029**: System MUST log all authentication and authorization events

### Key Entities

- **ResolvedConfig**: The single source of truth for runtime configuration, representing the merged result of all configuration sources (CLI, env, file, UI, defaults) with clear precedence order
- **ConfigurationChange**: A record of a configuration modification including the operator, timestamp, modified fields, before/after values, validation results, and application status
- **AuditEntry**: A log entry recording configuration changes and administrative actions, including operator identity, action type, affected configuration, and security context
- **ConfigurationSource**: An enumeration indicating where a configuration value originated (CLI, environment variable, config file, UI override, or default)
- **ValidationResult**: The outcome of configuration validation including any errors, warnings, compatibility issues, or security constraint violations

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Administrators can view complete effective configuration in under 5 seconds
- **SC-002**: Administrators can modify and apply non-security settings in under 30 seconds (compared to several minutes for file editing and reload)
- **SC-003**: 100% of security-degrading configuration changes display explicit warnings before application
- **SC-004**: All configuration changes are recorded in audit log with complete metadata (operator, timestamp, before/after)
- **SC-005**: Configuration import rejects invalid configurations with clear error messages before any changes are applied
- **SC-006**: Zero security downgrades occur without explicit administrator acknowledgment (no silent fallback)
- **SC-007**: System supports at least 10 concurrent administrators viewing/editing settings without conflict
- **SC-008**: Configuration validation completes in under 2 seconds for typical changes
- **SC-009**: Administrators can successfully roll back to previous configuration in under 10 seconds
- **SC-010**: 90% of administrators successfully complete their first configuration change without documentation or support

### Constraints and Assumptions

**Assumptions**:
- The existing configuration model is sound and does not require redesign
- Administrators have basic understanding of TLS/PQC/Hybrid concepts (in-app help text provided for complex security settings to support SC-010: 90% self-service success)
- The proxy already has configuration hot-reload capability for supported settings
- Authentication/authorization infrastructure exists or can be integrated
- Settings UI will be accessed via web browser (not mobile-optimized initially)
- **Browser Compatibility**: UI must support modern browsers: Chrome 90+, Firefox 88+, Safari 14+, Edge 90+ (released 2021 or later with ES2020 support)

**Out of Scope**:
- Redesigning the underlying configuration model or architecture
- Multi-tenant configuration management
- Configuration version control beyond single rollback
- Real-time configuration synchronization across multiple proxy instances
- Advanced configuration templating or inheritance
- Plugin or extension system for custom configuration options
- Mobile-responsive UI (desktop browser only for MVP)
- Configuration recommendation or auto-tuning features

**Security Constraints**:
- All settings API endpoints must require authentication
- Security-affecting changes must be auditable and traceable
- No configuration change may violate constitution principles (especially "No Silent Downgrade")
- Exported configuration files must not contain sensitive secrets in plaintext
- Audit logs must be tamper-evident

**Performance Constraints**:
- Configuration validation must complete in under 2 seconds (SC-008)
- Settings page must load in under 3 seconds on standard network
- Hot-reload of settings must complete in under 5 seconds total time including validation (i.e., validation <2s + reload application <3s = <5s total)
- Audit log queries must return results in under 1 second for typical date ranges (30-day window, <1000 entries)
- System must support at least 10 concurrent administrators without performance degradation (SC-007)
