#!/bin/bash

# Compile the detailed non-TLS client
echo "Compiling detailed non-TLS client..."
cargo build --bin quantum-safe-proxy

# Create a temporary Cargo.toml for the test client
cat > tests/Cargo.toml << EOF
[package]
name = "detailed_non_tls_client"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "detailed_non_tls_client"
path = "detailed_non_tls_client.rs"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
EOF

# Compile the detailed client
cd tests && cargo build && cd ..

# Start the proxy server in the background
echo "Starting proxy server..."
cargo run &
PROXY_PID=$!

# Wait for the proxy to start
echo "Waiting for proxy to start..."
sleep 3

# Run the detailed non-TLS client
echo "Running detailed non-TLS client..."
./tests/target/debug/detailed_non_tls_client

# Capture the exit code
CLIENT_EXIT_CODE=$?
echo "Detailed non-TLS client exited with code: $CLIENT_EXIT_CODE"

# Kill the proxy server
echo "Stopping proxy server..."
kill $PROXY_PID

# Clean up
rm -rf tests/Cargo.toml tests/Cargo.lock tests/target

echo "Test completed"
