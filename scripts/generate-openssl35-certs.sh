#!/bin/bash
# ============================================================================
# Generate Comprehensive Certificate Set for Quantum Safe Proxy with OpenSSL 3.5
# ============================================================================
#
# DESCRIPTION:
#   This script creates a complete set of certificates for all supported algorithms
#   using OpenSSL 3.5's native post-quantum cryptography support. It generates
#   traditional, hybrid, and pure post-quantum certificates for testing and production.
#
# USAGE:
#   docker compose -f docker compose.yml exec quantum-safe-proxy /app/scripts/generate-openssl35-certs.sh
#
# REQUIREMENTS:
#   - Must be run inside the Docker container with OpenSSL 3.5+ installed
#   - Requires OpenSSL 3.5+ with built-in post-quantum cryptography support
#
# CERTIFICATE TYPES GENERATED:
#   - Traditional certificates (RSA, ECDSA)
#   - Hybrid certificates (ML-DSA-44 with ECDSA P-256, ML-DSA-65 with ECDSA P-384, ML-DSA-87 with ECDSA P-521)
#   - Pure post-quantum certificates (ML-DSA-44, ML-DSA-65, ML-DSA-87)
#   - Hybrid KEM certificates (ML-KEM-768 with X25519)
#
# Note: Hybrid certificates are implemented as dual certificate chains, where the same CSR is signed by both
# a PQC CA and a traditional CA. The resulting certificates are concatenated into a single file.
#
# OUTPUT:
#   Creates a complete directory structure with all certificate types in /app/certs/

set -e

# Check if OpenSSL 3.5+ is available
OPENSSL_BIN="openssl"
OPENSSL_VERSION=$(${OPENSSL_BIN} version)
if ! echo "$OPENSSL_VERSION" | grep -q "3.5"; then
    # Try alternative path for OpenSSL 3.5
    if [ -f "/opt/openssl35/bin/openssl" ]; then
        OPENSSL_BIN="/opt/openssl35/bin/openssl"
        OPENSSL_VERSION=$(${OPENSSL_BIN} version)
        if ! echo "$OPENSSL_VERSION" | grep -q "3.5"; then
            echo "Error: OpenSSL 3.5+ is required for this script."
            echo "Current version: $OPENSSL_VERSION"
            echo "Please use the OpenSSL 3.5 Docker image or install OpenSSL 3.5+."
            exit 1
        fi
    else
        echo "Error: OpenSSL 3.5+ is required for this script."
        echo "Current version: $OPENSSL_VERSION"
        echo "Please use the OpenSSL 3.5 Docker image or install OpenSSL 3.5+."
        exit 1
    fi
fi

echo "Using OpenSSL: $OPENSSL_VERSION"

# Create base directories
mkdir -p /app/certs/traditional/rsa
mkdir -p /app/certs/traditional/ecdsa
mkdir -p /app/certs/hybrid/ml-dsa-44
mkdir -p /app/certs/hybrid/ml-dsa-65
mkdir -p /app/certs/hybrid/ml-dsa-87
mkdir -p /app/certs/hybrid/ml-kem-768
mkdir -p /app/certs/post-quantum/ml-dsa-44
mkdir -p /app/certs/post-quantum/ml-dsa-65
mkdir -p /app/certs/post-quantum/ml-dsa-87
mkdir -p /app/certs/config

# Create configuration file in the certs directory
cat > /app/certs/config/cert.cnf << 'CONFEOF'
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = localhost
O = Quantum Safe Proxy
OU = Testing
C = TW

