<!--
SYNC IMPACT REPORT
==================
Version Change: 1.0.0 → 1.2.1
Action: RESTORE to version 1.2.1 (comprehensive security governance)

Change Type: MAJOR RESTORATION - Reinstating comprehensive governance model

Principles Restored (1.2.1):
- I. Security Is Non-Negotiable (expanded from Security-First Development)
- II. Explicit Trust Boundaries (NEW)
- III. No Silent Downgrade (NON-NEGOTIABLE) (NEW)
- IV. Cryptographic Mode Classification (MANDATORY) (NEW)
- V. Policy-Driven Behavior Only (NEW)
- VI. Observability Is a Feature (enhanced from Observable Systems)
- VII. Test-Proven Security (enhanced from Test-Driven Development)
- VIII. No Overengineering (MANDATORY DISCIPLINE) (NEW)

Principles Replaced:
- II. Backward Compatibility → removed (replaced by explicit policy-driven behavior in Principle V)
- V. Configuration Flexibility → integrated into "Configuration & Operational Discipline" section
- VI. Documentation as Code → integrated into Development Workflow

Sections Restored:
- Security & Cryptography Constraints (expanded)
- Architecture & Design Constraints (NEW)
- Configuration & Operational Discipline (NEW)
- Failure Models & Resilience Rules (NEW)
- Performance & Resource Constraints (NEW)
- Code Health & Technical Debt Policy (NEW)
- AI-Assisted Development Rules (NEW)
- Release & Compatibility Rules (NEW)
- Definition of Success (NEW)

Breaking Changes Restored:
⚠️ Principle IV (Cryptographic Mode Classification) now MANDATORY - requires implementation
⚠️ Principle III (No Silent Downgrade) now NON-NEGOTIABLE - affects all TLS/PQC handling
⚠️ Principle VIII (No Overengineering) introduces strict complexity gates
⚠️ All resilience mechanisms must be policy-governed (Principle V)

Templates Status:
✅ plan-template.md - Constitution Check section validates new principles
✅ spec-template.md - Requirements structure supports new security constraints
✅ tasks-template.md - Task organization supports test-first and complexity discipline
⚠️ Templates may need updates for:
   - Cryptographic mode classification validation steps
   - Trust boundary documentation requirements
   - Anti-overengineering complexity justification sections

Follow-up Actions Required:
1. Implement cryptographic mode classification logic (Principle IV)
2. Add telemetry for crypto mode detection (security.crypto_mode, security.tls.version)
3. Document trust boundaries for Client↔Proxy, Proxy↔Upstream, Control↔Data planes
4. Review existing code for overengineering violations (Principle VIII)
5. Add integration tests for classical/hybrid/PQC classification scenarios
6. Update CI/CD to enforce constitutional compliance checks

Migration Impact:
- Existing features must add crypto mode classification or document exemption
- All configuration must document security impact (Configuration & Operational Discipline)
- All abstractions must justify "two consumers" rule or be removed (Principle VIII)
- Silent downgrades must be made explicit or removed (Principle III)

Version History:
- v1.0.0 (2025-12-30): Initial constitution with 6 basic principles
- v1.2.1 (2025-01-01): Major expansion - RESTORED (this version)
- v1.2.2 (2025-12-30): "Never over design" clarification (superseded)
- v1.0.0 (2025-12-30): Rollback (superseded)
- v1.2.1 (2025-12-30): RESTORED to comprehensive governance
-->

# Quantum Safe Proxy Constitution

## Core Principles

### I. Security Is Non-Negotiable

Security is the primary and overriding concern of Quantum Safe Proxy (QSP).

**Rules**:
- No feature, optimization, compatibility layer, or operational convenience may reduce security guarantees
- Security-related behavior MUST be explicit, auditable, and testable
- Silent security downgrade is strictly forbidden
- When uncertainty exists, the system MUST fail secure rather than allow ambiguous behavior
- Security correctness takes precedence over performance, backward compatibility, and developer convenience

**Rationale**: As a cryptographic security proxy, any security weakness undermines the entire purpose of the project. Post-quantum cryptography protection is meaningless if classical security is compromised. Security failures in cryptographic systems can be silent and catastrophic.

### II. Explicit Trust Boundaries

All trust boundaries in QSP must be explicitly defined, enforced, and testable.

