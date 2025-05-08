#!/bin/bash
# 測試配置文件讀取

set -e

# 顏色定義
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 輔助函數
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 清理函數
cleanup() {
    log_info "清理測試環境..."
    rm -f test_config.json
}

# 確保在腳本結束時清理
trap cleanup EXIT

# 測試配置文件讀取
test_config_file() {
    log_info "測試配置文件讀取..."

    # 創建測試配置文件
    cat > test_config.json << EOF
{
    "listen": "127.0.0.1:9999",
    "target": "127.0.0.1:6000",
    "log_level": "debug",
    "buffer_size": 16384,
    "connection_timeout": 60,
    "strategy": "sigalgs",
    "traditional_cert": "certs/traditional/rsa/server.crt",
    "traditional_key": "certs/traditional/rsa/server.key",
    "hybrid_cert": "certs/hybrid/ml-dsa-87/server.crt",
    "hybrid_key": "certs/hybrid/ml-dsa-87/server.key",
    "client_ca_cert_path": "certs/hybrid/ml-dsa-87/ca.crt",
    "openssl_dir": "/usr/local/opt/openssl@3.5",
    "client_cert_mode": "optional"
}
EOF

    # 設置環境變量
    log_info "設置環境變量..."
    export QUANTUM_SAFE_PROXY_LISTEN="127.0.0.1:8888"

    # 運行程序，使用測試配置文件和命令行參數
    log_info "運行程序，使用測試配置文件和命令行參數..."
    RUST_LOG=debug cargo run --bin quantum-safe-proxy -- --config-file test_config.json --listen 127.0.0.1:7777
}

# 運行測試
log_info "開始測試配置文件讀取..."
test_config_file
log_info "測試完成"
