# Admin API Trust Boundaries

**Feature**: Web-Based Settings Management UI
**Version**: 1.0
**Last Updated**: 2025-12-30

## Overview

The Quantum Safe Proxy Admin API introduces three distinct trust boundaries that operators must understand to deploy the feature securely. This document defines these boundaries, their security implications, and deployment recommendations.

## Trust Boundary 1: Network Exposure

### Definition

**Network Exposure** refers to which network interfaces and clients have access to the admin API endpoints.

### Security Model

- **Default Binding**: `127.0.0.1:8443` (localhost only)
- **Recommended**: Keep admin API accessible only from localhost or internal management network
- **Warning**: Exposing to public internet without additional protection creates significant risk

### Attack Surface

| Exposure Level | Attack Surface | Recommended Use |
|----------------|----------------|-----------------|
| Localhost (`127.0.0.1`) | Minimal - only local processes | ✅ Development, single-server deployments |
| Internal Network (`10.0.0.0/8`) | Medium - trusted network only | ⚠️ Enterprise with network segmentation |
| Public Internet (`0.0.0.0`) | **HIGH RISK** | ❌ Never without reverse proxy + TLS |

### Mitigation Strategies

#### Development/Testing
```bash
# Bind to localhost only
export ADMIN_API_ADDR="127.0.0.1:8443"
```

#### Production Deployment
```bash
# Option 1: Localhost + SSH tunnel
export ADMIN_API_ADDR="127.0.0.1:8443"
# Access via: ssh -L 8443:localhost:8443 user@proxy-server

# Option 2: Internal network with firewall rules
export ADMIN_API_ADDR="10.0.1.100:8443"
# Firewall: Only allow connections from management subnet
```

#### Never Do This (Public Exposure)
```bash
# ❌ INSECURE: Exposes admin API to entire internet
export ADMIN_API_ADDR="0.0.0.0:8443"
```

### Recommendations

1. **Always bind to localhost** unless you have a specific need and proper security controls
2. **Use SSH tunneling** for remote administration instead of network exposure
3. **Deploy reverse proxy with TLS** if admin API must be network-accessible
4. **Implement IP allowlisting** at firewall level for internal network deployments
5. **Monitor failed authentication attempts** as indicators of unauthorized access attempts

## Trust Boundary 2: Authentication and Authorization

### Definition

**Authentication and Authorization** determine who can access admin API endpoints and what actions they can perform.

### Authentication Mechanism

- **Method**: Bearer token (API keys)
- **Storage**: Environment variable or secure configuration
- **Transmission**: HTTP `Authorization` header
- **Validation**: Constant-time comparison to prevent timing attacks

### Authorization Roles

| Role | Permissions | Use Case |
|------|-------------|----------|
| **Viewer** | Read-only access to config and status | Monitoring tools, read-only dashboards |
| **Operator** | Modify non-security settings | Day-to-day operational changes |
| **Admin** | Full access including security settings | Security administrators, initial setup |

### Permission Matrix

| Action | Viewer | Operator | Admin |
|--------|:------:|:--------:|:-----:|
| View configuration | ✅ | ✅ | ✅ |
| View operational status | ✅ | ✅ | ✅ |
| View audit log | ✅ | ✅ | ✅ |
| Modify log_level | ❌ | ✅ | ✅ |
| Modify buffer_size | ❌ | ✅ | ✅ |
| Modify connection_timeout | ❌ | ✅ | ✅ |
| Enable classical fallback | ❌ | ❌ | ✅ |
| Disable crypto classification | ❌ | ❌ | ✅ |
| Weaken cert validation | ❌ | ❌ | ✅ |
| Export configuration | ❌ | ✅ | ✅ |
| Import configuration | ❌ | ❌ | ✅ |
| Rollback configuration | ❌ | ❌ | ✅ |
| Export audit log | ❌ | ❌ | ✅ |

### API Key Management

#### Generating Secure Keys

```bash
# Generate cryptographically secure API key
openssl rand -base64 32

# Example output: vK7s9mP2nQ4xR8wA1bC3dE5fG6hJ7kL8mN9oP0qR2sT=
```

#### Key Format

```bash
# Format: name:key:role
export ADMIN_API_KEYS="admin:vK7s9mP2nQ4xR8wA1bC3dE5fG6hJ7kL8mN9oP0qR2sT=:admin,ops:anotherkey:operator"
```

#### Key Rotation

