#!/bin/bash
# OQS-OpenSSL 1.1.1 Installation Script
# This script installs OQS-OpenSSL 1.1.1 with post-quantum cryptography support
#
# NOTE: This script installs the legacy OpenSSL 1.1.1 version with OQS patches.
# For new projects, it is recommended to use OpenSSL 3.x with OQS Provider instead.
# Please use the install-oqs-provider.sh script for OpenSSL 3.x with OQS Provider.

set -e

# Default installation directory
INSTALL_DIR="/opt/oqs-openssl"
OQS_BRANCH="OQS-OpenSSL_1_1_1-stable"
OQS_REPO="https://github.com/open-quantum-safe/openssl.git"

# Print usage information
function print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo "Install OQS-OpenSSL with post-quantum cryptography support"
    echo ""
    echo "Options:"
    echo "  -d, --dir DIR       Installation directory (default: $INSTALL_DIR)"
    echo "  -b, --branch BRANCH OQS-OpenSSL branch to use (default: $OQS_BRANCH)"
    echo "  -h, --help          Display this help message"
    echo ""
    echo "Example:"
    echo "  $0 --dir /usr/local/oqs-openssl"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        -b|--branch)
            OQS_BRANCH="$2"
            shift 2
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

echo "=== OQS-OpenSSL 1.1.1 Installation ==="
echo "WARNING: This installs the legacy OpenSSL 1.1.1 version with OQS patches."
echo "For new projects, consider using OpenSSL 3.x with OQS Provider instead."
echo ""
echo "Installation directory: $INSTALL_DIR"
echo "OQS-OpenSSL branch: $OQS_BRANCH"
echo ""

# Check if installation directory already exists
if [ -d "$INSTALL_DIR" ]; then
    echo "Installation directory already exists."
    read -p "Do you want to remove it and continue? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation aborted."
        exit 1
    fi
    echo "Removing existing installation directory..."
    rm -rf "$INSTALL_DIR"
fi

# Check for required tools
echo "Checking for required tools..."
for tool in git make gcc; do
    if ! command -v $tool &> /dev/null; then
        echo "Error: $tool is not installed. Please install it and try again."
        exit 1
    fi
done

# Create temporary directory
TEMP_DIR=$(mktemp -d)
echo "Created temporary directory: $TEMP_DIR"

# Clone OQS-OpenSSL repository
echo "Cloning OQS-OpenSSL repository..."
git clone --branch "$OQS_BRANCH" "$OQS_REPO" "$TEMP_DIR/openssl"
cd "$TEMP_DIR/openssl"

# Configure OQS-OpenSSL
echo "Configuring OQS-OpenSSL..."
./config --prefix="$INSTALL_DIR" shared

# Build OQS-OpenSSL
echo "Building OQS-OpenSSL (this may take a while)..."
make -j$(nproc)

# Install OQS-OpenSSL
echo "Installing OQS-OpenSSL..."
mkdir -p "$INSTALL_DIR"
make install

# Clean up
echo "Cleaning up..."
cd - > /dev/null
rm -rf "$TEMP_DIR"

# Set environment variables
echo "Setting up environment variables..."
cat > "$INSTALL_DIR/env.sh" << EOF
#!/bin/bash
# OQS-OpenSSL environment variables
export PATH="$INSTALL_DIR/bin:\$PATH"
export LD_LIBRARY_PATH="$INSTALL_DIR/lib:\$LD_LIBRARY_PATH"
export OQS_OPENSSL_PATH="$INSTALL_DIR"
EOF

chmod +x "$INSTALL_DIR/env.sh"

echo ""
echo "=== Installation Complete ==="
echo "OQS-OpenSSL 1.1.1 has been installed to $INSTALL_DIR"
echo ""
echo "NOTE: This is the legacy OpenSSL 1.1.1 version with OQS patches."
echo "For new projects, consider using OpenSSL 3.x with OQS Provider instead."
echo "Run ./scripts/install-oqs-provider.sh to install OpenSSL 3.x with OQS Provider."
echo ""
echo "To use OQS-OpenSSL, run:"
echo "  source $INSTALL_DIR/env.sh"
echo ""
echo "To make this permanent, add the following to your ~/.bashrc or ~/.zshrc:"
echo "  export PATH=\"$INSTALL_DIR/bin:\$PATH\""
echo "  export LD_LIBRARY_PATH=\"$INSTALL_DIR/lib:\$LD_LIBRARY_PATH\""
echo "  export OQS_OPENSSL_PATH=\"$INSTALL_DIR\""
echo ""
echo "To verify the installation, run:"
echo "  $INSTALL_DIR/bin/openssl version"
echo ""
