#!/bin/bash
# Generate test certificates for Quantum Safe Proxy
# This script generates test certificates using OpenSSL with OQS Provider

set -e

# Check if OpenSSL with OQS Provider is available
if ! openssl list -providers 2>/dev/null | grep -q "oqsprovider"; then
    echo "Error: OpenSSL with OQS Provider not found."
    echo "Please install OpenSSL with OQS Provider first:"
    echo "  ./scripts/install-oqs-provider.sh"
    exit 1
fi

# Create certificates directory
mkdir -p certs
cd certs

# Create CA configuration file
cat > ca.cnf << EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_ca
prompt = no

[req_distinguished_name]
CN = Quantum Safe CA
O = Quantum Safe Proxy
OU = Testing
C = TW

[v3_ca]
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer:always
basicConstraints = critical, CA:true
keyUsage = critical, digitalSignature, cRLSign, keyCertSign
EOF

# Create server certificate configuration file
cat > server.cnf << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = quantum-safe-proxy.local
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
DNS.2 = quantum-safe-proxy.local
IP.1 = 127.0.0.1
EOF

# Create client certificate configuration file
cat > client.cnf << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = quantum-safe-client
O = Quantum Safe Proxy
OU = Testing
C = TW

[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
EOF

echo "Generating CA certificate (Dilithium + ECDSA hybrid)..."
openssl req -x509 -new -newkey dilithium3 -keyout ca.key -out ca.crt -nodes -days 365 -config ca.cnf
echo "CA certificate generated."

echo "Generating server certificate (Kyber + ECDSA hybrid)..."
openssl req -new -newkey kyber768 -keyout server.key -out server.csr -nodes -config server.cnf
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365 -extensions v3_req -extfile server.cnf
echo "Server certificate generated."

echo "Generating client certificate (Kyber + ECDSA hybrid)..."
openssl req -new -newkey kyber768 -keyout client.key -out client.csr -nodes -config client.cnf
openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out client.crt -days 365 -extensions v3_req -extfile client.cnf
echo "Client certificate generated."

echo "Verifying certificates..."
openssl x509 -in ca.crt -text -noout | grep "Issuer\|Subject\|Public Key Algorithm"
echo ""
openssl x509 -in server.crt -text -noout | grep "Issuer\|Subject\|Public Key Algorithm"
echo ""
openssl x509 -in client.crt -text -noout | grep "Issuer\|Subject\|Public Key Algorithm"

echo "Test certificates generated successfully in the 'certs' directory."
