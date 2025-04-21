#!/bin/bash
# Docker entrypoint script for Quantum Safe Proxy
# This script checks for certificates and starts the proxy

set -e

# Check if certificates exist
if [ ! -d "/app/certs" ] || [ -z "$(ls -A /app/certs 2>/dev/null)" ]; then
    echo "Warning: Certificate directory is empty"

    # Auto-generate certificates if enabled
    if [ "$AUTO_GENERATE_CERTS" = "true" ]; then
        echo "AUTO_GENERATE_CERTS is set to true, generating certificates..."

        # Check if OpenSSL 3.5+ is available
        OPENSSL_VERSION=$(openssl version)
        if echo "$OPENSSL_VERSION" | grep -q "3.5"; then
            echo "Using OpenSSL 3.5+ to generate certificates"
            /app/scripts/generate-openssl35-certs.sh
        else
            echo "Using OQS-OpenSSL to generate certificates"
            /app/scripts/generate-certificates.sh
        fi
    else
        echo "Certificates not found. You can:"
        echo "  1. Mount a volume with certificates to /app/certs"
        echo "  2. Set AUTO_GENERATE_CERTS=true to generate certificates automatically"
        echo "  3. Run the certificate generation script manually:"
        echo "     - For OpenSSL 3.5+: /app/scripts/generate-openssl35-certs.sh"
        echo "     - For OQS-OpenSSL: /app/scripts/generate-certificates.sh"
    fi
fi

# Execute the command passed to the script
if [ "$1" = "--listen" ] || [ "$1" = "--config" ] || [ "$1" = "--help" ] || [ "$1" = "--version" ] || [ "$1" = "--target" ] || [ "$1" = "--cert" ]; then
    # If the first argument starts with --, prepend the binary name
    exec /usr/local/bin/quantum-safe-proxy "$@"
else
    # Otherwise, execute as is
    exec "$@"
fi
