# Quantum Proxy

[![Rust](https://github.com/yourusername/quantum-proxy/actions/workflows/rust.yml/badge.svg)](https://github.com/yourusername/quantum-proxy/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/quantum-proxy.svg)](https://crates.io/crates/quantum-proxy)
[![Documentation](https://docs.rs/quantum-proxy/badge.svg)](https://docs.rs/quantum-proxy)

PQC-Enabled Sidecar with Hybrid Certificate Support

## Overview

**Quantum Proxy** is a lightweight TCP proxy designed to secure long-term proxy connections using **Post-Quantum Cryptography (PQC)** and **hybrid X.509 certificates**. It enables secure mTLS communication through **OpenSSL + oqs-provider**, supporting both traditional and PQC algorithms via hybrid negotiation.

### Key Objectives

- Secure communications using **hybrid PQC + traditional certificates** (e.g., Kyber + ECDSA)
- Transparent support for both PQC-capable and traditional clients
- Deployable as a **sidecar proxy** without modifying existing services

## Architecture

```mermaid
graph LR
    subgraph Agent Side
        AGENT[Agent<br/>TCP Client<br/>Hybrid Certificate]
    end

    subgraph Service Side
        PROXY[Quantum Proxy<br/>Hybrid mTLS Listener → TCP Forwarder]
        SERVICE[Backend Service<br/>TCP Listener (6000)]
    end

    AGENT -->|Hybrid mTLS TCP| PROXY
    PROXY -->|Plain TCP (loopback)| SERVICE
```

## Features

- **Hybrid Certificate Support**: Seamlessly works with hybrid X.509 certificates (Kyber + ECDSA)
- **Transparent PQC Integration**: Handles both PQC and traditional clients
- **Efficient TCP Proxying**: High-performance data forwarding
- **Complete mTLS Support**: Client and server certificate validation
- **Flexible Configuration**: Command-line arguments, environment variables, and config files
- **Containerized Deployment**: Docker, docker-compose, and Kubernetes support

## Technology Stack

- **Language**: Rust
- **TLS Library**: OpenSSL with oqs-provider (hybrid certificate support)
- **Proxy Runtime**: tokio + tokio-openssl
- **Deployment**: Docker / Kubernetes / Systemd sidecar mode
- **Certificate Tools**: OQS OpenSSL CLI (hybrid CSR and certificates)

## Installation

### From Crates.io

```bash
cargo install quantum-proxy
```

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/quantum-proxy.git
cd quantum-proxy

# Build
cargo build --release
```

### Using Docker

```bash
# Pull the image
docker pull yourusername/quantum-proxy:latest

# Or build locally
docker build -t quantum-proxy .
```

## Usage

### Basic Usage

```bash
quantum-proxy --listen 0.0.0.0:8443 --target 127.0.0.1:6000 --cert certs/server.crt --key certs/server.key --ca-cert certs/ca.crt
```

### Using Environment Variables

```bash
# Set environment variables
export QUANTUM_PROXY_LISTEN="0.0.0.0:9443"
export QUANTUM_PROXY_TARGET="127.0.0.1:7000"
export QUANTUM_PROXY_CERT="certs/server.crt"
export QUANTUM_PROXY_KEY="certs/server.key"
export QUANTUM_PROXY_CA_CERT="certs/ca.crt"
export QUANTUM_PROXY_LOG_LEVEL="debug"
export QUANTUM_PROXY_HYBRID_MODE="true"

# Load configuration from environment variables
quantum-proxy --from-env
```

### Using Configuration File

Create a `config.json` file:

```json
{
  "listen": "0.0.0.0:8443",
  "target": "127.0.0.1:6000",
  "cert_path": "certs/server.crt",
  "key_path": "certs/server.key",
  "ca_cert_path": "certs/ca.crt",
  "hybrid_mode": true,
  "log_level": "info"
}
```

Then run:

```bash
quantum-proxy --config-file config.json
```

### Using Docker

```bash
docker run -p 8443:8443 \
  -v $(pwd)/certs:/app/certs \
  yourusername/quantum-proxy:latest \
  --listen 0.0.0.0:8443 \
  --target host.docker.internal:6000 \
  --cert /app/certs/server.crt \
  --key /app/certs/server.key \
  --ca-cert /app/certs/ca.crt
```

### Using docker-compose

```bash
docker-compose up -d
```

### Command-line Options

- `--listen`: Listen address (default: 0.0.0.0:8443)
- `--target`: Target service address (default: 127.0.0.1:6000)
- `--cert`: Server certificate path (default: certs/server.crt)
- `--key`: Server private key path (default: certs/server.key)
- `--ca-cert`: CA certificate path (default: certs/ca.crt)
- `--log-level`: Log level (default: info)
- `--hybrid-mode`: Enable hybrid certificate mode
- `--from-env`: Load configuration from environment variables
- `--config-file`: Load configuration from specified file

## Hybrid Certificate Generation

Hybrid certificates require the OQS OpenSSL fork and oqs-provider.

### Installing OQS OpenSSL

```bash
# Clone the OQS OpenSSL repository
git clone --branch OQS-OpenSSL_1_1_1-stable https://github.com/open-quantum-safe/openssl.git oqs-openssl
cd oqs-openssl

# Compile and install
./config --prefix=/opt/oqs-openssl shared
make -j$(nproc)
make install
```

### Generating Hybrid Certificates

```bash
# Set environment variables
export PATH="/opt/oqs-openssl/bin:$PATH"
export LD_LIBRARY_PATH="/opt/oqs-openssl/lib:$LD_LIBRARY_PATH"

# Generate hybrid certificate
openssl req -x509 -new -newkey oqsdefault -keyout certs/server.key -out certs/server.crt \
    -config openssl-hybrid.conf -nodes -days 365
```

## Development

### Project Structure

```
quantum-proxy/
├── src/
│   ├── common/
│   │   ├── error.rs       # Error handling
│   │   ├── fs.rs          # File system utilities
│   │   ├── log.rs         # Logging utilities
│   │   ├── net.rs         # Network utilities
│   │   ├── types.rs       # Shared types
│   │   └── mod.rs         # Re-exports
│   ├── config/
│   │   ├── config.rs      # Configuration structures
│   │   └── mod.rs         # Re-exports
│   ├── proxy/
│   │   ├── server.rs      # Proxy server
│   │   ├── handler.rs     # Connection handler
│   │   ├── forwarder.rs   # Data forwarding
│   │   └── mod.rs         # Re-exports
│   ├── tls/
│   │   ├── acceptor.rs    # TLS acceptor
│   │   ├── cert.rs        # Certificate handling
│   │   └── mod.rs         # Re-exports
│   ├── main.rs            # Main entry point
│   └── lib.rs             # Library entry point
├── tests/
│   └── integration_test.rs # Integration tests
├── examples/
│   ├── simple_proxy.rs     # Basic proxy example
│   ├── config_file.rs      # Config file example
│   ├── env_vars.rs         # Environment variables example
│   └── hybrid_certs.rs     # Hybrid certificate example
├── docker/
│   ├── Dockerfile          # Docker image definition
│   └── docker-compose.yml  # Docker Compose configuration
├── kubernetes/
│   ├── deployment.yaml     # Kubernetes deployment
│   └── service.yaml        # Kubernetes service
└── certs/                  # Certificate directory
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific tests
cargo test --test integration_test
```

### Running Examples

```bash
# Run the simple proxy example
cargo run --example simple_proxy

# Run the config file example
cargo run --example config_file

# Run the environment variables example
cargo run --example env_vars

# Run the hybrid certificates example
cargo run --example hybrid_certs
```

### Code Formatting and Linting

```bash
# Format code
cargo fmt

# Check code with Clippy
cargo clippy
```



## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to contribute to this project.

## License

This project is licensed under the [MIT License](LICENSE).
