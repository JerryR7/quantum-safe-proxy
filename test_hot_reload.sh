#!/bin/bash
# 測試熱重載功能

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
    rm -f test_config_reload.json
    if [ ! -z "$SERVER_PID" ]; then
        kill -9 $SERVER_PID 2>/dev/null || true
    fi
}

# 確保在腳本結束時清理
trap cleanup EXIT

# 測試熱重載功能
test_hot_reload() {
    log_info "測試熱重載功能..."

    # 創建初始配置文件
    cat > test_config_reload.json << EOF
{
    "listen": "127.0.0.1:9999",
    "target": "127.0.0.1:6000",
    "log_level": "info",
    "buffer_size": 8192,
    "connection_timeout": 30,
    "strategy": "single",
    "traditional_cert": "certs/traditional/rsa/server.crt",
    "traditional_key": "certs/traditional/rsa/server.key",
    "hybrid_cert": "certs/hybrid/ml-dsa-87/server.crt",
    "hybrid_key": "certs/hybrid/ml-dsa-87/server.key",
    "client_ca_cert_path": "certs/hybrid/ml-dsa-87/ca.crt",
    "openssl_dir": "/usr/local/opt/openssl@3.5",
    "client_cert_mode": "optional"
}
EOF

    # 在後台啟動服務
    log_info "啟動服務..."
    cargo run --bin quantum-safe-proxy -- --config-file test_config_reload.json &
    SERVER_PID=$!

    # 等待服務啟動
    log_info "等待服務啟動 (5秒)..."
    sleep 5

    # 檢查服務是否正在運行
    if ! ps -p $SERVER_PID > /dev/null; then
        log_error "服務未能成功啟動"
        exit 1
    fi

    log_info "服務成功啟動，PID: $SERVER_PID"

    # 更新配置文件
    log_info "更新配置文件..."
    cat > test_config_reload.json << EOF
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

    # 發送 SIGHUP 信號觸發熱重載
    log_info "發送 SIGHUP 信號觸發熱重載..."
    kill -HUP $SERVER_PID

    # 等待重載完成
    log_info "等待重載完成 (5秒)..."
    sleep 5

    # 檢查服務是否仍在運行
    if ! ps -p $SERVER_PID > /dev/null; then
        log_error "熱重載後服務停止運行"
        exit 1
    fi

    log_info "熱重載成功，服務仍在運行"

    # 停止服務
    log_info "停止服務..."
    kill -TERM $SERVER_PID

    # 等待服務停止
    log_info "等待服務停止 (2秒)..."
    sleep 2

    log_info "熱重載測試完成"
}

# 運行測試
log_info "開始測試熱重載功能..."
test_hot_reload
log_info "測試完成"
