# Admin API 配置持久化測試指南

## 測試目的
驗證通過 Admin API 修改的配置在容器重啟後是否能夠保持。

## 前置條件
1. 容器使用 docker-compose.test.yml 配置啟動
2. Admin API 在 https://localhost:9443 運行
3. 準備好 API Key: `admin:V1IC+TSEgQlVAjGU7OGmbyOw132BK5RzSx1R6L+lj4s=`

## 測試步驟

### 方法 1: 使用 Web UI（推薦）

#### 步驟 1: 訪問 Web UI
1. 打開瀏覽器
2. 訪問: https://localhost:9443/
3. 接受自簽證書警告（如果有）
4. 輸入 API Key: `admin:V1IC+TSEgQlVAjGU7OGmbyOw132BK5RzSx1R6L+lj4s=`
5. 點擊 "Connect"

#### 步驟 2: 記錄初始配置
1. 在 Configuration 區域查看當前配置
2. 記錄 `log_level` 當前值（應該是 `debug`）
3. 記錄 `buffer_size` 當前值（應該是 `8192`）

#### 步驟 3: 修改配置
1. 在 Update Configuration 表單中輸入：
   ```
   Setting Name: log_level
   New Value: info
   ```
2. 點擊 "Update Setting"
3. 確認修改成功（應該顯示確認訊息）

#### 步驟 4: 驗證配置已更新
1. 刷新 Configuration 區域
2. 確認 `log_level` 已變更為 `info`
3. 檢查 config.json 文件內容：
   ```bash
   docker exec quantum-safe-proxy-quantum-safe-proxy-1 cat /app/config.json
   ```
4. 確認 JSON 文件中 log_level 已更新為 "info"

#### 步驟 5: 重啟容器
1. 點擊 "Restart Service" 按鈕
2. 等待約 5 秒容器重啟
3. 重新連接 Web UI

#### 步驟 6: 驗證配置保持
1. 查看 Configuration 區域
2. **驗證點**: `log_level` 應該仍然是 `info`（不是 `debug`）
3. 檢查容器日誌：
   ```bash
   docker logs quantum-safe-proxy-quantum-safe-proxy-1 2>&1 | grep "Log level"
   ```
4. **驗證點**: 日誌應顯示 "Log level: info"

### 方法 2: 使用命令行工具

如果 Web UI 無法訪問，可以使用以下命令行方法：

#### 步驟 1: 查看當前配置
```bash
cat config.json | jq '.log_level, .buffer_size'
```

#### 步驟 2: 手動修改 config.json
```bash
# 備份原始文件
cp config.json config.json.backup

# 修改 log_level 為 info
jq '.log_level = "info"' config.json > config.json.tmp
mv config.json.tmp config.json
```

#### 步驟 3: 重啟容器
```bash
docker compose -f docker-compose.test.yml restart quantum-safe-proxy
```

#### 步驟 4: 驗證配置保持
```bash
# 等待容器啟動
sleep 5

# 檢查日誌
docker logs quantum-safe-proxy-quantum-safe-proxy-1 2>&1 | grep -E "(Log level|log_level)"

# 檢查配置文件
docker exec quantum-safe-proxy-quantum-safe-proxy-1 cat /app/config.json | jq '.log_level'
```

## 預期結果

### ✅ 測試通過條件
1. 修改配置後，config.json 文件內容即時更新
2. 重啟容器後，配置保持修改後的值
3. 容器日誌顯示新的配置值生效
4. 多次重啟後配置依然保持

### ❌ 測試失敗情況
1. 重啟後配置恢復為初始值
2. config.json 文件未更新
3. 容器無法啟動
4. Admin API 修改報錯

## 測試案例清單

### 案例 1: 修改 log_level
- 初始值: `debug`
- 修改為: `info`
- 預期: 重啟後保持 `info`

### 案例 2: 修改 buffer_size
- 初始值: `8192`
- 修改為: `16384`
- 預期: 重啟後保持 `16384`

### 案例 3: 修改 connection_timeout
- 初始值: `30`
- 修改為: `60`
- 預期: 重啟後保持 `60`

## 故障排除

### 問題 1: Web UI 無法訪問
**解決方案**:
```bash
# 檢查容器狀態
docker ps | grep quantum-safe-proxy

# 檢查日誌
docker logs quantum-safe-proxy-quantum-safe-proxy-1

# 檢查端口綁定
docker port quantum-safe-proxy-quantum-safe-proxy-1
```

### 問題 2: config.json 未更新
**可能原因**:
1. 文件掛載為唯讀（檢查 docker-compose.yml 中是否為 `:rw`）
2. 權限問題（檢查主機文件權限）

**解決方案**:
```bash
# 檢查掛載配置
docker inspect quantum-safe-proxy-quantum-safe-proxy-1 | grep -A 5 Mounts

# 檢查文件權限
ls -la config.json
```

### 問題 3: 重啟後配置丟失
**可能原因**:
1. docker-compose.yml 中的命令行參數覆蓋了配置文件
2. 配置載入順序問題

**診斷方法**:
```bash
# 檢查容器啟動命令
docker inspect quantum-safe-proxy-quantum-safe-proxy-1 | jq '.[0].Args'

# 查看配置載入日誌
docker logs quantum-safe-proxy-quantum-safe-proxy-1 2>&1 | grep -E "(Configuration|config.json)"
```

## 測試記錄模板

```
測試日期: ___________
測試人員: ___________

測試案例 1: 修改 log_level
- 初始值: [ ]
- 修改為: [ ]
- 重啟前驗證: [ ] 通過 / [ ] 失敗
- 重啟後驗證: [ ] 通過 / [ ] 失敗
- 備註: ___________

測試案例 2: 修改 buffer_size
- 初始值: [ ]
- 修改為: [ ]
- 重啟前驗證: [ ] 通過 / [ ] 失敗
- 重啟後驗證: [ ] 通過 / [ ] 失敗
- 備註: ___________

整體測試結果: [ ] 通過 / [ ] 失敗
```

## 自動化測試腳本

如需自動化測試，可以使用以下腳本：

```bash
#!/bin/bash
# test-config-persistence.sh

set -e

CONTAINER="quantum-safe-proxy-quantum-safe-proxy-1"
CONFIG_FILE="config.json"

echo "=== 測試 Admin API 配置持久化 ==="

# 1. 記錄初始值
echo "步驟 1: 記錄初始 log_level..."
INITIAL_LOG_LEVEL=$(cat $CONFIG_FILE | jq -r '.log_level')
echo "初始 log_level: $INITIAL_LOG_LEVEL"

# 2. 修改配置
echo "步驟 2: 修改 log_level 為 info..."
jq '.log_level = "info"' $CONFIG_FILE > $CONFIG_FILE.tmp
mv $CONFIG_FILE.tmp $CONFIG_FILE

# 3. 重啟容器
echo "步驟 3: 重啟容器..."
docker compose -f docker-compose.test.yml restart $CONTAINER
sleep 5

# 4. 驗證配置
echo "步驟 4: 驗證配置是否保持..."
CURRENT_LOG_LEVEL=$(docker exec $CONTAINER cat /app/config.json | jq -r '.log_level')
echo "當前 log_level: $CURRENT_LOG_LEVEL"

# 5. 檢查結果
if [ "$CURRENT_LOG_LEVEL" == "info" ]; then
    echo "✅ 測試通過: 配置在重啟後保持"
    exit 0
else
    echo "❌ 測試失敗: 配置在重啟後丟失"
    exit 1
fi
```

使用方法：
```bash
chmod +x test-config-persistence.sh
./test-config-persistence.sh
```
