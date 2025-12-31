# Quantum Safe Proxy 網頁管理介面使用指南

**版本**: 1.0
**日期**: 2025-12-30
**適用於**: Quantum Safe Proxy v0.1.0+

---

## 📋 目錄

1. [快速開始](#快速開始)
2. [啟用 Admin API](#啟用-admin-api)
3. [訪問網頁介面](#訪問網頁介面)
4. [功能說明](#功能說明)
5. [角色權限](#角色權限)
6. [常見問題](#常見問題)
7. [安全建議](#安全建議)

---

## 🚀 快速開始

### 步驟 1: 生成安全的 API Key

使用 OpenSSL 生成隨機密鑰：

```bash
# 生成管理員密鑰
openssl rand -base64 32

# 輸出範例: xK9mP2nQ4vR8wT5yU7aB3cD6eF9gH1jK2lM4nP5qR8sT9uV0wX3yZ6
```

建議為不同角色生成多個密鑰。

### 步驟 2: 配置環境變數

```bash
# 1. 啟用 Admin API
export ADMIN_API_ENABLED=1

# 2. 設定監聽地址（預設: 127.0.0.1:9443）
export ADMIN_API_ADDR="127.0.0.1:9443"

# 3. 設定審計日誌位置
export ADMIN_AUDIT_LOG="/var/log/quantum-safe-proxy/admin-audit.jsonl"

# 4. 配置 API Keys（格式: name:key:role）
export ADMIN_API_KEYS="admin:xK9mP2nQ4vR8wT5yU7aB3cD6eF9gH1jK2lM4nP5qR8sT9uV0wX3yZ6:admin,viewer:另一個密鑰:viewer"
```

**API Keys 格式說明**:
- `name`: 用戶名稱（用於審計日誌）
- `key`: 密鑰（建議 32 字元以上）
- `role`: 角色（`viewer`、`operator`、`admin`）
- 多個密鑰用逗號分隔

### 步驟 3: 啟動 Proxy

```bash
# 啟動 Quantum Safe Proxy
cargo run --release -- --config config.json

# 或使用編譯好的二進制檔案
./target/release/quantum-safe-proxy --config config.json
```

啟動後會看到類似輸出：
```
INFO quantum_safe_proxy: Admin API enabled on http://127.0.0.1:9443
INFO quantum_safe_proxy: Admin UI available at http://127.0.0.1:9443/
```

---

## 🔧 啟用 Admin API

### 方法 1: 環境變數（推薦）

```bash
export ADMIN_API_ENABLED=1
export ADMIN_API_ADDR="127.0.0.1:9443"
export ADMIN_API_KEYS="admin:your-secret-key:admin"
export ADMIN_AUDIT_LOG="/var/log/quantum-safe-proxy/admin-audit.jsonl"
```

### 方法 2: 配置文件（較不安全，密鑰會存在檔案中）

在 `config.json` 中添加：

```json
{
  "admin_api": {
    "enabled": true,
    "addr": "127.0.0.1:9443",
    "audit_log": "/var/log/quantum-safe-proxy/admin-audit.jsonl",
    "api_keys": {
      "admin": {
        "key": "your-secret-key",
        "role": "admin"
      }
    }
  }
}
```

⚠️ **安全警告**: 方法 2 會將密鑰存放在配置文件中，建議僅在開發環境使用。

---

## 🌐 訪問網頁介面

### 開啟瀏覽器

訪問 Admin API 地址（預設為 `http://127.0.0.1:9443/`）：

```
http://127.0.0.1:9443/
```

### 驗證身份

在提示時輸入你的 API Key：

```
API Key: xK9mP2nQ4vR8wT5yU7aB3cD6eF9gH1jK2lM4nP5qR8sT9uV0wX3yZ6
```

驗證成功後，你將看到管理介面的三個主要頁籤。

---

## 🎯 功能說明

### 1. Status（狀態）頁籤

顯示 Proxy 的即時運行狀態：

**運行狀態**:
- ⏱️ **執行時間**: Proxy 已運行時間
- 🔗 **總連線數**: 自啟動以來的總連線數
- 📊 **活動連線**: 目前正在處理的連線數

**TLS 模式統計** (憲法原則 IV):
- 🔒 **Classical TLS**: 傳統 TLS 連線數（ECDHE、RSA）
- 🔐 **Hybrid TLS**: 混合 PQC 連線數（X25519MLKEM768）
- 🛡️ **PQC-Only TLS**: 純量子安全連線數（未來支援）

**握手統計**:
- ✅ **成功率**: 近 5 分鐘的握手成功率
- ⏱️ **平均時長**: 握手平均耗時（毫秒）
- 📈 **成功/失敗**: 成功和失敗的握手次數

**用途**:
- 監控系統健康狀況
- 追蹤 PQC 採用率
- 檢測異常連線模式

---

### 2. Configuration（配置）頁籤

查看和修改 Proxy 配置：

#### 查看配置

每個設定顯示：
- **名稱**: 設定項目名稱
- **當前值**: 目前生效的值
- **來源**: 值的來源（CLI、環境變數、配置文件、預設）
- **類別**: 設定類別（網路、TLS、日誌等）
- **熱重載**: 是否支援無需重啟即可生效
- **安全影響**: 是否為安全相關設定

#### 修改設定

1. **選擇設定項目**: 點擊要修改的設定
2. **輸入新值**: 在輸入框中輸入新值
3. **驗證**: 系統自動進行類型和範圍驗證
4. **安全警告**: 如果是安全相關設定，會顯示風險警告
   - ⚠️ 啟用 Classical TLS fallback（允許降級）
   - ⚠️ 停用加密模式分類（移除可見性）
   - ⚠️ 弱化憑證驗證
   - ⚠️ 停用客戶端認證
   - ⚠️ 啟用 passthrough 模式（繞過檢查）
5. **確認**: 閱讀警告後明確確認
6. **應用**:
   - **熱重載設定**: 立即生效，無需重啟
   - **需要重啟**: 顯示提示，手動重啟後生效

#### 設定範例

**修改日誌等級**（熱重載）:
```
log_level: info → debug
✅ 立即生效，無需重啟
```

**修改監聽地址**（需要重啟）:
```
listen: 0.0.0.0:8443 → 0.0.0.0:9443
⚠️ 需要重啟才能生效
```

**啟用 Classical TLS fallback**（安全警告）:
```
allow_classical_fallback: false → true
⚠️ 安全降級警告！
這將允許連線降級到傳統 TLS，降低量子安全保護。
是否確認？ [取消] [確認]
```

#### 回滾配置

如果配置出錯，可以快速回滾：

1. 點擊 **Rollback** 按鈕
2. 系統恢復到上一個有效配置
3. 審計日誌記錄回滾操作

---

### 3. Audit Log（審計日誌）頁籤

查看所有配置變更的完整記錄：

#### 日誌內容

每條審計記錄包含：
- **時間戳**: 變更發生時間（ISO 8601 格式）
- **操作員**: 執行變更的用戶名稱
- **角色**: 操作員的角色（viewer/operator/admin）
- **動作**: 執行的操作類型
  - `config_change` - 配置變更
  - `config_rollback` - 配置回滾
  - `config_export` - 配置匯出
  - `config_import` - 配置匯入
- **設定名稱**: 修改的設定項目
- **變更前**: 原始值
- **變更後**: 新值
- **是否應用**: 變更是否成功應用
- **安全警告**: 如有安全影響，記錄顯示的警告
- **雜湊值**: SHA256 雜湊鏈（防竄改）

#### 過濾和搜尋

使用過濾器快速找到特定記錄：

```
按設定名稱過濾: log_level
按操作員過濾: admin
按日期範圍: 2025-12-01 到 2025-12-31
```

#### 匯出審計日誌

點擊 **Export** 按鈕可匯出審計日誌用於合規報告：

- **格式**: JSON Lines (.jsonl)
- **完整性**: 包含雜湊鏈驗證
- **用途**: 合規審計、安全分析、事件調查

---

### 4. Import/Export（匯入/匯出）功能

#### 匯出配置

**步驟**:
1. 點擊 **Export Configuration** 按鈕
2. 選擇格式：JSON 或 YAML
3. 下載配置文件

**匯出範例** (JSON):
```json
{
  "listen": "0.0.0.0:8443",
  "target": "127.0.0.1:6000",
  "log_level": "info",
  "client_cert_mode": "optional",
  "admin": {
    "api_keys": {
      "admin": "<REDACTED_API_KEY>"
    }
  }
}
```

**安全清理** (Phase 10B - T064 待實作):
- ✅ API 密鑰已清理為 `<REDACTED_API_KEY>`
- ✅ 私鑰路徑保留，內容未匯出
- ✅ 敏感憑證已移除

#### 匯入配置

**步驟**:
1. 點擊 **Import Configuration** 按鈕
2. 選擇配置文件（JSON 或 YAML）
3. 上傳檔案
4. **預覽**: 查看差異比對（當前 vs. 匯入）
5. **驗證**: 系統檢查相容性和安全約束
6. **確認**: 明確確認後才應用
7. ⚠️ **不會自動應用**: 需要明確確認

**安全檢查**:
- 版本相容性檢查
- 安全約束驗證
- 憲法原則合規性檢查
- 差異比對顯示

---

## 👥 角色權限

### Viewer（檢視者）

**權限**:
- ✅ 查看運行狀態
- ✅ 查看配置（唯讀）
- ✅ 查看審計日誌
- ❌ 修改配置
- ❌ 匯入配置
- ❌ 回滾配置

**用途**: 監控人員、審計人員

---

### Operator（操作員）

**權限**:
- ✅ Viewer 的所有權限
- ✅ 修改非安全設定
  - 日誌等級
  - 緩衝區大小
  - 連線逾時
- ✅ 匯出配置
- ❌ 修改安全設定
- ❌ 匯入配置
- ❌ 回滾配置

**用途**: 日常運維人員

---

### Admin（管理員）

**權限**:
- ✅ Operator 的所有權限
- ✅ 修改安全設定
  - TLS 模式
  - 憑證配置
  - 客戶端認證模式
  - Passthrough 模式
- ✅ 匯入配置
- ✅ 回滾配置
- ✅ 管理 API Keys（未來功能）

**用途**: 安全工程師、系統管理員

---

## ❓ 常見問題

### Q1: 忘記 API Key 怎麼辦？

**A**: API Key 儲存在環境變數 `ADMIN_API_KEYS` 中：

```bash
# 查看當前設定
echo $ADMIN_API_KEYS

# 重新生成密鑰
openssl rand -base64 32

# 更新環境變數
export ADMIN_API_KEYS="admin:新密鑰:admin"

# 重啟 Proxy
```

---

### Q2: 無法訪問網頁介面

**檢查清單**:

1. **Admin API 已啟用？**
   ```bash
   echo $ADMIN_API_ENABLED  # 應該顯示 "1"
   ```

2. **地址配置正確？**
   ```bash
   echo $ADMIN_API_ADDR  # 檢查地址和端口
   ```

3. **Proxy 正在運行？**
   ```bash
   curl http://127.0.0.1:9443/health
   # 預期輸出: {"status":"ok","timestamp":"..."}
   ```

4. **防火牆設定？**
   - 確保端口未被防火牆阻擋
   - 如果從遠端訪問，需要調整 `ADMIN_API_ADDR` 為 `0.0.0.0:9443`

---

### Q3: 修改配置後沒有生效

**可能原因**:

1. **設定需要重啟**:
   - 檢查設定的 "熱重載" 標記
   - 如顯示 ❌，需要手動重啟 Proxy

2. **優先級被覆蓋**:
   - 配置優先級：CLI 參數 > 環境變數 > UI 變更 > 配置文件 > 預設值
   - 檢查是否有更高優先級的來源覆蓋了變更

3. **驗證失敗**:
   - 查看錯誤訊息
   - 檢查值的類型和範圍是否正確

---

### Q4: 如何查看 TLS 模式統計？

**步驟**:
1. 訪問 **Status** 頁籤
2. 查看 **TLS Mode Statistics** 區塊
3. 數據每 30 秒更新一次

**統計來源**:
- 從結構化日誌收集
- 即時分類每個 TLS 連線
- 基於密碼套件檢查（憲法原則 IV）

**範例**:
```
Classical TLS: 150 connections (60%)
Hybrid TLS: 100 connections (40%)
PQC-Only TLS: 0 connections (0%)
```

---

### Q5: 審計日誌如何防止竄改？

**技術**:
- SHA256 雜湊鏈
- 每個條目包含上一個條目的雜湊值
- 竄改任何條目會破壞整個鏈

**驗證** (Phase 10B - T065 待實作):
```bash
# 使用 API 驗證審計日誌
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://127.0.0.1:9443/api/audit/verify

# 預期輸出
{
  "valid": true,
  "total_entries": 1250,
  "verified_entries": 1250,
  "first_mismatch": null
}
```

---

### Q6: 可以遠端訪問 Admin UI 嗎？

**預設**: 僅允許本地訪問（127.0.0.1）

**啟用遠端訪問**:

```bash
# ⚠️ 僅在安全網路中使用
export ADMIN_API_ADDR="0.0.0.0:9443"

# 更安全：使用內部 IP
export ADMIN_API_ADDR="192.168.1.100:9443"

# 最佳實踐：配置 HTTPS + 反向代理
# 使用 nginx/caddy 提供 TLS + 基本認證
```

⚠️ **安全警告**:
- 不建議直接暴露 Admin API 到公網
- 使用反向代理（nginx、caddy）提供 HTTPS
- 啟用 IP 白名單
- 考慮 VPN 或 SSH 隧道

---

### Q7: 如何備份配置？

**自動化備份腳本**:

```bash
#!/bin/bash
# backup-config.sh

API_KEY="your-admin-api-key"
BACKUP_DIR="/var/backups/quantum-safe-proxy"
DATE=$(date +%Y%m%d-%H%M%S)

# 建立備份目錄
mkdir -p "$BACKUP_DIR"

# 匯出配置
curl -H "Authorization: Bearer $API_KEY" \
     -X POST \
     -H "Content-Type: application/json" \
     -d '{"format": "json"}' \
     http://127.0.0.1:9443/api/config/export \
     > "$BACKUP_DIR/config-$DATE.json"

# 匯出審計日誌
curl -H "Authorization: Bearer $API_KEY" \
     -X POST \
     http://127.0.0.1:9443/api/audit/export \
     > "$BACKUP_DIR/audit-$DATE.jsonl"

echo "備份完成: $BACKUP_DIR"
```

**定期執行**:
```bash
# 添加到 crontab (每天凌晨 2 點)
crontab -e

# 添加以下行
0 2 * * * /path/to/backup-config.sh
```

---

### Q8: 多人同時編輯配置會怎樣？

**現狀** (Phase 10B - T063 待實作):
- ⚠️ 後一個保存的會覆蓋前一個
- 可能導致設定遺失

**計劃實作**:
- ✅ 樂觀鎖定（Optimistic Locking）
- ✅ ETag 版本控制
- ✅ 衝突檢測和提示
- ✅ 第二個編輯者會收到衝突警告

**目前建議**:
- 協調編輯時間
- 使用審計日誌追蹤變更
- 立即檢查變更結果

---

## 🔒 安全建議

### 1. API Key 管理

**生成強密鑰**:
```bash
# 至少 32 字元
openssl rand -base64 32
```

**定期輪換**:
```bash
# 每 90 天更換一次
# 1. 生成新密鑰
NEW_KEY=$(openssl rand -base64 32)

# 2. 更新環境變數
export ADMIN_API_KEYS="admin:$NEW_KEY:admin"

# 3. 重啟服務
systemctl restart quantum-safe-proxy

# 4. 通知相關人員
```

**分離權限**:
```bash
# 為不同角色使用不同密鑰
export ADMIN_API_KEYS="alice:key1:admin,bob:key2:operator,charlie:key3:viewer"
```

---

### 2. 網路安全

**僅本地訪問** (預設，最安全):
```bash
export ADMIN_API_ADDR="127.0.0.1:9443"
```

**使用 SSH 隧道** (遠端訪問):
```bash
# 在遠端機器上
ssh -L 9443:127.0.0.1:9443 user@proxy-server

# 在本地瀏覽器訪問
http://localhost:9443/
```

**反向代理 + HTTPS**:
```nginx
# nginx 配置範例
server {
    listen 443 ssl;
    server_name proxy-admin.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://127.0.0.1:9443;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;

        # IP 白名單
        allow 192.168.1.0/24;
        deny all;
    }
}
```

---

### 3. 審計日誌保護

**設定正確權限**:
```bash
# 建立日誌目錄
sudo mkdir -p /var/log/quantum-safe-proxy

# 設定擁有者和權限
sudo chown quantum-safe-proxy:quantum-safe-proxy /var/log/quantum-safe-proxy
sudo chmod 750 /var/log/quantum-safe-proxy

# 日誌檔案權限
sudo chmod 640 /var/log/quantum-safe-proxy/admin-audit.jsonl
```

**定期備份**:
```bash
# 每週備份到安全位置
tar -czf audit-backup-$(date +%Y%m%d).tar.gz \
    /var/log/quantum-safe-proxy/admin-audit.jsonl

# 傳輸到安全儲存
scp audit-backup-*.tar.gz backup-server:/secure/backups/
```

**監控異常**:
```bash
# 監控失敗的認證嘗試
grep "authentication failed" /var/log/quantum-safe-proxy/admin-audit.jsonl

# 監控安全設定變更
grep "security_affecting.*true" /var/log/quantum-safe-proxy/admin-audit.jsonl
```

---

### 4. 最佳實踐

✅ **啟用前**:
- 閱讀文件了解功能和限制
- 規劃 API key 管理策略
- 設定安全的網路存取控制

✅ **使用中**:
- 定期檢查審計日誌
- 驗證配置變更是否符合預期
- 監控異常活動（失敗的認證、大量變更）

✅ **維護**:
- 定期輪換 API keys
- 備份配置和審計日誌
- 保持 Proxy 版本更新

❌ **避免**:
- 在公網直接暴露 Admin API
- 在配置文件中儲存 API keys
- 共用 Admin 角色的 API key
- 忽略安全警告直接確認

---

## 📚 更多資源

### 文件

- **技術文件**: `docs/crypto-mode-classification.md` - 加密模式分類說明
- **實作筆記**: `docs/phase-10-implementation-notes.md` - 待實作功能
- **完整報告**: `docs/IMPLEMENTATION-COMPLETE.md` - 實作狀態
- **憲法**: `.specify/memory/constitution.md` - 安全原則

### API 文件

完整 REST API 文件位於：
- OpenAPI 規格: `specs/001-web-settings-ui/contracts/admin-api.yaml`
- 資料模型: `specs/001-web-settings-ui/data-model.md`

### 支援

遇到問題？
1. 查看日誌: `tail -f /var/log/quantum-safe-proxy/proxy.log`
2. 檢查審計日誌: 查看 Admin UI 的 Audit Log 頁籤
3. 提交 Issue: https://github.com/JerryR7/quantum-safe-proxy/issues

---

## ✨ 功能路線圖

### 已完成 ✅
- 配置查看和修改
- 熱重載支援
- 安全警告和確認
- 審計日誌（SHA256 雜湊鏈）
- 配置匯入/匯出
- 加密模式分類（憲法原則 IV）
- 角色權限控制（Viewer/Operator/Admin）

### 進行中 🚧 (Phase 10B)
- 並發編輯保護（樂觀鎖定）- T063
- 匯出清理（移除敏感憑證）- T064
- 審計日誌驗證端點 - T065
- 外部配置變更檢測 - T062

### 計劃中 📋
- 即時 TLS 模式統計聚合
- 配置版本控制（多版本回滾）
- 多租戶配置管理
- 高級配置範本

---

**最後更新**: 2025-12-30
**版本**: 1.0
**作者**: Quantum Safe Proxy Team
