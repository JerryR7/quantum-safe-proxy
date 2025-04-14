#!/bin/bash
# ============================================================================
# Generate Comprehensive Certificate Set for Quantum Safe Proxy
# ============================================================================
#
# DESCRIPTION:
#   This is the main certificate generation script that creates a complete set of
#   certificates for all supported algorithms. It generates traditional, hybrid,
#   and pure post-quantum certificates for thorough testing and production use.
#
# USAGE:
#   docker compose exec quantum-safe-proxy /scripts/generate_certificates.sh
#
# REQUIREMENTS:
#   - Must be run inside the Docker container with OQS-OpenSSL installed
#   - Requires OpenSSL with OQS Provider support
#
# CERTIFICATE TYPES GENERATED:
#   - Traditional certificates (RSA, ECDSA)
#   - Hybrid certificates (Dilithium3+ECDSA, Dilithium5/ML-DSA-87+ECDSA, Falcon-1024+ECDSA)
#   - Pure post-quantum certificates (Dilithium3, Dilithium5/ML-DSA-87, Falcon-1024)
#
# OUTPUT:
#   Creates a complete directory structure with all certificate types in /app/certs/

set -e

# Create base directories
mkdir -p /app/certs/traditional/rsa
mkdir -p /app/certs/traditional/ecdsa
mkdir -p /app/certs/hybrid/dilithium3
mkdir -p /app/certs/hybrid/dilithium5
mkdir -p /app/certs/hybrid/falcon1024
mkdir -p /app/certs/post-quantum/dilithium3
mkdir -p /app/certs/post-quantum/dilithium5
mkdir -p /app/certs/post-quantum/falcon1024
mkdir -p /app/certs/config

# Create configuration file in the certs directory
# Note: This configuration is similar to scripts/openssl-hybrid.conf but is generated
# automatically by this script for use with the certificate generation commands below.
# If you need to manually generate certificates, use scripts/openssl-hybrid.conf instead.
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
/opt/oqs/openssl/bin/openssl genrsa -out /app/certs/traditional/rsa/ca.key 3072
/opt/oqs/openssl/bin/openssl req -new -x509 -key /app/certs/traditional/rsa/ca.key -out /app/certs/traditional/rsa/ca.crt -days 365 -subj "/CN=Traditional RSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genrsa -out /app/certs/traditional/rsa/server.key 2048

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/traditional/rsa/server.key -out /app/certs/traditional/rsa/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/traditional/rsa/server.csr -CA /app/certs/traditional/rsa/ca.crt -CAkey /app/certs/traditional/rsa/ca.key -CAcreateserial -out /app/certs/traditional/rsa/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Traditional Certificate (ECDSA) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl ecparam -name prime256v1 -genkey -out /app/certs/traditional/ecdsa/ca.key
/opt/oqs/openssl/bin/openssl req -new -x509 -key /app/certs/traditional/ecdsa/ca.key -out /app/certs/traditional/ecdsa/ca.crt -days 365 -subj "/CN=Traditional ECDSA CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl ecparam -name prime256v1 -genkey -out /app/certs/traditional/ecdsa/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/traditional/ecdsa/server.key -out /app/certs/traditional/ecdsa/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/traditional/ecdsa/server.csr -CA /app/certs/traditional/ecdsa/ca.crt -CAkey /app/certs/traditional/ecdsa/ca.key -CAcreateserial -out /app/certs/traditional/ecdsa/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Hybrid Certificate (Dilithium3 + ECDSA) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl req -x509 -new -newkey dilithium3 -keyout /app/certs/hybrid/dilithium3/ca.key -out /app/certs/hybrid/dilithium3/ca.crt -nodes -days 365 -subj "/CN=Hybrid Dilithium3 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genpkey -algorithm dilithium3 -out /app/certs/hybrid/dilithium3/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/hybrid/dilithium3/server.key -out /app/certs/hybrid/dilithium3/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/hybrid/dilithium3/server.csr -CA /app/certs/hybrid/dilithium3/ca.crt -CAkey /app/certs/hybrid/dilithium3/ca.key -CAcreateserial -out /app/certs/hybrid/dilithium3/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Hybrid Certificate (Dilithium5/ML-DSA-87 + ECDSA) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl req -x509 -new -newkey dilithium5 -keyout /app/certs/hybrid/dilithium5/ca.key -out /app/certs/hybrid/dilithium5/ca.crt -nodes -days 365 -subj "/CN=Hybrid Dilithium5 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genpkey -algorithm dilithium5 -out /app/certs/hybrid/dilithium5/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/hybrid/dilithium5/server.key -out /app/certs/hybrid/dilithium5/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/hybrid/dilithium5/server.csr -CA /app/certs/hybrid/dilithium5/ca.crt -CAkey /app/certs/hybrid/dilithium5/ca.key -CAcreateserial -out /app/certs/hybrid/dilithium5/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Hybrid Certificate (Falcon-1024 + ECDSA) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl req -x509 -new -newkey falcon1024 -keyout /app/certs/hybrid/falcon1024/ca.key -out /app/certs/hybrid/falcon1024/ca.crt -nodes -days 365 -subj "/CN=Hybrid Falcon1024 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genpkey -algorithm falcon1024 -out /app/certs/hybrid/falcon1024/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/hybrid/falcon1024/server.key -out /app/certs/hybrid/falcon1024/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/hybrid/falcon1024/server.csr -CA /app/certs/hybrid/falcon1024/ca.crt -CAkey /app/certs/hybrid/falcon1024/ca.key -CAcreateserial -out /app/certs/hybrid/falcon1024/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Post-Quantum Certificate (Dilithium3) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl req -x509 -new -newkey dilithium3 -keyout /app/certs/post-quantum/dilithium3/ca.key -out /app/certs/post-quantum/dilithium3/ca.crt -nodes -days 365 -subj "/CN=PQ Dilithium3 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genpkey -algorithm dilithium3 -out /app/certs/post-quantum/dilithium3/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/post-quantum/dilithium3/server.key -out /app/certs/post-quantum/dilithium3/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/post-quantum/dilithium3/server.csr -CA /app/certs/post-quantum/dilithium3/ca.crt -CAkey /app/certs/post-quantum/dilithium3/ca.key -CAcreateserial -out /app/certs/post-quantum/dilithium3/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Post-Quantum Certificate (Dilithium5/ML-DSA-87) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl req -x509 -new -newkey dilithium5 -keyout /app/certs/post-quantum/dilithium5/ca.key -out /app/certs/post-quantum/dilithium5/ca.crt -nodes -days 365 -subj "/CN=PQ Dilithium5 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genpkey -algorithm dilithium5 -out /app/certs/post-quantum/dilithium5/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/post-quantum/dilithium5/server.key -out /app/certs/post-quantum/dilithium5/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/post-quantum/dilithium5/server.csr -CA /app/certs/post-quantum/dilithium5/ca.crt -CAkey /app/certs/post-quantum/dilithium5/ca.key -CAcreateserial -out /app/certs/post-quantum/dilithium5/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

