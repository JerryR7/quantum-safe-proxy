# Cryptographic Mode Classification

**Constitution Principle IV Implementation**

## Overview

Quantum Safe Proxy automatically classifies every TLS connection's cryptographic mode according to **Constitution Principle IV: Cryptographic Mode Classification (MANDATORY)**. This classification is deterministic, explainable, and observable through structured telemetry.

## Classification Modes

### Classical TLS
**Mode**: `classical`

Standard non-PQC TLS connections using traditional cryptographic algorithms:
- ECDHE (Elliptic Curve Diffie-Hellman Ephemeral)
- RSA (Rivest-Shamir-Adleman)
- DHE (Diffie-Hellman Ephemeral)
- Traditional cipher suites (AES, ChaCha20, etc.)

**Example Ciphers**:
- `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`
- `TLS_AES_256_GCM_SHA384`
- `TLS_CHACHA20_POLY1305_SHA256`

### Hybrid TLS
**Mode**: `hybrid`

Post-quantum + classical hybrid TLS connections combining:
- Classical key exchange (X25519, P256, P384, P521, ECDHE)
- Post-quantum key encapsulation (MLKEM, KYBER)

This provides defense-in-depth: security holds if either the classical OR post-quantum component remains secure.

**Example Ciphers**:
- `TLS_AES_256_GCM_SHA384` with X25519MLKEM768 key exchange
- `TLS_AES_128_GCM_SHA256` with P256MLKEM768 key exchange
- Any cipher containing both `MLKEM`/`KYBER` and `X25519`/`P256`/`ECDHE`

### PQC-Only TLS
**Mode**: `pqc`

Pure post-quantum TLS connections (future support):
- Only post-quantum cryptographic algorithms
- No classical fallback components

**Example Ciphers** (theoretical):
- Ciphers containing `MLKEM` or `KYBER` without classical components
- Currently not widely supported by TLS stacks

## Implementation

### Classification Logic

Located in `src/proxy/handler.rs`, the `classify_crypto_mode()` function inspects the negotiated cipher suite immediately after successful TLS handshake:

```rust
fn classify_crypto_mode(ssl: &openssl::ssl::SslRef) -> CryptoMode {
    let cipher_name = ssl.current_cipher()
        .map(|c| c.name())
        .unwrap_or("UNKNOWN");

    // Check for PQC algorithms (MLKEM, KYBER)
    let has_pqc = cipher_name.contains("MLKEM") || cipher_name.contains("KYBER");

    // Check for classical key exchange (X25519, P256, ECDHE)
    let has_classical = cipher_name.contains("X25519")
        || cipher_name.contains("P256")
        || cipher_name.contains("P384")
        || cipher_name.contains("P521")
        || cipher_name.contains("ECDHE");

    if has_pqc {
        if has_classical {
            CryptoMode::Hybrid  // Both PQC and classical
        } else {
            CryptoMode::Pqc     // PQC only
        }
    } else {
        CryptoMode::Classical   // No PQC detected
    }
}
```

### When Classification Occurs

Classification happens **immediately after successful TLS handshake** in the connection handler pipeline:

1. Client initiates TLS handshake
2. Proxy performs TLS handshake with client
3. **Handshake succeeds** ✓
4. **Classification occurs** → `crypto_mode` determined
5. Telemetry emitted
6. Connection forwarding begins

If handshake fails, no classification occurs (connection rejected before crypto mode can be determined).

### Determinism and Explainability

The classification is:
- **Deterministic**: Same cipher always produces same classification
- **Explainable**: Based on inspectable cipher suite name from OpenSSL
- **Testable**: Unit tests verify classification logic for known cipher patterns

Classification evidence is obtained during handshake via OpenSSL's `SSL_get_current_cipher()` function.

## Telemetry and Observability

### Structured Logging

Every successful TLS connection emits telemetry:

```
INFO: Established secure connection | crypto_mode=Hybrid tls_version=TLSv1.3 cipher=TLS_AES_256_GCM_SHA384
```

Debug-level logging provides additional structured fields:

```
DEBUG: security.crypto_mode=Hybrid security.tls.version=TLSv1.3 security.cipher=TLS_AES_256_GCM_SHA384 security.handshake.result=success
```

