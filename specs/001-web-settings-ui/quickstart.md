# Quickstart: Web-Based Settings Management UI

**Feature**: 001-web-settings-ui
**Date**: 2025-12-30
**For**: Developers implementing the admin API

## Overview

This guide provides a quickstart for implementing the web-based settings management UI. Follow these steps to add the admin API to the Quantum Safe Proxy.

## Prerequisites

- Rust 1.86.0 or later
- Existing Quantum Safe Proxy codebase
- Familiarity with tokio async runtime
- Basic understanding of axum HTTP framework

## Step 1: Add Dependencies

Edit `Cargo.toml` and add:

```toml
[dependencies]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
http = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
sha2 = "0.10"
```

Then run:
```bash
cargo check
```

## Step 2: Create Admin Module Structure

```bash
mkdir -p src/admin
touch src/admin/mod.rs
touch src/admin/types.rs
touch src/admin/server.rs
touch src/admin/handlers.rs
touch src/admin/auth.rs
touch src/admin/audit.rs
touch src/admin/error.rs
touch src/admin/html.rs
```

## Step 3: Define Core Types

In `src/admin/types.rs`, implement the data model from [data-model.md](./data-model.md):

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConfig {
    pub settings: Vec<ResolvedSetting>,
    pub status: OperationalStatus,
    pub resolved_at: DateTime<Utc>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSetting {
    pub name: String,
    pub value: serde_json::Value,
    pub source: ConfigSource,
    pub hot_reloadable: bool,
    pub category: SettingCategory,
    pub description: Option<String>,
    pub security_affecting: bool,
}

// ... (continue with other types from data-model.md)
```

## Step 4: Implement Authentication

In `src/admin/auth.rs`:

```rust
use axum::{
    extract::Request,
    http::{StatusCode, HeaderMap},
    middleware::Next,
    response::Response,
};
use crate::admin::types::Role;

pub async fn auth_middleware(
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Bearer token from Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // Validate API key and get role
    let role = validate_api_key(token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Insert role into request extensions for handlers
    req.extensions_mut().insert(role);

    Ok(next.run(req).await)
}

fn validate_api_key(key: &str) -> Option<Role> {
    // TODO: Load API keys from config
    // For now, hardcode example
    match key {
        "admin-key-example" => Some(Role::Admin),
        "operator-key-example" => Some(Role::Operator),
        "viewer-key-example" => Some(Role::Viewer),
        _ => None,
    }
}
```

## Step 5: Implement HTTP Server

In `src/admin/server.rs`:

```rust
use axum::{
    Router,
    routing::{get, post, patch},
    middleware,
};
use std::net::SocketAddr;
use std::sync::Arc;
use crate::config::ConfigManager;
use crate::admin::{handlers, auth};

pub async fn start_admin_server(
    listen_addr: SocketAddr,
    config_manager: Arc<ConfigManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        // Configuration endpoints
        .route("/api/config", get(handlers::get_config))
        .route("/api/config", patch(handlers::patch_config))
        .route("/api/config/rollback", post(handlers::rollback_config))
        .route("/api/config/export", post(handlers::export_config))
        .route("/api/config/import", post(handlers::import_config))

        // Status endpoint
        .route("/api/status", get(handlers::get_status))

        // Audit endpoints
        .route("/api/audit", get(handlers::get_audit_log))
        .route("/api/audit/:id", get(handlers::get_audit_entry))
        .route("/api/audit/export", post(handlers::export_audit_log))

        // UI endpoint
        .route("/", get(handlers::serve_ui))

        // Authentication middleware
        .layer(middleware::from_fn(auth::auth_middleware))

        // Shared state
        .with_state(config_manager);

    // Start server
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

## Step 6: Implement Handlers (Example)

In `src/admin/handlers.rs`:

```rust
use axum::{
    extract::{State, Path, Query},
    Json,
    response::Html,
    http::StatusCode,
};
use std::sync::Arc;
use crate::config::ConfigManager;
use crate::admin::types::*;

pub async fn get_config(
    State(manager): State<Arc<ConfigManager>>,
) -> Result<Json<ResolvedConfig>, StatusCode> {
    // Derive ResolvedConfig from ConfigManager
    let config = manager.get_config().await;
    let resolved = ResolvedConfig::from(&config);

    Ok(Json(resolved))
}

pub async fn patch_config(
    State(manager): State<Arc<ConfigManager>>,
    Json(update): Json<ConfigUpdateRequest>,
) -> Result<Json<ConfigurationChange>, StatusCode> {
    // TODO: Implement validation and update logic
    todo!("Implement config update")
}

pub async fn get_status(
    State(manager): State<Arc<ConfigManager>>,
) -> Result<Json<OperationalStatus>, StatusCode> {
    // TODO: Collect runtime statistics
    todo!("Implement status collection")
}

pub async fn serve_ui() -> Html<&'static str> {
    Html(crate::admin::html::ui_html())
}

// ... (implement other handlers)
```

## Step 7: Integrate with main.rs

In `src/main.rs`:

```rust
mod admin;  // Add admin module

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... existing initialization ...

    // Start admin API server in background
    let admin_addr = "127.0.0.1:8443".parse()?;
    let config_manager_clone = Arc::clone(&config_manager);

    tokio::spawn(async move {
        if let Err(e) = admin::server::start_admin_server(admin_addr, config_manager_clone).await {
            eprintln!("Admin server error: {}", e);
        }
    });

    // ... continue with proxy server ...
}
```

## Step 8: Create Embedded HTML UI

Create `web/admin-ui.html` (single file):

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Quantum Safe Proxy - Admin</title>
    <style>
        body {
            font-family: system-ui, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .setting {
            padding: 10px;
            border-bottom: 1px solid #eee;
        }
        .setting:last-child {
            border-bottom: none;
        }
        .warning {
            background: #fff3cd;
            border: 1px solid #ffc107;
            padding: 10px;
            margin: 10px 0;
            border-radius: 4px;
        }
        .critical {
            background: #f8d7da;
            border-color: #dc3545;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Quantum Safe Proxy Admin</h1>
        <div id="config"></div>
        <div id="status"></div>
    </div>

    <script>
        const API_BASE = '/api';
        let apiKey = prompt('Enter API key:');

        async function fetchConfig() {
            const response = await fetch(`${API_BASE}/config`, {
                headers: {
                    'Authorization': `Bearer ${apiKey}`
                }
            });
            const data = await response.json();
            displayConfig(data);
        }

        function displayConfig(config) {
            const container = document.getElementById('config');
            container.innerHTML = '<h2>Configuration</h2>';

            config.settings.forEach(setting => {
                const div = document.createElement('div');
                div.className = 'setting';
                div.innerHTML = `
                    <strong>${setting.name}</strong>: ${JSON.stringify(setting.value)}
                    <br><small>Source: ${setting.source} | Hot-reload: ${setting.hot_reloadable}</small>
                `;
                container.appendChild(div);
            });
        }

        // Load on page load
        fetchConfig();
    </script>
</body>
</html>
```

Then in `src/admin/html.rs`:

```rust
pub fn ui_html() -> &'static str {
    include_str!("../../web/admin-ui.html")
}
```

## Step 9: Configure Admin API

Add to your config file (`config.toml`):

```toml
[admin]
enabled = true
listen = "127.0.0.1:8443"

[[admin.api_keys]]
key = "admin-key-example"
role = "Admin"
name = "admin-user"

[[admin.api_keys]]
key = "viewer-key-example"
role = "Viewer"
name = "readonly-user"
```

## Step 10: Test the Implementation

### Test GET /api/config

```bash
curl -H "Authorization: Bearer admin-key-example" http://127.0.0.1:8443/api/config
```

Expected response:
```json
{
  "settings": [
    {
      "name": "log_level",
      "value": "info",
      "source": "File",
      "hot_reloadable": true,
      "category": "Observability",
      "security_affecting": false
    }
  ],
  "status": { ... },
  "resolved_at": "2025-12-30T10:30:00Z",
  "version": 1
}
```

### Test PATCH /api/config

```bash
curl -X PATCH \
  -H "Authorization: Bearer admin-key-example" \
  -H "Content-Type: application/json" \
  -d '{"changes":[{"name":"log_level","value":"debug"}],"confirmed":false}' \
  http://127.0.0.1:8443/api/config
```

### Test UI

Open browser: `http://127.0.0.1:8443/`

## Common Pitfalls

### 1. Forgetting to Clone ConfigManager

```rust
// BAD: ConfigManager moved
let admin_task = admin::server::start_admin_server(addr, config_manager);

// GOOD: Clone Arc before spawning
let config_clone = Arc::clone(&config_manager);
let admin_task = admin::server::start_admin_server(addr, config_clone);
```

### 2. Not Implementing Hot-Reload Check

Always check `ResolvedSetting.hot_reloadable` before applying changes:

```rust
if setting.hot_reloadable {
    manager.update(new_config).await?;
} else {
    return Err("Restart required");
}
```

### 3. Missing Security Warning Confirmation

For security-affecting changes, require explicit confirmation:

```rust
if has_security_warnings && !request.confirmed {
    return Err("Confirmation required");
}
```

### 4. Forgetting Audit Logging

Always log changes to audit log:

```rust
let audit_entry = AuditEntry {
    operator: role.name(),
    action: AuditAction::ConfigChange,
    changes: changes.clone(),
    // ...
};
audit_log.append(audit_entry).await?;
```

## Next Steps

1. Implement remaining handlers (see [admin-api.yaml](./contracts/admin-api.yaml))
2. Add integration tests (see tasks.md Phase 8)
3. Implement audit log rotation (T045)
4. Add crypto mode classification (Constitution Principle IV)
5. Document trust boundaries (Constitution Principle II)

## References

- [Feature Specification](./spec.md)
- [Data Model](./data-model.md)
- [API Contract](./contracts/admin-api.yaml)
- [Research Decisions](./research.md)
- [Implementation Tasks](./tasks.md)

## Support

For questions or issues during implementation:
- Review constitution principles in `.specify/memory/constitution.md`
- Check existing ConfigManager implementation in `src/config/manager.rs`
- Refer to OpenAPI spec for exact request/response formats
