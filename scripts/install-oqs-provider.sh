#!/bin/bash
# ============================================================================
# OpenSSL 3.x with OQS Provider Installation Script (RECOMMENDED)
# ============================================================================
#
# DESCRIPTION:
#   This script installs OpenSSL 3.x with OQS Provider for post-quantum cryptography.
#   This is the RECOMMENDED installation method for new projects as it uses the
#   modern OpenSSL 3.x architecture with pluggable providers.
#
# USAGE:
#   ./scripts/install-oqs-provider.sh [OPTIONS]
#
# OPTIONS:
#   -d, --dir DIR       Installation directory (default: /opt/oqs)
#   --openssl VERSION   OpenSSL version (default: 3.4.0)
#   --liboqs VERSION    liboqs version (default: 0.12.0)
#   --oqsprovider VERSION  OQS Provider version (default: 0.8.0)
#   -h, --help          Display help message
#
# REQUIREMENTS:
#   - git, make, gcc, cmake, ninja-build, pkg-config
#
# OUTPUT:
#   - Installs OpenSSL 3.x with OQS Provider to the specified directory
#   - Creates environment setup script at <INSTALL_DIR>/env.sh

set -e

# Default installation directory and versions
INSTALLDIR="/opt/oqs"
OPENSSL_TAG="3.4.0"
LIBOQS_TAG="0.12.0"
OQSPROVIDER_TAG="0.8.0"

# Print usage information
function print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo "Install OpenSSL 3.x with OQS Provider for post-quantum cryptography"
    echo ""
    echo "Options:"
    echo "  -d, --dir DIR       Installation directory (default: $INSTALLDIR)"
    echo "  --openssl VERSION   OpenSSL version (default: $OPENSSL_TAG)"
    echo "  --liboqs VERSION    liboqs version (default: $LIBOQS_TAG)"
    echo "  --oqsprovider VERSION  OQS Provider version (default: $OQSPROVIDER_TAG)"
    echo "  -h, --help          Display this help message"
    echo ""
    echo "Example:"
    echo "  $0 --dir /usr/local/oqs --openssl 3.4.0 --liboqs 0.12.0 --oqsprovider 0.8.0"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--dir)
            INSTALLDIR="$2"
            shift 2
            ;;
        --openssl)
            OPENSSL_TAG="$2"
            shift 2
            ;;
        --liboqs)
            LIBOQS_TAG="$2"
            shift 2
            ;;
        --oqsprovider)
            OQSPROVIDER_TAG="$2"
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

echo "=== OpenSSL 3.x with OQS Provider Installation ==="
echo "Installation directory: $INSTALLDIR"
echo "OpenSSL version: $OPENSSL_TAG"
echo "liboqs version: $LIBOQS_TAG"
echo "OQS Provider version: $OQSPROVIDER_TAG"
echo ""

# Check if installation directory already exists
if [ -d "$INSTALLDIR" ]; then
    echo "Installation directory already exists."
    read -p "Do you want to remove it and continue? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation aborted."
        exit 1
    fi
    echo "Removing existing installation directory..."
    rm -rf "$INSTALLDIR"
fi

# Check for required tools
echo "Checking for required tools..."
for tool in git make gcc cmake ninja-build pkg-config; do
    if ! command -v $tool &> /dev/null; then
        echo "Error: $tool is not installed. Please install it and try again."
        exit 1
    fi
done

# Create temporary directory
TEMP_DIR=$(mktemp -d)
echo "Created temporary directory: $TEMP_DIR"

# Create installation directories
mkdir -p "$INSTALLDIR/lib" "$INSTALLDIR/bin" "$INSTALLDIR/ssl"

# Clone and build liboqs
echo "Building liboqs..."
git clone --depth 1 --branch $LIBOQS_TAG https://github.com/open-quantum-safe/liboqs.git "$TEMP_DIR/liboqs"
mkdir -p "$TEMP_DIR/build-liboqs"
cd "$TEMP_DIR/build-liboqs"
cmake -G Ninja "$TEMP_DIR/liboqs" -DCMAKE_INSTALL_PREFIX="$INSTALLDIR/liboqs" -DBUILD_SHARED_LIBS=ON -DOQS_USE_OPENSSL=OFF -DCMAKE_INSTALL_RPATH="$INSTALLDIR/liboqs/lib"
ninja -j$(nproc)
ninja install

# Clone and build OpenSSL
echo "Building OpenSSL..."
git clone --depth 1 --branch openssl-$OPENSSL_TAG https://github.com/openssl/openssl.git "$TEMP_DIR/openssl"
cd "$TEMP_DIR/openssl"
LDFLAGS="-Wl,-rpath,$INSTALLDIR/liboqs/lib" ./config --prefix="$INSTALLDIR/openssl" --openssldir="$INSTALLDIR/ssl" shared
make -j$(nproc)
make install_sw install_ssldirs