### Handshake Failures

Failed TLS handshakes emit:

```
ERROR: TLS handshake failed: <error details>
ERROR: security.handshake.result=failure security.handshake.error=<error>
```

### Required Telemetry Fields

Per Constitution Principle IV.4, all connections emit:
- `security.crypto_mode` - Classification result (classical/hybrid/pqc)
- `security.tls.version` - Negotiated TLS version (e.g., TLSv1.3)
- `security.cipher` - Cipher suite name
- `security.handshake.result` - Success or failure

## Metrics Collection

TLS mode statistics are tracked in `TlsModeStats`:

```rust
pub struct TlsModeStats {
    pub classical_count: u64,
    pub hybrid_count: u64,
    pub pqc_count: u64,
    pub last_updated: DateTime<Utc>,
}
```

**Current Implementation**: Metrics are observable via structured logs. Real-time aggregation requires a metrics backend (e.g., Prometheus, StatsD) to parse and aggregate log events.

**Admin API Visibility**: The `/api/status` endpoint returns `OperationalStatus` including `TlsModeStats`. Connect a metrics collector to the proxy's structured logs to populate these statistics.

## Policy Integration

Cryptographic mode classification drives security policy:

### Allow/Deny Policies (Future)
- Allow only hybrid/PQC connections
- Block classical-only TLS
- Route based on crypto mode

### Alerting (Future)
- Alert when classical TLS detected
- Monitor hybrid adoption rate
- Detect unexpected downgrades

### Compliance Reporting
- Audit trail includes crypto mode for all connections
- Compliance teams can verify PQC adoption
- Generate reports from structured logs

## Testing

### Unit Tests

Located in `tests/integration/crypto_classification.rs`:
- `test_cipher_name_parsing()` - Verifies classification logic for known cipher patterns
- `test_classical_tls_classification()` - Integration test for classical connections
- `test_hybrid_tls_classification()` - Integration test for hybrid connections
- `test_handshake_failure_telemetry()` - Verifies failure telemetry
- `test_telemetry_completeness()` - Ensures all required fields present

### Test Cipher Suites

| Cipher Pattern | Expected Mode | Description |
|----------------|---------------|-------------|
| `TLS_AES_256_GCM_SHA384` | Classical | Standard AES cipher |
| `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256` | Classical | ECDHE-RSA |
| `TLS_AES_256_GCM_SHA384_X25519MLKEM768` | Hybrid | X25519 + MLKEM768 |
| `TLS_AES_128_GCM_SHA256_P256MLKEM768` | Hybrid | P256 + MLKEM768 |
| `TLS_KYBER_AES_256_GCM_SHA384` | Hybrid | KYBER variant |

## Failure Modes

### Insufficient Evidence
If cipher name cannot be determined:
- Cipher name defaults to "UNKNOWN"
- Classified as `Classical` (fail-safe default)
- Error logged for investigation

### Handshake Failure
If TLS handshake fails:
- No classification occurs (connection rejected before mode can be determined)
- Failure telemetry emitted: `security.handshake.result=failure`
- Connection terminated

### Policy Enforcement (Future)
If policy blocks a crypto mode:
- Classification occurs first
- Policy decision based on classification
- Connection rejected with clear error message

## Configuration

Crypto mode classification is **always enabled** and cannot be disabled. This is a constitutional requirement (Principle IV - MANDATORY).

The only exception is **passthrough mode**, where the proxy bypasses all TLS inspection and forwards raw TCP. In passthrough mode:
- No TLS handshake occurs at proxy
- No classification possible
- Telemetry indicates passthrough mode active

## References

- **Constitution**: `.specify/memory/constitution.md` - Principle IV
- **Implementation**: `src/proxy/handler.rs` - `classify_crypto_mode()` function
- **Types**: `src/admin/types.rs` - `CryptoMode` enum, `TlsModeStats` struct
- **Tests**: `tests/integration/crypto_classification.rs`
- **Plan**: `specs/001-web-settings-ui/plan.md` - Classification logic specification

## Changelog

- **2025-12-30**: Initial implementation (Phase 10A, Tasks T055-T060)
  - Crypto mode classification added to connection handler
  - Telemetry emission for all connections
  - Integration tests created
  - Documentation complete
