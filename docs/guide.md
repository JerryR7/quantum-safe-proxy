# Quantum Safe Proxy Guide

<!-- TOC -->
- [Installation](#installation)
  - [Prerequisites](#prerequisites)
  - [Installation Methods](#installation-methods)
    - [Using Docker (Recommended)](#using-docker-recommended)
    - [From Crates.io](#from-cratesio)
    - [From Source](#from-source)
  - [Verifying the Installation](#verifying-the-installation)
- [Certificate Types](#certificate-types)
  - [1. Traditional Certificates](#1-traditional-certificates)
  - [2. Hybrid Certificates](#2-hybrid-certificates)
  - [3. Pure Post-Quantum Certificates](#3-pure-post-quantum-certificates)
- [Utility Scripts](#utility-scripts)
  - [Certificate Generation Scripts](#certificate-generation-scripts)
  - [OpenSSL Installation Scripts](#openssl-installation-scripts)
  - [Configuration Files](#configuration-files)
  - [Choosing the Right Script](#choosing-the-right-script)
- [Working with Certificates](#working-with-certificates)
  - [Installing OpenSSL with Post-Quantum Support](#installing-openssl-with-post-quantum-support)
  - [Generating Certificates](#generating-certificates)
  - [Testing Different Certificate Types](#testing-different-certificate-types)
  - [Verifying Certificate Types](#verifying-certificate-types)
- [Performance and Compatibility](#performance-and-compatibility)
  - [Performance Considerations](#performance-considerations)
  - [Compatibility Notes](#compatibility-notes)
- [Troubleshooting](#troubleshooting)
<!-- /TOC -->

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
  - OpenSSL development libraries
  - C compiler (gcc, clang, or MSVC)

### Installation Methods

#### Using Docker (Recommended)

The easiest way to get started with Quantum Safe Proxy is using Docker:

1. **Pull the image**:

   ```bash
   docker pull jerryr7/quantum-safe-proxy:latest
   ```

2. **Create a docker-compose.yml file**:

   ```yaml
   services:
     quantum-safe-proxy:
       image: jerryr7/quantum-safe-proxy:oqs
       ports:
         - "8443:8443"
       volumes:
         - ./certs:/app/certs
         - ./config:/app/config
       command: [
         "--listen", "0.0.0.0:8443",
         "--target", "backend:6000",
         "--cert", "/app/certs/server.crt",
         "--key", "/app/certs/server.key",
         "--ca-cert", "/app/certs/ca.crt",
         "--log-level", "debug"
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

3. **Start the services**:

   ```bash
   docker-compose up -d
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

## Certificate Types

The Quantum Safe Proxy supports three categories of certificates:

### 1. Traditional Certificates

These use classical cryptographic algorithms that are widely supported but vulnerable to quantum attacks:

- **RSA**: The most common certificate type, uses integer factorization
- **ECDSA**: Uses elliptic curve cryptography, more efficient than RSA

### 2. Hybrid Certificates

These combine traditional and post-quantum algorithms for maximum security and compatibility:

- **Dilithium3 + ECDSA**: Combines NIST security level 2 post-quantum with traditional ECDSA
- **Dilithium5/ML-DSA-87 + ECDSA**: Uses higher security level post-quantum algorithm
- **Falcon-1024 + ECDSA**: Uses an alternative lattice-based post-quantum algorithm

### 3. Pure Post-Quantum Certificates

These use only post-quantum algorithms, providing maximum quantum resistance:

- **Dilithium3**: NIST security level 2 post-quantum algorithm
- **Dilithium5/ML-DSA-87**: NIST security level 3 post-quantum algorithm
- **Falcon-1024**: Alternative lattice-based post-quantum algorithm

## Utility Scripts

The project includes several utility scripts to help with certificate generation, OpenSSL installation, and configuration.

### Certificate Generation Scripts

| Script | Description | Usage |
|--------|-------------|-------|
| `generate_certificates.sh` | **Main certificate generation script** that creates a complete set of certificates for all supported algorithms (traditional, hybrid, and pure post-quantum). | Run inside Docker container:<br>`docker compose exec quantum-safe-proxy /scripts/generate_certificates.sh` |
| `generate-test-certs.sh` | **Simplified certificate generation script** for development and testing. Creates a smaller set of certificates. | Run on host system:<br>`./scripts/generate-test-certs.sh` |

### OpenSSL Installation Scripts

| Script | Description | Usage |
|--------|-------------|-------|
| `install-oqs-provider.sh` | **RECOMMENDED** installation script for OpenSSL 3.x with OQS Provider. Uses modern OpenSSL architecture with pluggable providers. | `./scripts/install-oqs-provider.sh [OPTIONS]` |
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

- For **production environments**, use `generate_certificates.sh` to create a complete set of certificates.
- For **development and testing**, use `generate-test-certs.sh` for a simpler setup.
- For **installing OpenSSL with post-quantum support**:
  - New projects should use `install-oqs-provider.sh` (OpenSSL 3.x)
  - Legacy systems can use `install-oqs.sh` (OpenSSL 1.1.1)

## Working with Certificates

### Installing OpenSSL with Post-Quantum Support

To use post-quantum cryptography features, you need OpenSSL with OQS support.

#### OpenSSL 3.x with OQS Provider (Recommended)

For new projects, we recommend using OpenSSL 3.x with the OQS Provider:

```bash
# Run the installation script
./scripts/install-oqs-provider.sh

# Source the environment variables
source /opt/oqs/env.sh
```

This will install OpenSSL 3.x with OQS Provider to `/opt/oqs` by default.

#### Legacy OpenSSL 1.1.1 with OQS Patches

For compatibility with older systems, you can use OpenSSL 1.1.1 with OQS patches:

```bash
# Run the installation script
./scripts/install-oqs.sh

# Source the environment variables
source /opt/oqs-openssl/env.sh
```

This will install OQS-OpenSSL 1.1.1 to `/opt/oqs-openssl` by default.

### Generating Certificates

The project includes two scripts to generate certificates:

#### Complete Certificate Set (Recommended for Production)

To generate a complete set of certificates including all types (traditional, hybrid, and post-quantum):

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
│   ├── dilithium3/
│   ├── dilithium5/
│   └── falcon1024/
└── post-quantum/
    ├── dilithium3/
    ├── dilithium5/
    └── falcon1024/
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
```

#### Hybrid Dilithium5/ML-DSA-87 Certificates

```yaml
command: >
  --listen 0.0.0.0:8443
  --target backend:6000
  --cert /app/certs/hybrid/dilithium5/server.crt
  --key /app/certs/hybrid/dilithium5/server.key
  --ca-cert /app/certs/hybrid/dilithium5/ca.crt
```

#### Pure Post-Quantum Dilithium5/ML-DSA-87 Certificates

```yaml
command: >
  --listen 0.0.0.0:8443
  --target backend:6000
  --cert /app/certs/post-quantum/dilithium5/server.crt
  --key /app/certs/post-quantum/dilithium5/server.key
  --ca-cert /app/certs/post-quantum/dilithium5/ca.crt
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

## Troubleshooting

If you encounter issues:

- **OpenSSL errors**: Ensure you have the correct version of OpenSSL installed and environment variables set
- **Compilation errors**: Make sure you have the required development libraries installed
- **Docker errors**: Check that Docker and Docker Compose are properly installed and running
- **Certificate errors**: Verify that certificates are generated correctly and paths are set properly

For more detailed troubleshooting, check the logs:

```bash
docker compose logs quantum-safe-proxy
```

Or run with increased log level:

```bash
quantum-safe-proxy --log-level debug [other options]
```
