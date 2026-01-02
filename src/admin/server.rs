//! Admin HTTP Server Module
//!
//! This module sets up the HTTP server for the admin API using axum.

use std::net::SocketAddr;
use axum::{
    Router,
    routing::{get, post, patch},
    middleware,
};
use tower_http::trace::TraceLayer;

use crate::admin::auth::{AuthState, auth_middleware};
use crate::admin::handlers;
use crate::admin::error::AdminResult;
use crate::admin::types::ApiKey;

/// Admin server configuration
#[derive(Debug, Clone)]
pub struct AdminServerConfig {
    /// Listen address for admin API
    pub listen_addr: SocketAddr,

    /// API keys for authentication
    pub api_keys: Vec<ApiKey>,

    /// Audit log file path
    pub audit_log_path: String,
}

impl Default for AdminServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8443".parse().unwrap(),
            api_keys: Vec::new(),
            audit_log_path: "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string(),
        }
    }
}

/// Start the admin HTTP server
pub async fn start_admin_server(config: AdminServerConfig) -> AdminResult<()> {
    // Create authentication state
    let auth_state = AuthState::new(config.api_keys);

    // Build application router
    let app = build_router(auth_state);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    log::info!("Admin API server listening on {}", config.listen_addr);

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::admin::error::AdminError::Internal(e.to_string()))?;

    Ok(())
}

/// Build the application router with all routes
fn build_router(auth_state: AuthState) -> Router {
    // Create protected API router (requires authentication)
    let api_router = Router::new()
        // Configuration endpoints
        .route("/config", get(handlers::get_config))
        .route("/config", patch(handlers::patch_config))
        .route("/config/rollback", post(handlers::rollback_config))
        .route("/config/export", post(handlers::export_config))
        .route("/config/import", post(handlers::import_config))

        // Status endpoint
        .route("/status", get(handlers::get_status))

        // Service control endpoints
        .route("/restart", post(handlers::restart_service))

        // Audit endpoints
        .route("/audit", get(handlers::get_audit_log))
        .route("/audit/:id", get(handlers::get_audit_entry))
        .route("/audit/export", post(handlers::export_audit_log))

        // Add authentication middleware to all API routes
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ))
        .with_state(auth_state.clone());

    // Combine public and protected routes
    Router::new()
        // Public routes (no authentication required)
        .route("/health", get(handlers::health_check))
        .route("/", get(handlers::serve_ui))

        // Protected API routes
        .nest("/api", api_router)

        // Add tracing to all routes
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AdminServerConfig::default();
        assert_eq!(config.listen_addr.port(), 8443);
        assert!(config.api_keys.is_empty());
    }
}
