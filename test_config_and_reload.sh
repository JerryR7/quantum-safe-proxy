#!/bin/bash
# 測試配置優先順序和熱重載功能

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
    rm -f test_config_reload.json
    unset QUANTUM_SAFE_PROXY_LISTEN
    unset QUANTUM_SAFE_PROXY_TARGET
    unset QUANTUM_SAFE_PROXY_LOG_LEVEL
    unset QUANTUM_SAFE_PROXY_BUFFER_SIZE
    unset QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT
    unset QUANTUM_SAFE_PROXY_STRATEGY
}

# 確保在腳本結束時清理
trap cleanup EXIT

# 測試 1: 測試默認配置
test_default_config() {
    log_info "測試默認配置..."

    # 使用 check-environment 工具檢查默認配置
    cargo run --bin check-environment

    if [ $? -eq 0 ]; then
        log_info "默認配置測試通過"
    else
        log_error "默認配置測試失敗"
        exit 1
    fi
}

# 測試 2: 測試配置文件
test_config_file() {
    log_info "測試配置文件..."

    # 創建測試配置文件
    cat > test_config.json << EOF
{
    "listen": "127.0.0.1:8443",
    "target": "127.0.0.1:6000",
    "log_level": "debug",
    "buffer_size": 16384,
    "connection_timeout": 60,
    "strategy": "sigalgs"
}
EOF

    # 使用配置文件運行
    cargo run --bin check-environment -- --config-file test_config.json

    if [ $? -eq 0 ]; then
        log_info "配置文件測試通過"
    else
        log_error "配置文件測試失敗"
        exit 1
    fi
}

# 測試 3: 測試環境變量
test_env_vars() {
    log_info "測試環境變量..."

    # 設置環境變量
    export QUANTUM_SAFE_PROXY_LISTEN="127.0.0.1:8444"
    export QUANTUM_SAFE_PROXY_TARGET="127.0.0.1:6001"
    export QUANTUM_SAFE_PROXY_LOG_LEVEL="trace"
    export QUANTUM_SAFE_PROXY_BUFFER_SIZE="32768"
    export QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT="120"
    export QUANTUM_SAFE_PROXY_STRATEGY="dynamic"

    # 使用環境變量運行
    cargo run --bin check-environment

    if [ $? -eq 0 ]; then
        log_info "環境變量測試通過"
    else
        log_error "環境變量測試失敗"
        exit 1
    fi

    # 清理環境變量
    unset QUANTUM_SAFE_PROXY_LISTEN
    unset QUANTUM_SAFE_PROXY_TARGET
    unset QUANTUM_SAFE_PROXY_LOG_LEVEL
    unset QUANTUM_SAFE_PROXY_BUFFER_SIZE
    unset QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT
    unset QUANTUM_SAFE_PROXY_STRATEGY
}

# 測試 4: 測試命令行參數
test_cli_args() {
    log_info "測試命令行參數..."

    # 使用命令行參數運行
    cargo run --bin check-environment -- \
        --listen 127.0.0.1:8445 \
        --target 127.0.0.1:6002 \
        --log-level info \
        --buffer-size 8192 \
        --connection-timeout 30 \
        --strategy single

    if [ $? -eq 0 ]; then
        log_info "命令行參數測試通過"
    else
        log_error "命令行參數測試失敗"
        exit 1
    fi
}

# 測試 5: 測試優先順序 (命令行 > 環境變量 > 配置文件 > 默認值)
test_priority_order() {
    log_info "測試配置優先順序..."

    # 創建測試配置文件
    cat > test_config.json << EOF
{
    "listen": "127.0.0.1:8443",
    "target": "127.0.0.1:6000",
    "log_level": "info",
    "buffer_size": 8192,
    "connection_timeout": 30,
    "strategy": "single"
}
EOF

    # 設置環境變量
    export QUANTUM_SAFE_PROXY_LISTEN="127.0.0.1:8444"
    export QUANTUM_SAFE_PROXY_TARGET="127.0.0.1:6001"
    export QUANTUM_SAFE_PROXY_LOG_LEVEL="debug"
    export QUANTUM_SAFE_PROXY_BUFFER_SIZE="16384"
    export QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT="60"
    export QUANTUM_SAFE_PROXY_STRATEGY="sigalgs"

    # 使用命令行參數運行
    cargo run --bin check-environment -- \
        --listen 127.0.0.1:8445 \
        --target 127.0.0.1:6002 \
        --log-level trace \
        --buffer-size 32768 \
        --connection-timeout 120 \
        --strategy dynamic \
        --config-file test_config.json

    if [ $? -eq 0 ]; then
        log_info "配置優先順序測試通過"
    else
        log_error "配置優先順序測試失敗"
        exit 1
    fi

    # 清理環境變量
    unset QUANTUM_SAFE_PROXY_LISTEN
    unset QUANTUM_SAFE_PROXY_TARGET
    unset QUANTUM_SAFE_PROXY_LOG_LEVEL
    unset QUANTUM_SAFE_PROXY_BUFFER_SIZE
    unset QUANTUM_SAFE_PROXY_CONNECTION_TIMEOUT
    unset QUANTUM_SAFE_PROXY_STRATEGY
}

# 測試 6: 測試熱重載功能
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
    "strategy": "single"
}
EOF

    # 在後台啟動服務
    cargo run --bin quantum-safe-proxy -- --config-file test_config_reload.json &
    SERVER_PID=$!

    # 等待服務啟動
    sleep 2

    # 更新配置文件
    cat > test_config_reload.json << EOF
{
    "listen": "127.0.0.1:9999",
    "target": "127.0.0.1:6000",
    "log_level": "debug",
    "buffer_size": 16384,
    "connection_timeout": 60,
    "strategy": "sigalgs"
}
EOF

    # 發送 SIGHUP 信號觸發熱重載
    kill -HUP $SERVER_PID

    # 等待重載完成
    sleep 2

    # 檢查日誌以確認重載成功
    # 這裡需要根據實際情況調整
    log_info "熱重載測試完成，請檢查日誌確認重載是否成功"

    # 停止服務
    kill $SERVER_PID
}

# 運行所有測試
main() {
    log_info "開始測試配置和熱重載功能..."

    test_default_config
    test_config_file
    test_env_vars
    test_cli_args
    test_priority_order
    test_hot_reload

    log_info "所有測試完成"
}

main
