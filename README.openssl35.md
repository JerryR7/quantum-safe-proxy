# Quantum Safe Proxy with OpenSSL 3.5

This document explains how to use the Quantum Safe Proxy with OpenSSL 3.5, which includes native post-quantum cryptography support.

## Overview

The Quantum Safe Proxy is a TLS termination proxy that supports post-quantum cryptography. It can be used to secure connections between clients and servers using quantum-resistant algorithms.

This version uses OpenSSL 3.5, which includes native support for post-quantum cryptography algorithms like ML-KEM and ML-DSA.

## Development Environment

For development and testing, you can use the `docker-compose.openssl35.yml` file, which automatically generates certificates if they don't exist:

```bash
# Build the image
docker compose -f docker-compose.openssl35.yml build

# Start the services
docker compose -f docker-compose.openssl35.yml up
```

In the development environment:
- `AUTO_GENERATE_CERTS=true` is set, so certificates will be automatically generated if they don't exist
- Traditional ECDSA certificates are used by default
- Client certificates are optional

## Production Environment

For production use, you should pre-generate certificates and use the `docker-compose.openssl35.prod.yml` file:

```bash
# Generate certificates (if needed)
docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh

# Start the services in production mode
docker compose -f docker-compose.openssl35.prod.yml up -d
```

In the production environment:
- `AUTO_GENERATE_CERTS=false` is set, so certificates must be pre-generated
- Hybrid P384_ML-DSA-65 certificates are used by default (stronger security)
- Client certificates are required

## Certificate Types

The certificate generation script creates several types of certificates:

1. **Traditional Certificates**:
   - RSA: `/app/certs/traditional/rsa/`
   - ECDSA: `/app/certs/traditional/ecdsa/`

2. **Hybrid Certificates**:
   - P256_ML-DSA-44: `/app/certs/hybrid/ml-dsa-44/`
   - P384_ML-DSA-65: `/app/certs/hybrid/ml-dsa-65/`
   - P521_ML-DSA-87: `/app/certs/hybrid/ml-dsa-87/`
   - X25519_ML-KEM-768: `/app/certs/hybrid/ml-kem-768/`

3. **Post-Quantum Certificates**:
   - ML-DSA-44: `/app/certs/post-quantum/ml-dsa-44/`
   - ML-DSA-65: `/app/certs/post-quantum/ml-dsa-65/`
   - ML-DSA-87: `/app/certs/post-quantum/ml-dsa-87/`

## Customizing the Configuration

You can customize the configuration by modifying the `command` section in the docker-compose file. For example, to use a different certificate type:

```yaml
command: [
  "--listen", "0.0.0.0:8443",
  "--target", "backend:6000",
  "--cert", "/app/certs/hybrid/ml-dsa-87/server.crt",
  "--key", "/app/certs/hybrid/ml-dsa-87/server.key",
  "--ca-cert", "/app/certs/hybrid/ml-dsa-87/ca.crt",
  "--log-level", "info",
  "--client-cert-mode", "required"
]
```

## Manually Generating Certificates

If you need to manually generate certificates, you can run:

```bash
docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh
```

This will generate all certificate types in the `/app/certs/` directory.

## Verifying Certificate Types

To verify the certificate types, you can run:

```bash
docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /opt/openssl35/bin/openssl x509 -in /app/certs/hybrid/ml-dsa-65/server.crt -text -noout | grep "Public Key Algorithm"
```

This should show that the certificate uses the P384_ML-DSA-65 algorithm.

## Troubleshooting

If you encounter issues with certificates, you can:

1. Check if the certificates exist:
   ```bash
   docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy ls -la /app/certs
   ```

2. Regenerate the certificates:
   ```bash
   docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh
   ```

3. Check the OpenSSL version:
   ```bash
   docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /opt/openssl35/bin/openssl version
   ```

4. Verify that OpenSSL supports post-quantum algorithms:
   ```bash
   docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /opt/openssl35/bin/openssl list -kem-algorithms | grep ML-KEM
   docker compose -f docker-compose.openssl35.yml run --rm quantum-safe-proxy /opt/openssl35/bin/openssl list -signature-algorithms | grep ML-DSA
   ```