1. **Generate new key** using `openssl rand -base64 32`
2. **Add new key** to `ADMIN_API_KEYS` alongside existing key
3. **Update clients** to use new key
4. **Remove old key** after all clients migrated
5. **Verify audit log** shows no usage of old key before deletion

### Security Considerations

1. **Never commit API keys to version control**
2. **Use separate keys per operator** for audit trail clarity
3. **Rotate keys quarterly** or after personnel changes
4. **Monitor authentication failures** in audit log
5. **Implement rate limiting** at reverse proxy level to prevent brute force

### Example: Least Privilege Deployment

```bash
# Production setup with role separation
export ADMIN_API_KEYS="
  admin-alice:$(openssl rand -base64 32):admin,
  operator-bob:$(openssl rand -base64 32):operator,
  monitoring:$(openssl rand -base64 32):viewer
"

# Store in secrets manager (e.g., Kubernetes secrets)
kubectl create secret generic qsp-admin-keys \
  --from-literal=admin-alice="..." \
  --from-literal=operator-bob="..." \
  --from-literal=monitoring="..."
```

## Trust Boundary 3: Configuration Persistence and Integrity

### Definition

**Configuration Persistence** refers to where configuration changes are stored and how their integrity is maintained.

### Storage Mechanisms

| Component | Storage Location | Integrity Protection |
|-----------|------------------|---------------------|
| Runtime Config | In-memory (ConfigManager) | Validation on change |
| Audit Log | JSONL file (`/var/log/...`) | SHA256 hash chaining |
| Config Backups | Export files (JSON/YAML) | Operator responsibility |

### Audit Log Tamper Evidence

The audit log uses cryptographic hash chaining to provide tamper evidence:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-30T10:30:00Z",
  "operator": "admin-user",
  "prev_hash": "abc123def456...",
  "hash": "789ghi012jkl..."
}
```

**Hash Calculation**: `SHA256(prev_hash || json_entry)`

### Verification

```bash
# Verify audit log integrity (planned feature)
quantum-safe-proxy verify-audit-log /var/log/quantum-safe-proxy/admin-audit.jsonl
```

If any entry in the chain is modified, the hash verification will fail, providing evidence of tampering.

### File System Permissions

Recommended permissions for audit log:

```bash
# Create log directory with restricted permissions
sudo mkdir -p /var/log/quantum-safe-proxy
sudo chown qsp-user:qsp-group /var/log/quantum-safe-proxy
sudo chmod 750 /var/log/quantum-safe-proxy

# Audit log should be writable only by proxy process
sudo touch /var/log/quantum-safe-proxy/admin-audit.jsonl
sudo chown qsp-user:qsp-group /var/log/quantum-safe-proxy/admin-audit.jsonl
sudo chmod 640 /var/log/quantum-safe-proxy/admin-audit.jsonl
```

### Backup and Retention

1. **Audit Log Rotation**: 90-day retention (automatic)
2. **Archived Logs**: Moved to `.archive` files with timestamp
3. **Configuration Exports**: Operator-managed backups
4. **Disaster Recovery**: Store exported configs in version control (without secrets)

### Configuration Precedence

Understanding precedence prevents unexpected behavior:

```
CLI Arguments > Environment Variables > UI Changes > Config File > Defaults
```

**Example Scenario**:
- Config file sets `log_level: info`
- UI changes to `log_level: debug` (applied)
- Environment variable `LOG_LEVEL=warn` **overrides** UI change
- CLI argument `--log-level error` **overrides** everything

### Recommendations

1. **Monitor audit log file** with log aggregation tools (e.g., Splunk, ELK)
2. **Alert on failed validation** attempts or rejected changes
3. **Export configuration regularly** for disaster recovery
4. **Verify hash chain integrity** periodically (manual or automated)
5. **Implement file integrity monitoring** (FIM) on audit log files

## Cross-Boundary Attack Scenarios

### Scenario 1: Unauthorized Network Access

**Attack**: Attacker gains network access to exposed admin API

**Mitigations**:
1. ✅ Bind to localhost (Trust Boundary 1)
2. ✅ Require API key authentication (Trust Boundary 2)
3. ✅ Log all authentication attempts (Trust Boundary 3)
4. ✅ Rate limiting at reverse proxy
5. ✅ IP allowlisting at firewall

### Scenario 2: Compromised API Key

**Attack**: Operator's API key is leaked or stolen

**Mitigations**:
1. ✅ Role-based permissions limit damage (Trust Boundary 2)
2. ✅ Audit log records all actions (Trust Boundary 3)
3. ✅ Key rotation invalidates compromised key
4. ✅ Security warnings require explicit confirmation
5. ✅ Alert on unusual activity patterns

### Scenario 3: Malicious Configuration Change

**Attack**: Attacker with Admin role disables security features

**Mitigations**:
1. ✅ Security warnings displayed (Trust Boundary 2)
2. ✅ Explicit confirmation required
3. ✅ Audit log records change + confirmation (Trust Boundary 3)
4. ✅ Hash chaining provides tamper evidence
5. ✅ Rollback capability available

### Scenario 4: Audit Log Tampering

**Attack**: Attacker attempts to cover tracks by modifying audit log

**Mitigations**:
1. ✅ SHA256 hash chaining makes tampering evident (Trust Boundary 3)
2. ✅ File system permissions prevent unauthorized writes
3. ✅ File integrity monitoring detects modifications
4. ✅ Log aggregation provides external copy
5. ✅ Verification tool detects broken chains

## Deployment Patterns

### Pattern 1: Single-Server Development

```
[Developer Machine]
    ↓ localhost