**Rules**:
- The system MUST clearly distinguish and document:
  - Client ↔ Proxy
  - Proxy ↔ Upstream Service
  - Control Plane ↔ Data Plane
- For each boundary, the following MUST be defined:
  - Authentication mechanism
  - Certificate and key ownership
  - Validation responsibility
  - Failure behavior
- Implicit trust assumptions are prohibited

**Rationale**: Trust boundaries are the seams where security breaks. Undefined boundaries create ambiguity about who validates what, leading to attacks that exploit the gaps between components. Explicit boundaries enable security reasoning and testing.

### III. No Silent Downgrade (NON-NEGOTIABLE)

Any downgrade involving TLS, PQC, or Hybrid cryptographic modes must be explicit, observable, and controllable.

**Rules**:
- All downgrades MUST:
  - Be explicitly configured
  - Be observable via logs, metrics, or events
  - Be explicitly disableable
- Downgrade due to compatibility, performance, or operational convenience without explicit user consent is forbidden

**Rationale**: Silent downgrades are the primary attack vector against hybrid cryptographic systems. An attacker who can force downgrade to classical algorithms defeats the entire purpose of PQC deployment. Observability and control are security requirements, not nice-to-haves.

### IV. Cryptographic Mode Classification (MANDATORY)

Quantum Safe Proxy MUST automatically determine and classify the cryptographic mode of every TLS session at the proxy boundary.

#### IV.1 Required Classification Outcomes

Each TLS session MUST be classified as exactly one of:
- `classical_tls` — Classical (non-PQC) TLS
- `hybrid` — Hybrid TLS using classical + PQC mechanisms
- `pqc` — PQC-only TLS (if supported by the underlying ecosystem)

Manual tagging or operator-provided classification is not permitted.

#### IV.2 Deterministic and Explainable Classification

Classification MUST be based on deterministic, inspectable evidence obtained during handshake or session establishment, including:
- Negotiated TLS protocol version
- Cipher suite and key exchange / key share information
- Certificate chain signature algorithms
- Presence or absence of PQC-related primitives as exposed by the TLS stack / crypto provider

If sufficient evidence cannot be obtained, the session MUST fail explicitly rather than be misclassified.

#### IV.3 Classification Drives Policy

Cryptographic mode classification MUST be consumable by policy logic. Policies may:
- Allow or deny connections
- Route traffic differently
- Trigger alerts or enforcement actions

No implicit downgrade or fallback is permitted without explicit policy authorization.

#### IV.4 Classification Must Be Observable

The classification result MUST be emitted via structured telemetry and/or metrics, including at minimum:
- `security.crypto_mode`
- `security.tls.version`
- `security.handshake.result`
- `security.downgrade.occurred` (boolean)

Classification telemetry is considered part of the security contract.

#### IV.5 Classification Is Test-Critical

Integration tests MUST verify correct classification behavior for:
- Classical TLS handshakes
- Hybrid TLS handshakes (as implemented by the chosen crypto stack)
- Failure cases (unsupported mode, misconfiguration, negotiation failure)

Untested classification logic is considered invalid.

**Rationale**: Without automatic classification, operators cannot verify PQC adoption progress, detect downgrade attacks, or enforce policy based on cryptographic strength. Classification is the foundation for observable, policy-driven security.

### V. Policy-Driven Behavior Only

All security decisions must be driven by explicit policy or configuration, not hardcoded logic.

**Rules**:
- The following MUST be policy-driven, not hardcoded:
  - Cryptographic mode selection
  - Downgrade and fallback behavior
  - Retry and resilience behavior
  - Protocol negotiation
- Hardcoded environment assumptions are prohibited in production code paths

**Rationale**: Hardcoded security decisions cannot adapt to evolving threats, organizational requirements, or deployment contexts. Policy-driven behavior enables security teams to adjust protection without code changes.

### VI. Observability Is a Feature

Observability is a first-class feature of QSP.

**Rules**:
- Any new behavior affecting security, protocol handling, or routing MUST expose at least one of:
  - Structured logs
  - Metrics
  - Explicit security events
- A feature without observability is considered incomplete
- Error messages MUST be actionable: include context and suggest resolution steps

**Rationale**: Security systems require comprehensive visibility for debugging, auditing, and incident response. Post-quantum algorithm adoption needs detailed telemetry to track negotiation success rates and identify compatibility issues. Unobservable security is unverifiable security.

### VII. Test-Proven Security

All security-relevant behavior must be verified through tests.

