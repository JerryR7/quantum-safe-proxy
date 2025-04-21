#!/bin/bash
set -e

OPENSSL_VERSION=3.5.0
INSTALL_DIR="/opt/openssl-${OPENSSL_VERSION}"
BUILD_DIR="/tmp/openssl-build-${OPENSSL_VERSION}"
PKGCONFIG_DIR="${INSTALL_DIR}/lib/pkgconfig"

echo "==> [1/8] 安裝建構工具"
sudo apt update && sudo apt install -y build-essential curl wget make pkg-config

echo "==> [2/8] 準備乾淨 build 環境"
rm -rf ${BUILD_DIR}
mkdir -p ${BUILD_DIR}
cd ${BUILD_DIR}
wget https://www.openssl.org/source/openssl-${OPENSSL_VERSION}.tar.gz
tar -xzf openssl-${OPENSSL_VERSION}.tar.gz
cd openssl-${OPENSSL_VERSION}

echo "==> [3/8] Configure（enable-shared + rpath）"
./Configure linux-x86_64 \
  --prefix=${INSTALL_DIR} \
  enable-shared \
  -Wl,-rpath=${INSTALL_DIR}/lib

echo "==> [4/8] 編譯並安裝"
make clean
make -j$(nproc)
sudo make install

echo "==> [5/8] 手動補 `.so.3` 檔案（避免安裝漏裝）"
sudo mkdir -p ${INSTALL_DIR}/lib
sudo cp libssl.so* ${INSTALL_DIR}/lib/
sudo cp libcrypto.so* ${INSTALL_DIR}/lib/

echo "==> [6/8] 建立 openssl35 指令"
sudo tee /usr/local/bin/openssl35 > /dev/null <<EOF
#!/bin/bash
LD_LIBRARY_PATH=${INSTALL_DIR}/lib ${INSTALL_DIR}/bin/openssl "\$@"
EOF
sudo chmod +x /usr/local/bin/openssl35

echo "==> [7/8] 建立 openssl.pc（pkg-config 專用）"
sudo mkdir -p ${PKGCONFIG_DIR}
sudo tee ${PKGCONFIG_DIR}/openssl.pc > /dev/null <<EOF
prefix=${INSTALL_DIR}
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: OpenSSL
Description: Secure Sockets Layer and cryptography libraries
Version: ${OPENSSL_VERSION}
Requires:
Libs: -L\${libdir} -lssl -lcrypto
Cflags: -I\${includedir}
EOF

echo "==> [8/8] 設定 PKG_CONFIG_PATH（永久生效）"
CONFIG_LINE="export PKG_CONFIG_PATH=${PKGCONFIG_DIR}:\$PKG_CONFIG_PATH"
if ! grep -Fxq "${CONFIG_LINE}" ~/.bashrc; then
  echo "${CONFIG_LINE}" >> ~/.bashrc
  echo "✔ 已寫入 ~/.bashrc"
else
  echo "✔ 已存在 ~/.bashrc，略過"
fi

echo
echo "✅ 安裝完成！你現在可以使用："
echo "   openssl35 version"
echo "   pkg-config --modversion openssl"
echo
echo "✅ 若為 Rust 專案，可直接讓 openssl-sys 自動抓取新版 OpenSSL"
echo
