#!/bin/bash

# 終止所有正在運行的 quantum-safe-proxy 進程
pkill -f quantum-safe-proxy

# 清除終端
clear

echo "===== 測試配置優先順序 ====="
echo ""

# 測試 1: 只使用配置文件
echo "測試 1: 只使用配置文件"
echo "預期結果: log_level=info, buffer_size=4096"
RUST_LOG=quantum_safe_proxy=debug cargo run --bin quantum-safe-proxy -- --config-file config.json &
PID=$!
sleep 2
echo "按 Enter 繼續..."
read
kill -9 $PID
pkill -f quantum-safe-proxy
echo ""

# 測試 2: 配置文件 + 環境變量
echo "測試 2: 配置文件 + 環境變量"
echo "預期結果: log_level=trace, buffer_size=4096 (環境變量覆蓋配置文件)"
export QUANTUM_SAFE_PROXY_LOG_LEVEL="trace"
RUST_LOG=quantum_safe_proxy=debug cargo run --bin quantum-safe-proxy -- --config-file config.json &
PID=$!
sleep 2
echo "按 Enter 繼續..."
read
kill -9 $PID
pkill -f quantum-safe-proxy
echo ""

# 測試 3: 配置文件 + 環境變量 + 命令行參數
echo "測試 3: 配置文件 + 環境變量 + 命令行參數"
echo "預期結果: log_level=warn, buffer_size=16384 (命令行參數覆蓋環境變量和配置文件)"
export QUANTUM_SAFE_PROXY_LOG_LEVEL="trace"
RUST_LOG=quantum_safe_proxy=debug cargo run --bin quantum-safe-proxy -- --config-file config.json --log-level warn --buffer-size 16384 &
PID=$!
sleep 2
echo "按 Enter 繼續..."
read
kill -9 $PID
pkill -f quantum-safe-proxy
echo ""

# 清除環境變量
unset QUANTUM_SAFE_PROXY_LOG_LEVEL

echo "測試完成!"