# Create pkg-config file for OpenSSL
mkdir -p "$INSTALLDIR/openssl/lib/pkgconfig"
cat > "$INSTALLDIR/openssl/lib/pkgconfig/openssl.pc" << EOF
prefix=$INSTALLDIR/openssl
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib64
includedir=\${prefix}/include

Name: OpenSSL
Description: Secure Sockets Layer and cryptography libraries and tools
Version: $OPENSSL_TAG
Libs: -L\${libdir} -lssl -lcrypto
Cflags: -I\${includedir}
EOF

# Handle lib64 directory if needed
if [ -d "$INSTALLDIR/openssl/lib64" ]; then
    ln -s "$INSTALLDIR/openssl/lib64" "$INSTALLDIR/openssl/lib"
fi
if [ -d "$INSTALLDIR/openssl/lib" ]; then
    ln -s "$INSTALLDIR/openssl/lib" "$INSTALLDIR/openssl/lib64"
fi

# Clone and build OQS Provider
echo "Building OQS Provider..."
git clone --depth 1 --branch $OQSPROVIDER_TAG https://github.com/open-quantum-safe/oqs-provider.git "$TEMP_DIR/oqs-provider"
mkdir -p "$TEMP_DIR/build-oqs-provider"
cd "$TEMP_DIR/build-oqs-provider"
cmake -G Ninja -DOPENSSL_ROOT_DIR="$INSTALLDIR/openssl" -DCMAKE_PREFIX_PATH="$INSTALLDIR/openssl;$INSTALLDIR/liboqs" -DCMAKE_INSTALL_PREFIX="$INSTALLDIR/oqs-provider" -DCMAKE_INSTALL_RPATH="$INSTALLDIR/openssl/lib:$INSTALLDIR/liboqs/lib" "$TEMP_DIR/oqs-provider"
ninja -j$(nproc)
mkdir -p "$INSTALLDIR/openssl/lib64/ossl-modules"
cp "$TEMP_DIR/build-oqs-provider/lib/oqsprovider.so" "$INSTALLDIR/openssl/lib64/ossl-modules/"

# Set up OpenSSL to load the OQS provider
CONFIG_FILE="$INSTALLDIR/ssl/openssl.cnf"
sed -i "s/default = default_sect/default = default_sect\noqsprovider = oqsprovider_sect/g" "$CONFIG_FILE"
sed -i "s/\[default_sect\]/\[default_sect\]\nactivate = 1\n\[oqsprovider_sect\]\nactivate = 1\n/g" "$CONFIG_FILE"

# Create a verification script
cat > "$INSTALLDIR/test-oqs.sh" << 'EOF'
#!/bin/sh
echo "Testing OpenSSL with OQS Provider"
INSTALLDIR=$(dirname "$0")
$INSTALLDIR/openssl/bin/openssl list -providers
$INSTALLDIR/openssl/bin/openssl list -signature-algorithms | grep -i dilithium
$INSTALLDIR/openssl/bin/openssl list -key-exchange-algorithms | grep -i kyber
echo "OQS Provider test completed"
EOF
chmod +x "$INSTALLDIR/test-oqs.sh"

# Create environment setup script
cat > "$INSTALLDIR/env.sh" << EOF
#!/bin/bash
# Environment setup for OpenSSL 3.x with OQS Provider
export PATH="$INSTALLDIR/openssl/bin:\$PATH"
export LD_LIBRARY_PATH="$INSTALLDIR/openssl/lib64:$INSTALLDIR/liboqs/lib:\$LD_LIBRARY_PATH"
export OPENSSL_DIR="$INSTALLDIR/openssl"
export OPENSSL_LIB_DIR="$INSTALLDIR/openssl/lib64"
export OPENSSL_INCLUDE_DIR="$INSTALLDIR/openssl/include"
export PKG_CONFIG_PATH="$INSTALLDIR/openssl/lib/pkgconfig:\$PKG_CONFIG_PATH"
export OQS_OPENSSL_PATH="$INSTALLDIR/openssl"
EOF
chmod +x "$INSTALLDIR/env.sh"

# Clean up
echo "Cleaning up..."
rm -rf "$TEMP_DIR"

# Test the installation
echo "Testing the installation..."
"$INSTALLDIR/test-oqs.sh"

echo ""
echo "=== Installation Complete ==="
echo "OpenSSL 3.x with OQS Provider has been installed to $INSTALLDIR"
echo ""
echo "To use OpenSSL with OQS Provider, run:"
echo "  source $INSTALLDIR/env.sh"
echo ""
echo "To make this permanent, add the following to your ~/.bashrc or ~/.zshrc:"
echo "  source $INSTALLDIR/env.sh"
echo ""
echo "To verify the installation, run:"
echo "  $INSTALLDIR/test-oqs.sh"
echo ""