**Rules**:
- Minimum test requirements:
  - Unit tests for logic correctness
  - Integration tests for real protocol behavior
  - Explicit failure scenario testing
- Tests MUST be written before implementation (Red-Green-Refactor)
- All critical paths MUST have integration tests: TLS handshake flows, certificate validation, protocol detection
- Contract tests MUST verify OpenSSL integration and cryptographic operations
- Untested security behavior is considered nonexistent

**Rationale**: In cryptographic systems, undetected bugs create silent security failures. TDD ensures observable failures rather than silent vulnerabilities. The high cost of security incidents justifies the upfront testing investment. Tests are executable specifications of security requirements.

### VIII. No Overengineering (MANDATORY DISCIPLINE)

Quantum Safe Proxy MUST NOT introduce complexity without proven necessity.

**Rules**:
- No abstraction without at least TWO proven consumers or a documented near-term need
- No speculative extensibility ("future-proofing") without:
  - A concrete use case
  - A defined owner
  - An explicit removal or revision criterion
- No generic framework, engine, or meta-layer unless:
  - Simpler alternatives have been evaluated and rejected
  - The added complexity is justified by measurable benefit
- No configuration option unless it:
  - Changes observable behavior
  - Has a documented security or operational impact
  - Is actively used

**Rationale**: Overengineering is treated as a security and maintainability risk, not as a sign of robustness. Complex systems have larger attack surfaces, more hidden failure modes, and higher cognitive load for security review. Simplicity is not a lack of rigor. Simplicity is a prerequisite for security.

## Security & Cryptography Constraints

**Cryptographic Standards**:
- Cryptographic keys MUST never be shared across roles
- Certificate lifecycle MUST be explicit and traceable
- Cryptographic dependencies MUST be replaceable
- Hybrid and PQC algorithms MUST be treated as evolvable, not permanent
- Debug or development shortcuts MUST NOT enter production code paths
- TLS crypto-mode classification is MANDATORY unless the proxy is explicitly configured in passthrough mode
- Security mechanisms MUST prefer explicit logic over generalized frameworks

**Post-Quantum Support**:
- MUST support NIST-standardized PQC (ML-KEM, ML-DSA) via OpenSSL 3.5+
- MUST support X.509 certificates with combined classical + PQC signatures
- MUST support hybrid key agreement (X25519MLKEM768, P256MLKEM768, P384MLKEM1024)
- MUST allow algorithm selection via configuration without code changes

**Security Boundaries**:
- TLS Termination: Proxy MUST decrypt external TLS and forward plaintext to backend over localhost only
- Certificate Trust: CA certificates MUST be explicitly configured; system trust stores SHOULD NOT be used by default
- Access Control: Client certificate validation mode (`required`, `optional`, `none`) MUST be configurable
- Protocol Enforcement: Non-TLS connections MUST be rejected before any data processing

## Architecture & Design Constraints

**Component Independence**:
- Components MUST follow single-responsibility principles
- Core modules MUST remain logically independent:
  - Listener
  - Handshake
  - Crypto
  - Routing
  - Policy
  - Telemetry
- Abstractions MUST remain shallow and purpose-driven
- Hidden coupling and speculative layers are prohibited

**State Management**:
- Data Plane MUST remain minimally stateful
- Any unavoidable state MUST define:
  - Ownership
  - Lifetime
  - Upper bounds
  - Cleanup strategy

## Configuration & Operational Discipline

**Configuration as Contract**:
- Configuration is treated as a contract
- Every configuration option MUST:
  - Have a clear owner module
  - Define behavioral impact
  - Document security risk
- Unused or redundant configuration options are prohibited
- Configuration priority MUST be: CLI arguments > Environment variables > Config file > Defaults

**Default Configuration**:
- Default configuration MUST:
  - Be secure
  - Be executable
  - Fail explicitly if unsafe
- Configuration sprawl is treated as technical debt and a security risk
- All configuration changes except listen address MUST support hot reload (SIGHUP on Unix, automatic polling on Windows)
- Configuration validation MUST occur at startup and reload with clear error messages

## Failure Models & Resilience Rules

**Explicit Failure Handling**:
- Failures MUST be explicit and observable
- Generic or ambiguous error handling is prohibited
- Retry, fallback, and circuit-breaking mechanisms MUST NOT:
  - Bypass security checks
  - Implicitly change cryptographic mode
