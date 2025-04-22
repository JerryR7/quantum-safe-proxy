#!/bin/bash
# Build script for Quantum Safe Proxy with OpenSSL 3.5

set -e

# Build the Docker image
echo "Building Quantum Safe Proxy with OpenSSL 3.5..."
docker build -t quantum-safe-proxy:openssl35 -f docker/Dockerfile.openssl35 .

# Verify the build
echo "Verifying the build..."
docker run --rm quantum-safe-proxy:openssl35 --version

# Verify OpenSSL version and PQC support
echo "Verifying OpenSSL 3.5 and PQC support..."
docker run --rm quantum-safe-proxy:openssl35 /opt/openssl35/bin/openssl version
docker run --rm quantum-safe-proxy:openssl35 /opt/openssl35/bin/openssl list -kem-algorithms | grep -i ML-KEM
docker run --rm quantum-safe-proxy:openssl35 /opt/openssl35/bin/openssl list -signature-algorithms | grep -i ML-DSA

echo "Build completed successfully!"
echo "To run the proxy, use: docker-compose -f docker-compose.openssl35.yml up"