[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = quantum-safe-proxy
IP.1 = 127.0.0.1

[v3_ca]
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer:always
basicConstraints = critical, CA:true
keyUsage = critical, digitalSignature, cRLSign, keyCertSign
CONFEOF

# Use the configuration file from the certs directory
CONFIG_FILE="/app/certs/config/cert.cnf"

echo "=== Generating Traditional Certificate (RSA) ==="
# Generate CA private key and certificate
${OPENSSL_BIN} genrsa -out /app/certs/traditional/rsa/ca.key 3072
${OPENSSL_BIN} req -new -x509 -key /app/certs/traditional/rsa/ca.key -out /app/certs/traditional/rsa/ca.crt -days 365 -subj "/CN=Traditional RSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genrsa -out /app/certs/traditional/rsa/server.key 2048

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/traditional/rsa/server.key -out /app/certs/traditional/rsa/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
${OPENSSL_BIN} x509 -req -in /app/certs/traditional/rsa/server.csr -CA /app/certs/traditional/rsa/ca.crt -CAkey /app/certs/traditional/rsa/ca.key -CAcreateserial -out /app/certs/traditional/rsa/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Traditional Certificate (ECDSA) ==="
# Generate CA private key and certificate
${OPENSSL_BIN} ecparam -name prime256v1 -genkey -out /app/certs/traditional/ecdsa/ca.key
${OPENSSL_BIN} req -new -x509 -key /app/certs/traditional/ecdsa/ca.key -out /app/certs/traditional/ecdsa/ca.crt -days 365 -subj "/CN=Traditional ECDSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} ecparam -name prime256v1 -genkey -out /app/certs/traditional/ecdsa/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/traditional/ecdsa/server.key -out /app/certs/traditional/ecdsa/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
${OPENSSL_BIN} x509 -req -in /app/certs/traditional/ecdsa/server.csr -CA /app/certs/traditional/ecdsa/ca.crt -CAkey /app/certs/traditional/ecdsa/ca.key -CAcreateserial -out /app/certs/traditional/ecdsa/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Checking available algorithms ==="
${OPENSSL_BIN} list -kem-algorithms | grep -i ML-KEM
${OPENSSL_BIN} list -signature-algorithms | grep -i ML-DSA

echo "=== Generating Hybrid Certificate (ML-DSA-44 with ECDSA) ==="
# Generate ECDSA CA private key and certificate
${OPENSSL_BIN} ecparam -name prime256v1 -genkey -out /app/certs/hybrid/ml-dsa-44/ecdsa_ca.key
${OPENSSL_BIN} req -new -x509 -key /app/certs/hybrid/ml-dsa-44/ecdsa_ca.key -out /app/certs/hybrid/ml-dsa-44/ecdsa_ca.crt -days 365 -subj "/CN=ECDSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate ML-DSA-44 CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-DSA-44 -keyout /app/certs/hybrid/ml-dsa-44/ca.key -out /app/certs/hybrid/ml-dsa-44/ca.crt -nodes -days 365 -subj "/CN=Hybrid ML-DSA-44 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-DSA-44 -out /app/certs/hybrid/ml-dsa-44/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/hybrid/ml-dsa-44/server.key -out /app/certs/hybrid/ml-dsa-44/server.csr -config $CONFIG_FILE

# Sign server certificate with ML-DSA-44 CA
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-dsa-44/server.csr -CA /app/certs/hybrid/ml-dsa-44/ca.crt -CAkey /app/certs/hybrid/ml-dsa-44/ca.key -CAcreateserial -out /app/certs/hybrid/ml-dsa-44/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Also sign the same CSR with ECDSA CA to create a hybrid certificate chain
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-dsa-44/server.csr -CA /app/certs/hybrid/ml-dsa-44/ecdsa_ca.crt -CAkey /app/certs/hybrid/ml-dsa-44/ecdsa_ca.key -CAcreateserial -out /app/certs/hybrid/ml-dsa-44/server_ecdsa.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Create a combined certificate file (this is a simple concatenation for demonstration)
cat /app/certs/hybrid/ml-dsa-44/server.crt /app/certs/hybrid/ml-dsa-44/server_ecdsa.crt > /app/certs/hybrid/ml-dsa-44/server_hybrid.crt

echo "=== Generating Hybrid Certificate (ML-DSA-65 with ECDSA) ==="
# Generate ECDSA CA private key and certificate
${OPENSSL_BIN} ecparam -name secp384r1 -genkey -out /app/certs/hybrid/ml-dsa-65/ecdsa_ca.key
${OPENSSL_BIN} req -new -x509 -key /app/certs/hybrid/ml-dsa-65/ecdsa_ca.key -out /app/certs/hybrid/ml-dsa-65/ecdsa_ca.crt -days 365 -subj "/CN=ECDSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate ML-DSA-65 CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-DSA-65 -keyout /app/certs/hybrid/ml-dsa-65/ca.key -out /app/certs/hybrid/ml-dsa-65/ca.crt -nodes -days 365 -subj "/CN=Hybrid ML-DSA-65 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-DSA-65 -out /app/certs/hybrid/ml-dsa-65/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/hybrid/ml-dsa-65/server.key -out /app/certs/hybrid/ml-dsa-65/server.csr -config $CONFIG_FILE

# Sign server certificate with ML-DSA-65 CA
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-dsa-65/server.csr -CA /app/certs/hybrid/ml-dsa-65/ca.crt -CAkey /app/certs/hybrid/ml-dsa-65/ca.key -CAcreateserial -out /app/certs/hybrid/ml-dsa-65/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Also sign the same CSR with ECDSA CA to create a hybrid certificate chain
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-dsa-65/server.csr -CA /app/certs/hybrid/ml-dsa-65/ecdsa_ca.crt -CAkey /app/certs/hybrid/ml-dsa-65/ecdsa_ca.key -CAcreateserial -out /app/certs/hybrid/ml-dsa-65/server_ecdsa.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Create a combined certificate file (this is a simple concatenation for demonstration)
cat /app/certs/hybrid/ml-dsa-65/server.crt /app/certs/hybrid/ml-dsa-65/server_ecdsa.crt > /app/certs/hybrid/ml-dsa-65/server_hybrid.crt

echo "=== Generating Hybrid Certificate (ML-DSA-87 with ECDSA) ==="
# Generate ECDSA CA private key and certificate
${OPENSSL_BIN} ecparam -name secp521r1 -genkey -out /app/certs/hybrid/ml-dsa-87/ecdsa_ca.key
${OPENSSL_BIN} req -new -x509 -key /app/certs/hybrid/ml-dsa-87/ecdsa_ca.key -out /app/certs/hybrid/ml-dsa-87/ecdsa_ca.crt -days 365 -subj "/CN=ECDSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate ML-DSA-87 CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-DSA-87 -keyout /app/certs/hybrid/ml-dsa-87/ca.key -out /app/certs/hybrid/ml-dsa-87/ca.crt -nodes -days 365 -subj "/CN=Hybrid ML-DSA-87 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-DSA-87 -out /app/certs/hybrid/ml-dsa-87/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/hybrid/ml-dsa-87/server.key -out /app/certs/hybrid/ml-dsa-87/server.csr -config $CONFIG_FILE

# Sign server certificate with ML-DSA-87 CA
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-dsa-87/server.csr -CA /app/certs/hybrid/ml-dsa-87/ca.crt -CAkey /app/certs/hybrid/ml-dsa-87/ca.key -CAcreateserial -out /app/certs/hybrid/ml-dsa-87/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Also sign the same CSR with ECDSA CA to create a hybrid certificate chain
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-dsa-87/server.csr -CA /app/certs/hybrid/ml-dsa-87/ecdsa_ca.crt -CAkey /app/certs/hybrid/ml-dsa-87/ecdsa_ca.key -CAcreateserial -out /app/certs/hybrid/ml-dsa-87/server_ecdsa.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Create a combined certificate file (this is a simple concatenation for demonstration)
cat /app/certs/hybrid/ml-dsa-87/server.crt /app/certs/hybrid/ml-dsa-87/server_ecdsa.crt > /app/certs/hybrid/ml-dsa-87/server_hybrid.crt

echo "=== Generating Hybrid Certificate (ML-KEM-768 with X25519) ==="
# Generate X25519 CA private key and certificate
${OPENSSL_BIN} genpkey -algorithm X25519 -out /app/certs/hybrid/ml-kem-768/x25519_ca.key
${OPENSSL_BIN} req -new -x509 -key /app/certs/hybrid/ml-kem-768/x25519_ca.key -out /app/certs/hybrid/ml-kem-768/x25519_ca.crt -days 365 -subj "/CN=X25519 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate ML-KEM-768 CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-KEM-768 -keyout /app/certs/hybrid/ml-kem-768/ca.key -out /app/certs/hybrid/ml-kem-768/ca.crt -nodes -days 365 -subj "/CN=Hybrid ML-KEM-768 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-KEM-768 -out /app/certs/hybrid/ml-kem-768/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/hybrid/ml-kem-768/server.key -out /app/certs/hybrid/ml-kem-768/server.csr -config $CONFIG_FILE

# Sign server certificate with ML-KEM-768 CA
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-kem-768/server.csr -CA /app/certs/hybrid/ml-kem-768/ca.crt -CAkey /app/certs/hybrid/ml-kem-768/ca.key -CAcreateserial -out /app/certs/hybrid/ml-kem-768/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Also sign the same CSR with X25519 CA to create a hybrid certificate chain
${OPENSSL_BIN} x509 -req -in /app/certs/hybrid/ml-kem-768/server.csr -CA /app/certs/hybrid/ml-kem-768/x25519_ca.crt -CAkey /app/certs/hybrid/ml-kem-768/x25519_ca.key -CAcreateserial -out /app/certs/hybrid/ml-kem-768/server_x25519.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Create a combined certificate file (this is a simple concatenation for demonstration)
cat /app/certs/hybrid/ml-kem-768/server.crt /app/certs/hybrid/ml-kem-768/server_x25519.crt > /app/certs/hybrid/ml-kem-768/server_hybrid.crt

echo "=== Generating Post-Quantum Certificate (ML-DSA-44) ==="
# Generate CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-DSA-44 -keyout /app/certs/post-quantum/ml-dsa-44/ca.key -out /app/certs/post-quantum/ml-dsa-44/ca.crt -nodes -days 365 -subj "/CN=PQ ML-DSA-44 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-DSA-44 -out /app/certs/post-quantum/ml-dsa-44/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/post-quantum/ml-dsa-44/server.key -out /app/certs/post-quantum/ml-dsa-44/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
${OPENSSL_BIN} x509 -req -in /app/certs/post-quantum/ml-dsa-44/server.csr -CA /app/certs/post-quantum/ml-dsa-44/ca.crt -CAkey /app/certs/post-quantum/ml-dsa-44/ca.key -CAcreateserial -out /app/certs/post-quantum/ml-dsa-44/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Post-Quantum Certificate (ML-DSA-65) ==="
# Generate CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-DSA-65 -keyout /app/certs/post-quantum/ml-dsa-65/ca.key -out /app/certs/post-quantum/ml-dsa-65/ca.crt -nodes -days 365 -subj "/CN=PQ ML-DSA-65 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-DSA-65 -out /app/certs/post-quantum/ml-dsa-65/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/post-quantum/ml-dsa-65/server.key -out /app/certs/post-quantum/ml-dsa-65/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
${OPENSSL_BIN} x509 -req -in /app/certs/post-quantum/ml-dsa-65/server.csr -CA /app/certs/post-quantum/ml-dsa-65/ca.crt -CAkey /app/certs/post-quantum/ml-dsa-65/ca.key -CAcreateserial -out /app/certs/post-quantum/ml-dsa-65/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Post-Quantum Certificate (ML-DSA-87) ==="
# Generate CA private key and certificate
${OPENSSL_BIN} req -x509 -new -newkey ML-DSA-87 -keyout /app/certs/post-quantum/ml-dsa-87/ca.key -out /app/certs/post-quantum/ml-dsa-87/ca.crt -nodes -days 365 -subj "/CN=PQ ML-DSA-87 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
${OPENSSL_BIN} genpkey -algorithm ML-DSA-87 -out /app/certs/post-quantum/ml-dsa-87/server.key

# Generate certificate signing request (CSR)
${OPENSSL_BIN} req -new -key /app/certs/post-quantum/ml-dsa-87/server.key -out /app/certs/post-quantum/ml-dsa-87/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
${OPENSSL_BIN} x509 -req -in /app/certs/post-quantum/ml-dsa-87/server.csr -CA /app/certs/post-quantum/ml-dsa-87/ca.crt -CAkey /app/certs/post-quantum/ml-dsa-87/ca.key -CAcreateserial -out /app/certs/post-quantum/ml-dsa-87/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Verify certificate types
echo "=== Verifying Certificate Types ==="
echo "Traditional Certificate (RSA) algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/traditional/rsa/server.crt -text -noout | grep "Public Key Algorithm"

echo "Traditional Certificate (ECDSA) algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/traditional/ecdsa/server.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (ML-DSA-44 with ECDSA) PQC algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-dsa-44/server.crt -text -noout | grep "Public Key Algorithm"
echo "Hybrid Certificate (ML-DSA-44 with ECDSA) traditional algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-dsa-44/server_ecdsa.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (ML-DSA-65 with ECDSA) PQC algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-dsa-65/server.crt -text -noout | grep "Public Key Algorithm"
echo "Hybrid Certificate (ML-DSA-65 with ECDSA) traditional algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-dsa-65/server_ecdsa.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (ML-DSA-87 with ECDSA) PQC algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-dsa-87/server.crt -text -noout | grep "Public Key Algorithm"
echo "Hybrid Certificate (ML-DSA-87 with ECDSA) traditional algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-dsa-87/server_ecdsa.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (ML-KEM-768 with X25519) PQC algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-kem-768/server.crt -text -noout | grep "Public Key Algorithm"
echo "Hybrid Certificate (ML-KEM-768 with X25519) traditional algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/hybrid/ml-kem-768/server_x25519.crt -text -noout | grep "Public Key Algorithm"

echo "Post-Quantum Certificate (ML-DSA-44) algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/post-quantum/ml-dsa-44/server.crt -text -noout | grep "Public Key Algorithm"

echo "Post-Quantum Certificate (ML-DSA-65) algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/post-quantum/ml-dsa-65/server.crt -text -noout | grep "Public Key Algorithm"

echo "Post-Quantum Certificate (ML-DSA-87) algorithm:"
${OPENSSL_BIN} x509 -in /app/certs/post-quantum/ml-dsa-87/server.crt -text -noout | grep "Public Key Algorithm"

# Create symbolic links for backward compatibility
echo "=== Creating Symbolic Links for Backward Compatibility ==="
mkdir -p /app/certs/hybrid/dilithium3
mkdir -p /app/certs/hybrid/dilithium5
mkdir -p /app/certs/post-quantum/dilithium3
mkdir -p /app/certs/post-quantum/dilithium5

# Link ML-DSA-44 (equivalent to Dilithium2) to Dilithium3 for backward compatibility
ln -sf /app/certs/hybrid/ml-dsa-44/ca.crt /app/certs/hybrid/dilithium3/ca.crt
ln -sf /app/certs/hybrid/ml-dsa-44/ca.key /app/certs/hybrid/dilithium3/ca.key
ln -sf /app/certs/hybrid/ml-dsa-44/server_hybrid.crt /app/certs/hybrid/dilithium3/server.crt
ln -sf /app/certs/hybrid/ml-dsa-44/server.key /app/certs/hybrid/dilithium3/server.key

# Link ML-DSA-87 (equivalent to Dilithium5) to Dilithium5 for backward compatibility
ln -sf /app/certs/hybrid/ml-dsa-87/ca.crt /app/certs/hybrid/dilithium5/ca.crt
ln -sf /app/certs/hybrid/ml-dsa-87/ca.key /app/certs/hybrid/dilithium5/ca.key
ln -sf /app/certs/hybrid/ml-dsa-87/server_hybrid.crt /app/certs/hybrid/dilithium5/server.crt
ln -sf /app/certs/hybrid/ml-dsa-87/server.key /app/certs/hybrid/dilithium5/server.key

# Link post-quantum certificates for backward compatibility
ln -sf /app/certs/post-quantum/ml-dsa-44/ca.crt /app/certs/post-quantum/dilithium3/ca.crt
ln -sf /app/certs/post-quantum/ml-dsa-44/ca.key /app/certs/post-quantum/dilithium3/ca.key
ln -sf /app/certs/post-quantum/ml-dsa-44/server.crt /app/certs/post-quantum/dilithium3/server.crt
ln -sf /app/certs/post-quantum/ml-dsa-44/server.key /app/certs/post-quantum/dilithium3/server.key

ln -sf /app/certs/post-quantum/ml-dsa-87/ca.crt /app/certs/post-quantum/dilithium5/ca.crt
ln -sf /app/certs/post-quantum/ml-dsa-87/ca.key /app/certs/post-quantum/dilithium5/ca.key
ln -sf /app/certs/post-quantum/ml-dsa-87/server.crt /app/certs/post-quantum/dilithium5/server.crt
ln -sf /app/certs/post-quantum/ml-dsa-87/server.key /app/certs/post-quantum/dilithium5/server.key

echo "=== Certificate Generation Complete ==="
echo "Certificates have been saved to the following directories:"
echo "- Traditional Certificate (RSA): /app/certs/traditional/rsa/"
echo "- Traditional Certificate (ECDSA): /app/certs/traditional/ecdsa/"
echo "- Hybrid Certificate (ML-DSA-44 with ECDSA): /app/certs/hybrid/ml-dsa-44/"
echo "- Hybrid Certificate (ML-DSA-65 with ECDSA): /app/certs/hybrid/ml-dsa-65/"
echo "- Hybrid Certificate (ML-DSA-87 with ECDSA): /app/certs/hybrid/ml-dsa-87/"
echo "- Hybrid Certificate (ML-KEM-768 with X25519): /app/certs/hybrid/ml-kem-768/"
echo "- Post-Quantum Certificate (ML-DSA-44): /app/certs/post-quantum/ml-dsa-44/"
echo "- Post-Quantum Certificate (ML-DSA-65): /app/certs/post-quantum/ml-dsa-65/"
echo "- Post-Quantum Certificate (ML-DSA-87): /app/certs/post-quantum/ml-dsa-87/"
echo ""
echo "Backward compatibility links have been created for:"
echo "- Hybrid Certificate (Dilithium3) -> ML-DSA-44 with ECDSA P-256"
echo "- Hybrid Certificate (Dilithium5) -> ML-DSA-87 with ECDSA P-521"
echo "- Post-Quantum Certificate (Dilithium3) -> ML-DSA-44"
echo "- Post-Quantum Certificate (Dilithium5) -> ML-DSA-87"
