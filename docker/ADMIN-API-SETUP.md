# Docker Admin API 設定指南

## 📋 更新內容

docker-compose.yml 已更新，新增以下功能：

1. ✅ Admin API 端口映射 (9443)
2. ✅ 審計日誌 volume 掛載
3. ✅ Admin API 環境變數配置
4. ✅ 三種角色的 API Key 範本

## 🔑 步驟 1: 生成安全的 API Keys

### 自動生成腳本

```bash
# 建立 logs 目錄
mkdir -p logs

# 生成三個安全密鑰
echo "正在生成安全的 API Keys..."
ADMIN_KEY=$(openssl rand -base64 32)
OPERATOR_KEY=$(openssl rand -base64 32)
VIEWER_KEY=$(openssl rand -base64 32)

echo ""
echo "==================== API Keys ===================="
echo "Admin Key:    $ADMIN_KEY"
echo "Operator Key: $OPERATOR_KEY"
echo "Viewer Key:   $VIEWER_KEY"
echo "=================================================="
echo ""
echo "請將以下內容更新到 docker-compose.yml 的 ADMIN_API_KEYS:"
echo ""
echo "admin:${ADMIN_KEY}:admin,operator:${OPERATOR_KEY}:operator,viewer:${VIEWER_KEY}:viewer"
echo ""

# 保存到文件（僅供參考，請妥善保管！）
cat > .api-keys.txt << EOF
# ⚠️ 重要：請妥善保管此文件，不要提交到 Git！
# 生成時間: $(date)

Admin Key:    $ADMIN_KEY
Operator Key: $OPERATOR_KEY
Viewer Key:   $VIEWER_KEY

Docker Compose 環境變數格式:
ADMIN_API_KEYS=admin:${ADMIN_KEY}:admin,operator:${OPERATOR_KEY}:operator,viewer:${VIEWER_KEY}:viewer
EOF

echo "✅ API Keys 已保存到 .api-keys.txt"
echo "⚠️  請確保 .api-keys.txt 已加入 .gitignore！"
```

### 手動生成

如果你想手動生成：

```bash
# 生成 Admin Key
openssl rand -base64 32

# 生成 Operator Key
openssl rand -base64 32

# 生成 Viewer Key
openssl rand -base64 32
```

## 🚀 步驟 2: 更新 docker-compose.yml

在 `docker-compose.yml` 中找到這一行：

```yaml
- ADMIN_API_KEYS=admin:CHANGE_THIS_TO_SECURE_KEY_GENERATED_BY_OPENSSL:admin,operator:ANOTHER_SECURE_KEY:operator,viewer:READONLY_KEY:viewer
```

替換為：

```yaml
- ADMIN_API_KEYS=admin:你的Admin密鑰:admin,operator:你的Operator密鑰:operator,viewer:你的Viewer密鑰:viewer
```

**範例** (使用上面生成的密鑰):
```yaml
- ADMIN_API_KEYS=admin:xK9mP2nQ4vR8wT5yU7aB3cD6eF9gH1jK:admin,operator:lM4nP5qR8sT9uV0wX3yZ6A1bC2dE3fG4:operator,viewer:hI5jK6lM7nO8pQ9rS0tU1vW2xY3zA4bC:viewer
```

## 🏃 步驟 3: 啟動服務

```bash
# 停止現有服務（如果在運行）
docker-compose down

# 重新構建映像（如果有代碼更新）
docker-compose build

# 啟動服務
docker-compose up -d

# 查看日誌
docker-compose logs -f quantum-safe-proxy
```

你應該看到類似的輸出：
```
INFO quantum_safe_proxy: Admin API enabled on http://0.0.0.0:9443
INFO quantum_safe_proxy: Admin UI available at http://0.0.0.0:9443/
```

## 🌐 步驟 4: 訪問 Admin UI

### 在 Docker Host 上訪問

打開瀏覽器訪問：
```
http://localhost:9443/
```

### 從其他電腦訪問

如果你想從網路中的其他電腦訪問：
```
http://你的Docker主機IP:9443/
```

**範例**:
```
http://192.168.1.100:9443/
```

### 輸入 API Key

當提示時，輸入你生成的 API Key：

- **Admin** 權限: 輸入 Admin Key
- **Operator** 權限: 輸入 Operator Key
- **Viewer** 權限: 輸入 Viewer Key

## 🔍 步驟 5: 驗證功能

### 1. 檢查健康狀態

```bash
curl http://localhost:9443/health
```

預期輸出：
```json
{"status":"ok","timestamp":"2025-12-30T12:00:00Z"}
```

### 2. 查看配置（使用 API Key）

```bash
# 替換 YOUR_API_KEY 為你的實際密鑰
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:9443/api/config
```

### 3. 查看運行狀態

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:9443/api/status
```

### 4. 查看審計日誌

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:9443/api/audit
```