echo "=== Generating Post-Quantum Certificate (Falcon-1024) ==="
# Generate CA private key and certificate
/opt/oqs/openssl/bin/openssl req -x509 -new -newkey falcon1024 -keyout /app/certs/post-quantum/falcon1024/ca.key -out /app/certs/post-quantum/falcon1024/ca.crt -nodes -days 365 -subj "/CN=PQ Falcon1024 CA/O=Quantum Safe Proxy/OU=Testing/C=TW" -config $CONFIG_FILE -extensions v3_ca

# Generate server private key
/opt/oqs/openssl/bin/openssl genpkey -algorithm falcon1024 -out /app/certs/post-quantum/falcon1024/server.key

# Generate certificate signing request (CSR)
/opt/oqs/openssl/bin/openssl req -new -key /app/certs/post-quantum/falcon1024/server.key -out /app/certs/post-quantum/falcon1024/server.csr -config $CONFIG_FILE

# Sign server certificate with CA
/opt/oqs/openssl/bin/openssl x509 -req -in /app/certs/post-quantum/falcon1024/server.csr -CA /app/certs/post-quantum/falcon1024/ca.crt -CAkey /app/certs/post-quantum/falcon1024/ca.key -CAcreateserial -out /app/certs/post-quantum/falcon1024/server.crt -days 365 -extensions v3_req -extfile $CONFIG_FILE

# Verify certificate types
echo "=== Verifying Certificate Types ==="
echo "Traditional Certificate (RSA) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/traditional/rsa/server.crt -text -noout | grep "Public Key Algorithm"

echo "Traditional Certificate (ECDSA) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/traditional/ecdsa/server.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (Dilithium3) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/hybrid/dilithium3/server.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (Dilithium5/ML-DSA-87) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/hybrid/dilithium5/server.crt -text -noout | grep "Public Key Algorithm"

echo "Hybrid Certificate (Falcon-1024) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/hybrid/falcon1024/server.crt -text -noout | grep "Public Key Algorithm"

echo "Post-Quantum Certificate (Dilithium3) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/post-quantum/dilithium3/server.crt -text -noout | grep "Public Key Algorithm"

echo "Post-Quantum Certificate (Dilithium5/ML-DSA-87) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/post-quantum/dilithium5/server.crt -text -noout | grep "Public Key Algorithm"

echo "Post-Quantum Certificate (Falcon-1024) algorithm:"
/opt/oqs/openssl/bin/openssl x509 -in /app/certs/post-quantum/falcon1024/server.crt -text -noout | grep "Public Key Algorithm"

echo "=== Certificate Generation Complete ==="
echo "Certificates have been saved to the following directories:"
echo "- Traditional Certificate (RSA): /app/certs/traditional/rsa/"
echo "- Traditional Certificate (ECDSA): /app/certs/traditional/ecdsa/"
echo "- Hybrid Certificate (Dilithium3): /app/certs/hybrid/dilithium3/"
echo "- Hybrid Certificate (Dilithium5/ML-DSA-87): /app/certs/hybrid/dilithium5/"
echo "- Hybrid Certificate (Falcon-1024): /app/certs/hybrid/falcon1024/"
echo "- Post-Quantum Certificate (Dilithium3): /app/certs/post-quantum/dilithium3/"
echo "- Post-Quantum Certificate (Dilithium5/ML-DSA-87): /app/certs/post-quantum/dilithium5/"
echo "- Post-Quantum Certificate (Falcon-1024): /app/certs/post-quantum/falcon1024/"
