#!/bin/bash

# Compile the non-TLS client
echo "Compiling non-TLS client..."
cargo build --bin quantum-safe-proxy
rustc -o non_tls_client tests/non_tls_client.rs

# Start the proxy server in the background
echo "Starting proxy server..."
cargo run --bin quantum-safe-proxy -- --listen 0.0.0.0:8443 --target 127.0.0.1:6000 --hybrid-cert certs/hybrid/ml-dsa-87/hybrid.crt --hybrid-key certs/hybrid/ml-dsa-87/hybrid.key --traditional-cert certs/traditional/rsa/server.crt --traditional-key certs/traditional/rsa/privkey.pem --client-ca-cert certs/hybrid/ml-dsa-87/fullchain.pem &
PROXY_PID=$!

# Wait for the proxy to start
echo "Waiting for proxy to start..."
sleep 3

# Run the non-TLS client
echo "Running non-TLS client..."
./non_tls_client

# Capture the exit code
CLIENT_EXIT_CODE=$?
echo "Non-TLS client exited with code: $CLIENT_EXIT_CODE"

# Kill the proxy server
echo "Stopping proxy server..."
kill $PROXY_PID

# Clean up
rm -f non_tls_client

echo "Test completed"