## 📁 檔案結構

更新後的結構：

```
quantum-safe-proxy/
├── docker-compose.yml          # ✅ 已更新（包含 Admin API 配置）
├── config.json
├── certs/
│   └── ...
├── logs/                       # 🆕 新建目錄
│   └── admin-audit.jsonl      # 審計日誌（自動生成）
├── scripts/
│   └── ...
└── docker/
    ├── ADMIN-API-SETUP.md      # 🆕 本文件
    └── ...
```

## 🔒 安全建議

### 1. 保護 API Keys

```bash
# 將 API Keys 文件加入 .gitignore
echo ".api-keys.txt" >> .gitignore
echo "logs/" >> .gitignore
```

### 2. 僅在受信任網路中使用

Admin API 預設綁定 `0.0.0.0:9443`，這意味著可以從任何網路介面訪問。

**在生產環境中的建議**:

#### 選項 A: 僅本地訪問（最安全）

修改 docker-compose.yml：
```yaml
ports:
  - "127.0.0.1:9443:9443"  # 僅本地訪問
```

然後使用 SSH 隧道遠端訪問：
```bash
ssh -L 9443:localhost:9443 user@docker-host
```

#### 選項 B: 使用反向代理（推薦）

使用 nginx 或 Traefik 提供 HTTPS + 認證：

```yaml
# docker-compose.yml 添加 nginx
nginx:
  image: nginx:alpine
  ports:
    - "443:443"
  volumes:
    - ./nginx/nginx.conf:/etc/nginx/nginx.conf
    - ./nginx/ssl:/etc/nginx/ssl
  depends_on:
    - quantum-safe-proxy
```

nginx 配置範例：
```nginx
server {
    listen 443 ssl;
    server_name proxy-admin.example.com;

    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;

    location / {
        proxy_pass http://quantum-safe-proxy:9443;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;

        # IP 白名單
        allow 192.168.1.0/24;
        deny all;
    }
}
```

### 3. 定期輪換 API Keys

```bash
# 每 90 天更換一次 API Keys
# 1. 生成新密鑰（使用上面的腳本）
# 2. 更新 docker-compose.yml
# 3. 重啟服務
docker-compose restart quantum-safe-proxy
```

### 4. 監控審計日誌

```bash
# 查看最近的審計日誌
tail -f logs/admin-audit.jsonl

# 搜尋失敗的認證
grep "authentication failed" logs/admin-audit.jsonl

# 搜尋安全相關變更
grep "security_affecting" logs/admin-audit.jsonl
```

## 🔧 故障排除

### 問題 1: Admin API 無法訪問

**檢查服務狀態**:
```bash
docker-compose ps
docker-compose logs quantum-safe-proxy | grep -i admin
```

**確認環境變數**:
```bash
docker-compose exec quantum-safe-proxy env | grep ADMIN
```

### 問題 2: 認證失敗

**檢查 API Key 格式**:
- 確保沒有多餘的空格
- 確保使用正確的格式: `name:key:role`
- 確保密鑰已正確設定在環境變數中

**查看認證日誌**:
```bash
docker-compose logs quantum-safe-proxy | grep -i "auth"
```

### 問題 3: 審計日誌寫入失敗

**檢查目錄權限**:
```bash
# 在 host 上
ls -la logs/

# 如果需要，修正權限
chmod 755 logs/
```

**檢查容器內的路徑**:
```bash
docker-compose exec quantum-safe-proxy ls -la /var/log/quantum-safe-proxy/
```

### 問題 4: 端口衝突

如果端口 9443 已被佔用：

**修改端口映射**:
```yaml
ports:
  - "19443:9443"  # 使用不同的 host 端口
```

然後訪問：
```
http://localhost:19443/
```

## 📞 獲取幫助

- **文件**: `docs/USER-GUIDE-ZH-TW.md` - 完整使用指南
- **API 文件**: `specs/001-web-settings-ui/contracts/admin-api.yaml`
- **日誌**: `docker-compose logs quantum-safe-proxy`
- **GitHub Issues**: https://github.com/JerryR7/quantum-safe-proxy/issues

## ✅ 快速檢查清單

在啟動前確認：

- [ ] 已生成安全的 API Keys
- [ ] 已更新 docker-compose.yml 中的 ADMIN_API_KEYS
- [ ] 已建立 logs/ 目錄
- [ ] 已將 .api-keys.txt 加入 .gitignore
- [ ] 已審查網路安全設定
- [ ] 已閱讀安全建議

啟動後驗證：

- [ ] Admin API 健康檢查通過
- [ ] 可以使用 API Key 登入 UI
- [ ] 可以查看配置和狀態
- [ ] 審計日誌正常記錄

---

**最後更新**: 2025-12-30
**版本**: 1.0