- All resilience behavior MUST be policy-governed
- Resilience logic MUST be simpler than the failure it mitigates

## Performance & Resource Constraints

**Performance Discipline**:
- Performance optimizations MUST NOT reduce auditability or explainability
- All performance shortcuts MUST be explicit and disableable
- Resource usage MUST be predictable
- Unbounded memory, state, or resource growth is prohibited
- Performance optimizations MUST NOT introduce architectural indirection without necessity

## Code Health & Technical Debt Policy

**Code Hygiene**:
- Dead code is treated as a security risk
- Unused feature flags, configuration options, and legacy paths MUST be periodically removed
- Refactoring MUST be accompanied by behavioral proof
- Refactoring without tests is considered incomplete
- Complexity reduction is considered a valid and valuable refactoring goal

## AI-Assisted Development Rules

**AI Governance**:
- AI is an executor, not a decision-maker
- All AI-generated code MUST be reviewable, testable, and replaceable
- Security decisions MUST be validated via specification and tests
- The Spec → Plan → Tasks → Implement flow is MANDATORY
- Skipping steps is a process violation

## Development Workflow

**Feature Development Lifecycle**:
1. **Specification**: Define requirements in `/specs/[###-feature-name]/spec.md` using `/speckit.specify`
2. **Planning**: Create implementation plan with `/speckit.plan` including constitution compliance check
3. **Task Breakdown**: Generate actionable tasks with `/speckit.tasks` organized by user story
4. **Implementation**: Follow TDD - write failing tests, implement feature, verify tests pass
5. **Validation**: Run full test suite, verify constitution compliance, update documentation

**Code Review Requirements**:
- All TLS, certificate, and cryptographic code MUST have security-focused review
- Breaking changes MUST be flagged, versioned, and documented with migration paths
- New features MUST include passing tests demonstrating functionality
- User-facing changes MUST update README, config examples, and help text

**Complexity Justification**:
- New dependencies MUST justify why existing dependencies (tokio, openssl) are insufficient
- Abstractions MUST justify why direct implementation is insufficient
- Third-party Rust crates MUST be evaluated for security, maintenance status, and necessity

## Release & Compatibility Rules

**Breaking Changes**:
- All security-affecting changes MUST be documented in release notes
- Breaking changes require:
  - Explicit justification
  - Migration guidance
  - Impact explanation
- Backward compatibility is an explicit decision, not a default

**Versioning**:
- Follow semantic versioning strictly
- MAJOR: Breaking governance/security changes
- MINOR: New principles, expanded guidance
- PATCH: Clarifications, non-semantic improvements

## Definition of Success

Quantum Safe Proxy is considered successful when:
- Every cryptographic and security decision can be clearly explained
- TLS sessions are correctly classified as classical / hybrid / PQC
- Behavior under failure and stress is predictable
- The system remains trustworthy as cryptographic standards evolve
- Security can be verified without reading implementation code

## Governance

**Amendment Procedure**:
1. **Proposal**: Document proposed changes with rationale and impact analysis
2. **Review**: Assess impact on existing features, tests, and documentation
3. **Approval**: Requires explicit acknowledgment of breaking changes and migration plan
4. **Migration**: Update all dependent templates (spec, plan, tasks, commands) for consistency
5. **Communication**: Document changes in constitution Sync Impact Report

**Versioning Policy**:
- MAJOR: Backward-incompatible governance changes (removing principles, changing enforcement)
- MINOR: New principles added, expanded guidance, new mandatory sections
- PATCH: Clarifications, wording improvements, typo fixes, non-semantic changes

**Compliance Verification**:
- This constitution supersedes all informal practices
- All feature plans MUST include "Constitution Check" section verifying principle adherence
- All pull requests MUST be reviewed for constitutional compliance
- Security violations (Principle I) MUST block implementation until resolved
- Violations MUST be explicit and documented as exceptions
- Exceptions require:
  - Justification
  - Risk analysis
  - Named owner
  - Removal criteria

**Runtime Development Guidance**:
- For detailed implementation guidance aligned with these principles, refer to:
  - Feature specifications: `/specs/[###-feature-name]/spec.md`
  - Implementation plans: `/specs/[###-feature-name]/plan.md`
  - Task breakdowns: `/specs/[###-feature-name]/tasks.md`

**Version**: 1.2.1 | **Ratified**: 2025-01-01 | **Last Amended**: 2025-12-30
