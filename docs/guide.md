# Quantum Safe Proxy Guide

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
  - [Prerequisites](#prerequisites)
  - [Installation Methods](#installation-methods)
    - [Using Docker (Recommended)](#using-docker-recommended)
    - [From Crates.io](#from-cratesio)
    - [From Source](#from-source)
  - [Verifying the Installation](#verifying-the-installation)
- [Configuration](#configuration)
  - [Configuration Methods](#configuration-methods)
  - [Configuration Priority](#configuration-priority)
  - [Configuration Hot Reload](#configuration-hot-reload)
  - [Environment-Specific Configuration](#environment-specific-configuration)
  - [Configuration Options](#configuration-options)
- [Post-Quantum Cryptography](#post-quantum-cryptography)
  - [OpenSSL 3.5+ Support](#openssl-35-support)
  - [Certificate Types](#certificate-types)
    - [Traditional Certificates](#traditional-certificates)
    - [Hybrid Certificates](#hybrid-certificates)
    - [Pure Post-Quantum Certificates](#pure-post-quantum-certificates)
  - [Supported Algorithms](#supported-algorithms)
- [Utility Scripts](#utility-scripts)
  - [Certificate Generation Scripts](#certificate-generation-scripts)
  - [OpenSSL Installation Scripts](#openssl-installation-scripts)
  - [Configuration Files](#configuration-files)
  - [Choosing the Right Script](#choosing-the-right-script)
- [Working with Certificates](#working-with-certificates)
  - [Installing OpenSSL with Post-Quantum Support](#installing-openssl-with-post-quantum-support)
  - [Generating Certificates](#generating-certificates)
  - [Certificate Directory Structure](#certificate-directory-structure)
  - [Testing Different Certificate Types](#testing-different-certificate-types)
  - [Verifying Certificate Types](#verifying-certificate-types)
- [Performance and Compatibility](#performance-and-compatibility)
  - [Performance Considerations](#performance-considerations)
  - [Compatibility Notes](#compatibility-notes)
- [Migrating from OQS to OpenSSL 3.5+](#migrating-from-oqs-to-openssl-35)
- [Troubleshooting](#troubleshooting)
  - [Common Issues](#common-issues)
  - [Diagnostic Tools](#diagnostic-tools)

## Introduction

Quantum Safe Proxy is a TLS proxy designed to provide post-quantum cryptographic protection for existing services. It sits between clients and your backend services, handling TLS connections with post-quantum algorithms while allowing your existing services to remain unchanged.

This comprehensive guide covers everything you need to know about installing, configuring, and using the Quantum Safe Proxy, including working with different types of certificates and cryptography.

## Installation

### Prerequisites

Before installing Quantum Safe Proxy, ensure you have the following prerequisites:

- **For Docker installation**:
  - Docker Engine 20.10.0 or later
  - Docker Compose v2 or later

- **For native installation**:
  - Rust 1.70.0 or later
  - Cargo package manager
  - OpenSSL 3.5.0 or newer with development libraries
  - C compiler (gcc, clang, or MSVC)

### Installation Methods

#### Using Docker (Recommended)

The easiest way to get started with Quantum Safe Proxy is using Docker. Follow these steps for the best experience:

1. **Build the Docker images**:

   First, build the Docker images manually. This provides better control over the build process and prevents dangling (`<none>`) images.

   ```bash
   # Build standard image
   docker build -f docker/Dockerfile -t quantum-safe-proxy:latest .

   # Build OpenSSL 3.5 image (with built-in post-quantum support)
   docker build -f docker/Dockerfile.openssl35 -t quantum-safe-proxy:openssl35 .

   # Build OQS image (with legacy post-quantum support)
   docker build -f docker/Dockerfile.oqs -t quantum-safe-proxy:oqs .
   ```

2. **Create a docker-compose.yml file**:

   ```yaml
   services:
     quantum-safe-proxy:
       # Choose one of the following images:
       image: quantum-safe-proxy:openssl35  # Use OpenSSL 3.5 with built-in PQC
       # image: quantum-safe-proxy:oqs      # Use legacy OQS provider
       ports:
         - "8443:8443"
       volumes:
         - ./certs:/app/certs
         - ./config:/app/config
       command: [
         "--listen", "0.0.0.0:8443",
         "--target", "backend:6000",
         # For OpenSSL 3.5, use ML-DSA certificates
         "--cert", "/app/certs/hybrid/ml-dsa-65/server.crt",
         "--key", "/app/certs/hybrid/ml-dsa-65/server.key",
         "--ca-cert", "/app/certs/hybrid/ml-dsa-65/ca.crt",
         # For OQS provider, use Dilithium certificates
         # "--cert", "/app/certs/hybrid/dilithium3/server.crt",
         # "--key", "/app/certs/hybrid/dilithium3/server.key",
         # "--ca-cert", "/app/certs/hybrid/dilithium3/ca.crt",
         "--log-level", "debug",
         "--client-cert-mode", "optional"
       ]
       networks:
         - proxy-network
       restart: unless-stopped
       depends_on:
         - backend

     backend:
       image: nginx:alpine
       container_name: backend-service
       ports:
         - "6000:6000"
       networks:
         - proxy-network
       restart: unless-stopped

   networks:
     proxy-network:
       driver: bridge
   ```

   > **Note**: Notice that we're using `image:` instead of including a `build:` section. This approach prevents the creation of dangling images. Choose either `quantum-safe-proxy:openssl35` for OpenSSL 3.5 with built-in PQC or `quantum-safe-proxy:oqs` for legacy OQS provider support.

3. **Start the services**:

   ```bash
   docker-compose up -d
   ```

4. **Updating images**:

   When your code changes and you need to update the images:

   ```bash
   # Rebuild the image (choose the appropriate Dockerfile)
   docker build -f docker/Dockerfile.openssl35 -t quantum-safe-proxy:openssl35 .
   # or
   docker build -f docker/Dockerfile.oqs -t quantum-safe-proxy:oqs .

   # Restart the services
   docker-compose down
   docker-compose up -d
   ```

5. **Image management best practices**:

   If you still see `<none>` tagged images, you can clean them up with:

   ```bash
   docker image prune -f
   ```

   For important versions, consider adding version tags:

   ```bash
   # Build with version tag
   docker build -f docker/Dockerfile.oqs -t quantum-safe-proxy:oqs-1.0.0 .

   # Add the oqs tag for docker-compose
   docker tag quantum-safe-proxy:oqs-1.0.0 quantum-safe-proxy:oqs
   ```

#### From Crates.io

To install from Crates.io (Rust package registry):

```bash
cargo install quantum-safe-proxy
```

This will install the `quantum-safe-proxy` binary in your Cargo bin directory.

#### From Source

To build and install from source:

1. **Clone the repository**:

   ```bash
   git clone https://github.com/JerryR7/quantum-safe-proxy.git
   cd quantum-safe-proxy
   ```

2. **Build the project**:

   ```bash
   cargo build --release
   ```

3. **Install the binary** (optional):

   ```bash
   cargo install --path .
   ```

### Verifying the Installation

To verify that Quantum Safe Proxy is installed correctly:

```bash
# Check the version
quantum-safe-proxy --version

# Check if OpenSSL with OQS is available
quantum-safe-proxy check-environment
```

If you're using Docker, you can verify the installation with:

```bash
docker compose exec quantum-safe-proxy quantum-safe-proxy --version
```

## Configuration

Quantum Safe Proxy offers flexible configuration options with a clear priority system and environment-specific configurations.

### Configuration Methods

You can configure the proxy using any of the following methods:

1. **Command-line Arguments**:
   ```bash
   quantum-safe-proxy --listen 0.0.0.0:8443 --target 127.0.0.1:6000 \
     --cert certs/hybrid/dilithium3/server.crt \
     --key certs/hybrid/dilithium3/server.key \
     --ca-cert certs/hybrid/dilithium3/ca.crt \
     --client-cert-mode optional \
     --log-level debug
   ```

2. **Environment Variables**:
   ```bash
   export QUANTUM_SAFE_PROXY_LISTEN="0.0.0.0:8443"
   export QUANTUM_SAFE_PROXY_TARGET="127.0.0.1:6000"
   export QUANTUM_SAFE_PROXY_CERT="certs/hybrid/dilithium3/server.crt"
   export QUANTUM_SAFE_PROXY_KEY="certs/hybrid/dilithium3/server.key"
   export QUANTUM_SAFE_PROXY_CA_CERT="certs/hybrid/dilithium3/ca.crt"
   export QUANTUM_SAFE_PROXY_LOG_LEVEL="debug"
   export QUANTUM_SAFE_PROXY_CLIENT_CERT_MODE="optional"

   quantum-safe-proxy --from-env
   ```

3. **Configuration File**:
   ```bash
   # Create a config.json file
   cat > config.json << EOF
   {
     "listen": "0.0.0.0:8443",
     "target": "127.0.0.1:6000",
     "cert_path": "certs/hybrid/dilithium3/server.crt",
     "key_path": "certs/hybrid/dilithium3/server.key",
     "ca_cert_path": "certs/hybrid/dilithium3/ca.crt",
     "log_level": "debug",
     "hybrid_mode": true,
     "client_cert_mode": "optional",
     "environment": "production"
   }
   EOF

   # Run with the configuration file
   quantum-safe-proxy --config-file config.json
   ```

### Configuration Priority

When multiple configuration methods are used, the following priority order applies:

1. **Command-line Arguments** (highest priority)
2. **Environment Variables**
3. **Configuration File**
4. **Default Values** (lowest priority)

This means that command-line arguments will override environment variables, which will override configuration file values, which will override default values.

### Configuration Hot Reload

Quantum Safe Proxy supports hot reloading of configuration without restarting the service. This is particularly useful in production environments where downtime should be minimized.

#### Hot Reload Methods

##### On Unix-like Systems (Linux, macOS)

On Unix-like systems, you can trigger a configuration reload by sending a SIGHUP signal to the process:

```bash
# Find the process ID
pidof quantum-safe-proxy

# Send SIGHUP signal
kill -HUP <process_id>
```

Alternatively, if you're using systemd:

```bash
systemctl kill --signal=HUP quantum-safe-proxy
```

##### On Windows

On Windows, the proxy automatically checks for configuration file changes every 30 seconds. To reload the configuration:

1. Modify the configuration file
2. Save the changes
3. Wait for up to 30 seconds for the changes to be detected and applied

#### What Gets Reloaded

The following configuration options can be changed during hot reload:

- **Target service address**: You can change where the proxy forwards traffic to
- **TLS certificates and keys**: Update certificates without downtime
- **Client certificate verification mode**: Change between required, optional, and none
- **Log level**: Adjust logging verbosity on the fly
- **Hybrid mode**: Enable or disable hybrid certificate support
- **CA certificate**: Update the CA certificate used for client verification

#### What Doesn't Get Reloaded

Some configuration options cannot be changed during hot reload and require a restart:

- **Listen address**: Changing the listen address requires restarting the listener

#### Monitoring Hot Reload

When a configuration reload is triggered, the proxy will log information about the reload process:

```
INFO: Reloading configuration from config.json
INFO: New target address: 127.0.0.1:7000
INFO: Proxy configuration reloaded successfully
```

If there are any issues with the new configuration, warnings or errors will be logged, and the proxy will continue using the previous configuration.

### Environment-Specific Configuration

You can create environment-specific configuration files for different environments:

```bash
# Development environment configuration
cat > config.development.json << EOF
{
  "listen": "127.0.0.1:8443",
  "target": "127.0.0.1:6000",
  "cert_path": "certs/hybrid/dilithium3/server.crt",
  "key_path": "certs/hybrid/dilithium3/server.key",
  "ca_cert_path": "certs/hybrid/dilithium3/ca.crt",
  "log_level": "debug",
  "hybrid_mode": true,
  "client_cert_mode": "optional",
  "environment": "development"
}
EOF
```

To use an environment-specific configuration:

```bash
quantum-safe-proxy --environment development
```

The proxy will automatically look for a `config.{environment}.json` file and load it if it exists.

### Configuration Options

| Option | Description | Default Value |
|--------|-------------|--------------|
| `listen` | Listen address for the proxy server | `0.0.0.0:8443` |
| `target` | Target service address to forward traffic to | `127.0.0.1:6000` |
| `cert_path` | Server certificate path (legacy parameter) | `certs/hybrid/ml-dsa-87/server.crt` |
| `key_path` | Server private key path (legacy parameter) | `certs/hybrid/ml-dsa-87/server.key` |
| `classic_cert` | Path to classic (RSA/ECDSA) certificate | - |
| `classic_key` | Path to classic private key | - |
| `use_sigalgs` | Auto-select certificate by client signature_algorithms | `false` |
| `ca_cert_path` | CA certificate path for client certificate validation | `certs/hybrid/ml-dsa-87/ca.crt` |
| `log_level` | Log level (debug, info, warn, error) | `info` |
| `client_cert_mode` | Client certificate verification mode (required, optional, none) | `optional` |
| `buffer_size` | Buffer size for data transfer in bytes | `8192` |
| `connection_timeout` | Connection timeout in seconds | `30` |
| `openssl_dir` | Path to OpenSSL installation directory | - |

## Post-Quantum Cryptography

### OpenSSL 3.5+ Support

Quantum Safe Proxy uses OpenSSL 3.5+ which includes built-in support for post-quantum cryptography. This provides several advantages:

- **Standardized algorithms**: Uses NIST-standardized algorithms like ML-KEM and ML-DSA
- **Better integration**: Native integration with OpenSSL's API and tools
- **Improved performance**: Optimized implementations of post-quantum algorithms
- **Regular updates**: Benefits from OpenSSL's security updates and improvements

### Certificate Types

The Quantum Safe Proxy supports three categories of certificates:

#### Traditional Certificates

These use classical cryptographic algorithms that are widely supported but vulnerable to quantum attacks:

- **RSA**: The most common certificate type, uses integer factorization
- **ECDSA**: Uses elliptic curve cryptography, more efficient than RSA
- **Ed25519**: Edwards-curve Digital Signature Algorithm, offering better performance than ECDSA

#### Hybrid Certificates

These combine traditional and post-quantum algorithms for maximum security and compatibility:

- **ML-DSA-44 + ECDSA**: Combines NIST security level 2 post-quantum with traditional ECDSA (formerly Dilithium2)
- **ML-DSA-65 + ECDSA**: Uses medium security level post-quantum algorithm (formerly Dilithium3)
- **ML-DSA-87 + ECDSA**: Uses higher security level post-quantum algorithm (formerly Dilithium5)
- **ML-KEM-768 + X25519**: Combines post-quantum key exchange with traditional elliptic curve

#### Pure Post-Quantum Certificates

These use only post-quantum algorithms, providing maximum quantum resistance:

- **ML-DSA-44**: NIST security level 2 post-quantum algorithm (formerly Dilithium2)
- **ML-DSA-65**: NIST security level 3 post-quantum algorithm (formerly Dilithium3)
- **ML-DSA-87**: NIST security level 5 post-quantum algorithm (formerly Dilithium5)

### Supported Algorithms

| Type | Algorithms (OpenSSL 3.5+) | Description |
|------|---------------------------|-------------|
| **Key Exchange** | ML-KEM-512, ML-KEM-768, ML-KEM-1024 | NIST standardized post-quantum key encapsulation mechanisms (formerly Kyber) |
| **Signatures** | ML-DSA-44, ML-DSA-65, ML-DSA-87 | NIST standardized post-quantum digital signature algorithms (formerly Dilithium) |
| **Lattice-Based Signatures** | SLH-DSA-FALCON-512, SLH-DSA-FALCON-1024 | Stateless hash-based digital signature algorithms |
| **Hybrid Groups** | X25519MLKEM768, P256MLKEM768, P384MLKEM1024 | Hybrid key exchange combining classical and post-quantum algorithms |
| **Classical Fallback** | ECDSA (P-256, P-384, P-521), RSA, Ed25519 | Traditional algorithms for backward compatibility |

## Utility Scripts

The project includes several utility scripts to help with certificate generation, OpenSSL installation, and configuration.

### Certificate Generation Scripts

| Script | Description | Usage |
|--------|-------------|-------|
| `generate-openssl35-certs.sh` | **Recommended certificate generation script** that creates a complete set of certificates using OpenSSL 3.5+ with built-in PQC support. Generates ML-DSA and ML-KEM certificates. | Run inside Docker container:<br>`docker compose exec quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh` |
| `generate-oqs-certs.sh` | **Legacy certificate generation script** that creates certificates using OQS provider. Generates Dilithium and Falcon certificates. | Run inside Docker container:<br>`docker compose exec quantum-safe-proxy /app/scripts/generate-oqs-certs.sh` |
| `generate-test-certs.sh` | **Simplified certificate generation script** for development and testing. Creates a smaller set of certificates. | Run on host system:<br>`./scripts/generate-test-certs.sh` |

### OpenSSL Installation Scripts

| Script | Description | Usage |
|--------|-------------|-------|
| `build-openssl35.sh` | **RECOMMENDED** script for building Docker image with OpenSSL 3.5+ that has built-in post-quantum cryptography support. | `./scripts/build-openssl35.sh` |
| `openssl35-install.sh` | Installation script for OpenSSL 3.5+ on the host system. | `./scripts/openssl35-install.sh` |
| `build-proxy-with-openssl35.sh` | Builds the Quantum Safe Proxy Docker image with OpenSSL 3.5+ and verifies the installation. | `./scripts/build-proxy-with-openssl35.sh` |
| `install-oqs-provider.sh` | Installation script for OpenSSL 3.x with OQS Provider. Uses modern OpenSSL architecture with pluggable providers. | `./scripts/install-oqs-provider.sh [OPTIONS]` |
| `install-oqs.sh` | **LEGACY** installation script for OpenSSL 1.1.1 with OQS patches. Provided for backward compatibility only. | `./scripts/install-oqs.sh [OPTIONS]` |

### Configuration Files

The project includes several OpenSSL configuration files for certificate generation:

| File | Description | Usage |
|------|-------------|-------|
| `scripts/openssl-hybrid.conf` | Standalone OpenSSL configuration template for manually generating hybrid certificates. | Used with OpenSSL commands:<br>`openssl req -x509 -new -newkey dilithium3 -keyout ca.key -out ca.crt -config openssl-hybrid.conf -nodes -days 365 -extensions v3_ca` |
| `certs/config/cert.cnf` | Configuration file generated by `generate_certificates.sh` for automated certificate generation. | Generated and used automatically by the script, not intended for manual use. |
| Inline configs in `generate-test-certs.sh` | Configuration templates created within the test certificate generation script. | Generated and used automatically by the script, not intended for manual use. |

#### Relationship Between Configuration Files

These configuration files serve different purposes but contain similar settings:

- **`scripts/openssl-hybrid.conf`**: A standalone template for manual certificate generation. It includes comprehensive sections for CA, server, and client certificates.

- **`certs/config/cert.cnf`**: Generated by `generate_certificates.sh` for automated use. It's similar to `openssl-hybrid.conf` but with settings tailored for the automated script.

- **Inline configs in `generate-test-certs.sh`**: Created within the script for testing purposes. These are simplified versions focused on quick test certificate generation.

### Choosing the Right Script

- For **production environments**:
  - Use `generate-openssl35-certs.sh` to create certificates with OpenSSL 3.5+ (recommended)
  - Use `generate_certificates.sh` for legacy OQS provider certificates
- For **development and testing**, use `generate-test-certs.sh` for a simpler setup.
- For **installing OpenSSL with post-quantum support**:
  - New projects should use `build-openssl35.sh` to build a Docker image with OpenSSL 3.5+ (recommended)
  - Alternative approach: use `install-oqs-provider.sh` for OpenSSL 3.x with OQS provider
  - Legacy systems can use `install-oqs.sh` (OpenSSL 1.1.1)

## Working with Certificates

### Installing OpenSSL with Post-Quantum Support

To use post-quantum cryptography features, you need OpenSSL with PQC support. There are three options available:

#### Option 1: OpenSSL 3.5+ with Built-in PQC (Recommended)

For new projects, we recommend using OpenSSL 3.5+ (or newer versions like 3.6+, 3.7+) which has built-in support for post-quantum cryptography:

```bash
# Build the Docker image with OpenSSL 3.5+
./scripts/build-openssl35.sh

# Verify the installation
docker run --rm quantum-safe-proxy:openssl35 /opt/openssl35/bin/openssl version
docker run --rm quantum-safe-proxy:openssl35 /opt/openssl35/bin/openssl list -kem-algorithms | grep -i ML-KEM
docker run --rm quantum-safe-proxy:openssl35 /opt/openssl35/bin/openssl list -signature-algorithms | grep -i ML-DSA
```

This will build a Docker image with OpenSSL 3.5+ installed at `/opt/openssl35`.

Alternatively, you can install OpenSSL 3.5+ directly on your system using the provided script:

```bash
# Run the installation script
./scripts/openssl35-install.sh

# Verify the installation
/opt/openssl35/bin/openssl version
/opt/openssl35/bin/openssl list -kem-algorithms | grep -i ML-KEM
```

#### Option 2: OpenSSL 3.x with OQS Provider

As an alternative, you can use OpenSSL 3.x with the OQS Provider:

```bash
# Run the installation script
./scripts/install-oqs-provider.sh

# Source the environment variables
source /opt/oqs/env.sh
```

This will install OpenSSL 3.x with OQS Provider to `/opt/oqs` by default.

#### Option 3: Legacy OpenSSL 1.1.1 with OQS Patches (Deprecated)

For compatibility with older systems, you can use OpenSSL 1.1.1 with OQS patches:

```bash
# Run the installation script
./scripts/install-oqs.sh

# Source the environment variables
source /opt/oqs-openssl/env.sh
```

This will install OQS-OpenSSL 1.1.1 to `/opt/oqs-openssl` by default.

### Generating Certificates

The project includes several scripts to generate certificates:

#### OpenSSL 3.5+ Certificates (Recommended for Production)

To generate a complete set of certificates using OpenSSL 3.5+ with built-in PQC support:

```bash
# Run the script inside the Docker container
docker compose exec quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh
```

This script generates certificates for all supported algorithms in OpenSSL 3.5+, including:
- Traditional certificates (RSA, ECDSA)
- Hybrid certificates (ML-DSA-44/65/87 + ECDSA, ML-KEM-768 + X25519)
- Pure post-quantum certificates (ML-DSA-44/65/87)

#### OQS Provider Certificates (Legacy Support)

To generate certificates using the OQS Provider:

```bash
# Run the script inside the Docker container
docker compose exec quantum-safe-proxy /scripts/generate_certificates.sh
```

#### Simple Test Certificates (For Development)

For quick development and testing, you can use the simplified certificate generation script:

```bash
# Make sure OpenSSL with OQS Provider is installed on your host system
./scripts/generate-test-certs.sh
```

This simplified script generates only three certificates (CA, server, client) using Kyber768 + ECDSA hybrid algorithms.

This will create certificates in the following directory structure:

```
/app/certs/
├── config/
│   └── cert.cnf
├── traditional/
│   ├── rsa/
│   └── ecdsa/
├── hybrid/
│   ├── ml-dsa-44/       # OpenSSL 3.5+ (equivalent to Dilithium2)
│   ├── ml-dsa-65/       # OpenSSL 3.5+ (equivalent to Dilithium3)
│   ├── ml-dsa-87/       # OpenSSL 3.5+ (equivalent to Dilithium5)
│   ├── ml-kem-768/      # OpenSSL 3.5+ (equivalent to Kyber768)
│   ├── dilithium3/      # OQS Provider (symlinks to ml-dsa-44 for compatibility)
│   ├── dilithium5/      # OQS Provider (symlinks to ml-dsa-87 for compatibility)
│   └── falcon1024/      # OQS Provider only
└── post-quantum/
    ├── ml-dsa-44/       # OpenSSL 3.5+ (pure PQC)
    ├── ml-dsa-65/       # OpenSSL 3.5+ (pure PQC)
    ├── ml-dsa-87/       # OpenSSL 3.5+ (pure PQC)
    ├── dilithium3/      # OQS Provider (symlinks to ml-dsa-44 for compatibility)
    ├── dilithium5/      # OQS Provider (symlinks to ml-dsa-87 for compatibility)
    └── falcon1024/      # OQS Provider only
```

Each directory contains:
- `ca.key`: CA private key
- `ca.crt`: CA certificate
- `server.key`: Server private key
- `server.crt`: Server certificate

### Testing Different Certificate Types

To test different certificate types, modify your `docker-compose.yml` file:

#### Traditional RSA Certificates

```yaml
command: >
  --listen 0.0.0.0:8443
  --target backend:6000
  --cert /app/certs/traditional/rsa/server.crt
  --key /app/certs/traditional/rsa/server.key
  --ca-cert /app/certs/traditional/rsa/ca.crt
  --client-cert-mode optional
```

#### Hybrid ML-DSA-65 Certificates (OpenSSL 3.5+)

```yaml
command: >
  --listen 0.0.0.0:8443
  --target backend:6000
  --cert /app/certs/hybrid/ml-dsa-65/server.crt
  --key /app/certs/hybrid/ml-dsa-65/server.key
  --ca-cert /app/certs/hybrid/ml-dsa-65/ca.crt
  --client-cert-mode optional
```

#### Hybrid Dilithium5/ML-DSA-87 Certificates

```yaml
command: >
  --listen 0.0.0.0:8443
  --target backend:6000
  --cert /app/certs/hybrid/ml-dsa-87/server.crt  # or dilithium5/server.crt for OQS provider
  --key /app/certs/hybrid/ml-dsa-87/server.key   # or dilithium5/server.key for OQS provider
  --ca-cert /app/certs/hybrid/ml-dsa-87/ca.crt   # or dilithium5/ca.crt for OQS provider
  --client-cert-mode optional
```

#### Pure Post-Quantum ML-DSA-65 Certificates (OpenSSL 3.5+)

```yaml
command: >
  --listen 0.0.0.0:8443
  --target backend:6000
  --cert /app/certs/post-quantum/ml-dsa-65/server.crt
  --key /app/certs/post-quantum/ml-dsa-65/server.key
  --ca-cert /app/certs/post-quantum/ml-dsa-65/ca.crt
  --client-cert-mode optional
```

After modifying the configuration, restart the services:

```bash
docker compose down
docker compose up -d
```

### Verifying Certificate Types

You can verify which certificate type is being used by checking the logs:

```bash
docker compose logs quantum-safe-proxy
```

Look for messages like:
- `Using traditional certificate, not hybrid` (for traditional certificates)
- `Hybrid certificate mode enabled` (for hybrid certificates)
- `Using OpenSSL 3.5+ with built-in post-quantum support` (when using OpenSSL 3.5+)
- `Post-quantum key exchange algorithms (ML-KEM) are available` (when using OpenSSL 3.5+)
- `Post-quantum signature algorithms (ML-DSA) are available` (when using OpenSSL 3.5+)
- `Using OpenSSL with oqs-provider` (when using OQS provider)
- Messages about post-quantum algorithms being used

## Performance and Compatibility

### Performance Considerations

Different certificate types have different performance characteristics:

- **Traditional certificates**: Smallest size, fastest processing
- **Hybrid certificates**: Larger size, moderate processing overhead
- **Pure post-quantum certificates**: Medium size, higher processing overhead

When testing, consider monitoring:
- TLS handshake time
- CPU usage
- Memory usage
- Network bandwidth

### Compatibility Notes

- Traditional certificates work with all TLS clients
- Hybrid certificates work with most clients (the traditional part ensures compatibility)
- Pure post-quantum certificates only work with clients that support the specific algorithm

For maximum compatibility and security, hybrid certificates are recommended for most use cases.

## Migrating from OQS to OpenSSL 3.5+

If you're currently using the OQS provider and want to migrate to OpenSSL 3.5+ with built-in PQC support, follow these steps:

### 1. Build the OpenSSL 3.5+ Docker Image

```bash
./scripts/build-openssl35.sh
```

### 2. Update Your docker-compose.yml File

Change the image from `quantum-safe-proxy:oqs` to `quantum-safe-proxy:openssl35`:

```yaml
services:
  quantum-safe-proxy:
    image: quantum-safe-proxy:openssl35  # Changed from quantum-safe-proxy:oqs
    # rest of configuration...
```

### 3. Generate New Certificates

```bash
docker compose exec quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh
```

### 4. Update Certificate Paths

Update your certificate paths to use the new ML-DSA certificates:

```yaml
command: [
  # other options...
  "--cert", "/app/certs/hybrid/ml-dsa-65/server.crt",  # Changed from dilithium3
  "--key", "/app/certs/hybrid/ml-dsa-65/server.key",   # Changed from dilithium3
  "--ca-cert", "/app/certs/hybrid/ml-dsa-65/ca.crt",  # Changed from dilithium3
  # other options...
]
```

### 5. Restart the Services

```bash
docker compose down
docker compose up -d
```

### 6. Verify the Migration

Check the logs to confirm that OpenSSL 3.5+ is being used:

```bash
docker compose logs quantum-safe-proxy
```

Look for messages like:
- `Using OpenSSL 3.5+ with built-in post-quantum support`
- `Post-quantum key exchange algorithms (ML-KEM) are available`
- `Post-quantum signature algorithms (ML-DSA) are available`

### Compatibility Notes

- The `generate-openssl35-certs.sh` script creates symbolic links for backward compatibility, so existing paths like `/app/certs/hybrid/dilithium3/` will still work, but they now point to the ML-DSA certificates.
- The algorithm names have changed (Dilithium → ML-DSA, Kyber → ML-KEM), but the functionality remains the same.
- OpenSSL 3.5+ provides better integration and standardization compared to the OQS provider.

## Troubleshooting

### Common Issues

#### OpenSSL Installation Issues

If you encounter issues with OpenSSL:

```bash
# Verify OpenSSL installation
/opt/openssl35/bin/openssl version

# Check for post-quantum algorithms
/opt/openssl35/bin/openssl list -kem-algorithms | grep -i ML-KEM
/opt/openssl35/bin/openssl list -signature-algorithms | grep -i ML-DSA
```

#### Certificate Issues

If you encounter certificate-related issues:

```bash
# Verify certificate paths
ls -la /app/certs/hybrid/ml-dsa-65/

# Check certificate details
/opt/openssl35/bin/openssl x509 -in /app/certs/hybrid/ml-dsa-65/server.crt -text -noout
```

#### Docker Issues

- **Image build failures**: Check Docker daemon logs and ensure you have sufficient disk space
- **Container startup failures**: Verify that ports are not already in use and volumes are properly mounted
- **Permission issues**: Ensure that certificate files have proper permissions

#### Network Issues

- **Connection refused**: Verify that the proxy is running and listening on the correct port
- **Handshake failures**: Check that the client supports the certificate type being used
- **Timeout errors**: Increase the connection timeout setting

### Diagnostic Tools

The proxy includes built-in diagnostic tools:

```bash
# Check environment
quantum-safe-proxy check-environment

# Run with increased logging
quantum-safe-proxy --log-level debug [other options]
```

For more detailed troubleshooting, check the logs:

```bash
docker compose logs quantum-safe-proxy
```