[Quantum Safe Proxy + Admin API]
    ↓ localhost
[Backend Service]
```

**Trust Boundaries**:
- Network: Localhost only (✅ Secure)
- Auth: Single admin key (⚠️ Acceptable for dev)
- Persistence: Local file system (✅ Acceptable for dev)

### Pattern 2: Production with SSH Tunnel

```
[Operator Workstation]
    ↓ SSH tunnel
[Production Server]
    ↓ localhost
[Quantum Safe Proxy + Admin API]
    ↓ internal network
[Backend Service]
```

**Trust Boundaries**:
- Network: Localhost + SSH (✅ Secure)
- Auth: Per-operator keys (✅ Secure)
- Persistence: Monitored file system (✅ Secure)

### Pattern 3: Enterprise with Reverse Proxy

```
[Operator] → [VPN] → [Reverse Proxy (TLS)] → [Admin API] → [Backend]
```

**Trust Boundaries**:
- Network: Internal network + TLS reverse proxy (✅ Secure)
- Auth: Per-operator keys + RBAC (✅ Secure)
- Persistence: Log aggregation + FIM (✅ Secure)

### Pattern 4: ❌ Insecure Public Exposure (Never Do This)

```
[Public Internet] → [Admin API on 0.0.0.0:8443] → [Backend]
```

**Trust Boundaries**:
- Network: ❌ Public exposure without TLS
- Auth: ⚠️ API keys transmitted over HTTP
- Persistence: ⚠️ No external audit trail

**Why This Fails**:
1. API keys transmitted in plaintext
2. No protection against eavesdropping
3. Brute force attacks possible
4. No rate limiting
5. DDoS vector

## Compliance Considerations

### Audit Requirements

The admin API audit log satisfies common compliance requirements:

| Requirement | Implementation |
|-------------|----------------|
| **Who** (Identity) | `operator` and `role` fields |
| **What** (Action) | `action` and `changes` fields |
| **When** (Timestamp) | `timestamp` field (ISO 8601) |
| **Why** (Justification) | `confirmation` field |
| **Tamper Evidence** | SHA256 hash chaining |
| **Retention** | 90-day automatic retention |

### Compliance Mappings

- **SOC 2 CC6.2**: Configuration change logging
- **PCI-DSS 10.2**: User activity tracking
- **HIPAA §164.312(b)**: Audit controls
- **ISO 27001 A.12.4**: Logging and monitoring

## Security Checklist

Before deploying admin API to production, verify:

- [ ] Admin API bound to localhost or internal network only
- [ ] API keys generated with cryptographically secure method
- [ ] Separate API keys issued per operator
- [ ] Role-based access control configured appropriately
- [ ] Audit log file permissions set correctly (640)
- [ ] Audit log directory permissions set correctly (750)
- [ ] Log aggregation configured for audit trail
- [ ] File integrity monitoring enabled on audit log
- [ ] Reverse proxy with TLS if network-accessible
- [ ] Firewall rules limit access to management subnet
- [ ] SSH tunnel or VPN required for remote access
- [ ] Key rotation process documented and tested
- [ ] Incident response plan includes compromised key scenario
- [ ] Monitoring alerts configured for auth failures
- [ ] Configuration export backups automated
- [ ] Disaster recovery procedure tested

## References

- [Admin API Documentation](../README.md#admin-api-and-web-based-settings-management)
- [Security Best Practices](../README.md#security-best-practices)
- [Quantum Safe Proxy Constitution](../constitution.md) (if exists)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)

## Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-30 | 1.0 | Initial trust boundary documentation |
