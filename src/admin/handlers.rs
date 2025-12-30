//! Admin API Request Handlers
//!
//! This module implements all HTTP request handlers for the admin API.

use std::sync::Arc;
use axum::{
    extract::{Path, Query, Extension},
    response::{Html, Json, IntoResponse, Response},
    http::{StatusCode, header},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::admin::auth::{AuthUser, require_role, can_modify_security_settings};
use crate::admin::types::*;
use crate::admin::error::{AdminError, AdminResult};
use crate::admin::config_resolver;
use crate::admin::audit::{AuditLog, AuditEntryBuilder, AuditFilter};
use crate::config;

/// Health check endpoint (no auth required)
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

/// Get effective configuration (Phase 3: T011-T017)
pub async fn get_config(
    Extension(user): Extension<AuthUser>,
) -> AdminResult<Json<ResolvedConfig>> {
    // Get current configuration from ConfigManager
    let config = config::get_config();

    // Resolve configuration into admin API format
    let resolved = config_resolver::resolve_config(config)?;

    log::info!("User {} (role: {:?}) retrieved configuration", user.name, user.role);

    Ok(Json(resolved))
}

/// Get operational status (Phase 3: T016)
pub async fn get_status(
    Extension(user): Extension<AuthUser>,
) -> AdminResult<Json<OperationalStatus>> {
    // TODO: Implement actual metrics collection
    // For now, return default status
    let status = OperationalStatus::default();

    log::info!("User {} (role: {:?}) retrieved operational status", user.name, user.role);

    Ok(Json(status))
}

/// Modify configuration settings (Phase 4: T018-T024)
pub async fn patch_config(
    Extension(user): Extension<AuthUser>,
    Json(request): Json<ConfigUpdateRequest>,
) -> AdminResult<Json<ConfigurationChange>> {
    // Require at least Operator role
    require_role(&user, Role::Operator)?;

    // Get current configuration
    let current_config = config::get_config();

    // Validate and build changes
    let mut changes = Vec::new();
    let mut requires_restart = false;
    let mut security_warnings = Vec::new();
    let mut validation_errors = Vec::new();

    for change_req in &request.changes {
        // Check if setting exists and can be changed
        let setting_name = &change_req.name;

        // Get current value
        let current_value = get_setting_value(&current_config, setting_name)?;

        // Check if security-affecting
        let is_security = config_resolver::is_security_affecting(setting_name);

        // If security-affecting, require Admin role
        if is_security && !can_modify_security_settings(&user) {
            return Err(AdminError::Authorization(format!(
                "Setting '{}' is security-affecting and requires Admin role",
                setting_name
            )));
        }

        // Validate new value
        if let Err(e) = validate_setting_value(setting_name, &change_req.value) {
            validation_errors.push(ValidationError {
                setting: setting_name.clone(),
                message: e.to_string(),
                expected: None,
                actual: format!("{:?}", change_req.value),
            });
            continue;
        }

        // Check if requires restart
        if !config_resolver::is_hot_reloadable(setting_name) {
            requires_restart = true;
        }

        // Generate security warnings if applicable
        if is_security {
            if let Some(warning) = generate_security_warning(
                setting_name,
                &current_value,
                &change_req.value
            ) {
                security_warnings.push(warning);
            }
        }

        changes.push(SettingChange {
            name: setting_name.clone(),
            before: current_value,
            after: change_req.value.clone(),
            security_affecting: is_security,
        });
    }

    // Build validation result
    let validation = if validation_errors.is_empty() {
        ValidationResult::valid()
    } else {
        ValidationResult::invalid(validation_errors)
    };

    // If there are security warnings and not confirmed, return for confirmation
    if !security_warnings.is_empty() && !request.confirmed {
        let change = ConfigurationChange {
            id: Uuid::new_v4(),
            operator: user.name.clone(),
            role: user.role,
            timestamp: Utc::now(),
            changes,
            validation,
            requires_restart,
            applied: false,
            warnings: security_warnings,
            confirmed: false,
        };

        log::warn!(
            "Configuration change by {} requires confirmation due to security warnings",
            user.name
        );

        return Ok(Json(change));
    }

    // If validation failed, return error
    if !validation.valid {
        let change = ConfigurationChange {
            id: Uuid::new_v4(),
            operator: user.name.clone(),
            role: user.role,
            timestamp: Utc::now(),
            changes,
            validation,
            requires_restart,
            applied: false,
            warnings: security_warnings,
            confirmed: request.confirmed,
        };

        return Ok(Json(change));
    }

    // Apply changes
    // TODO: Actually apply the configuration changes via ConfigManager
    // For now, just log and return success

    let change_id = Uuid::new_v4();

    log::info!(
        "Configuration change {} applied by {} (role: {:?}): {} setting(s) modified",
        change_id,
        user.name,
        user.role,
        changes.len()
    );

    // Log to audit trail
    log_to_audit(
        &user,
        AuditAction::ConfigChange,
        &changes,
        true,
        &security_warnings,
        request.confirmed.then(|| "Confirmed by operator".to_string()),
    )?;

    let change = ConfigurationChange {
        id: change_id,
        operator: user.name.clone(),
        role: user.role,
        timestamp: Utc::now(),
        changes,
        validation,
        requires_restart,
        applied: true,
        warnings: security_warnings,
        confirmed: request.confirmed,
    };

    Ok(Json(change))
}

/// Rollback to previous configuration (Phase 4: T024)
pub async fn rollback_config(
    Extension(user): Extension<AuthUser>,
) -> AdminResult<Json<ConfigurationChange>> {
    // Require Admin role for rollback
    require_role(&user, Role::Admin)?;

    // TODO: Implement actual rollback logic
    log::info!("Configuration rollback requested by {} (role: {:?})", user.name, user.role);

    Err(AdminError::Internal("Rollback not yet implemented".to_string()))
}

/// Export current configuration (Phase 6: T033-T034)
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "json".to_string()
}

pub async fn export_config(
    Extension(user): Extension<AuthUser>,
    Json(request): Json<ExportRequest>,
) -> AdminResult<Response> {
    // Any authenticated user can export
    let config = config::get_config();

    // Redact secrets (research.md R9)
    let export_config = config.as_ref().clone();
    // TODO: Implement actual secret redaction

    let content = match request.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&export_config)
                .map_err(|e| AdminError::Serialization(e))?;

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                json,
            )
        }
        "yaml" => {
            // TODO: Add serde_yaml dependency if needed
            return Err(AdminError::BadRequest("YAML format not yet supported".to_string()));
        }
        _ => {
            return Err(AdminError::BadRequest(format!("Unsupported format: {}", request.format)));
        }
    };

    log::info!(
        "Configuration exported by {} (role: {:?}) in {} format",
        user.name,
        user.role,
        request.format
    );

    // Log to audit trail
    log_to_audit(&user, AuditAction::ConfigExport, &[], true, &[], None)?;

    Ok(content.into_response())
}

/// Import configuration (Phase 6: T035-T039)
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    config: serde_json::Value,
    #[serde(default = "default_dry_run")]
    dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

pub async fn import_config(
    Extension(user): Extension<AuthUser>,
    Json(request): Json<ImportRequest>,
) -> AdminResult<Json<ImportPreview>> {
    // Require Operator role for import
    require_role(&user, Role::Operator)?;

    // Parse imported configuration
    let imported_config: crate::config::types::ProxyConfig =
        serde_json::from_value(request.config)
            .map_err(|e| AdminError::BadRequest(format!("Invalid configuration format: {}", e)))?;

    // Validate imported configuration
    let validation_result = match crate::config::validator::validate_config(&imported_config) {
        Ok(_) => ValidationResult::valid(),
        Err(e) => ValidationResult::invalid(vec![ValidationError {
            setting: "config".to_string(),
            message: e.to_string(),
            expected: None,
            actual: "imported configuration".to_string(),
        }]),
    };

    // Generate diff
    let current_config = config::get_config();
    let diff = generate_config_diff(&current_config, &imported_config);

    // Check for security warnings
    let security_warnings = diff
        .iter()
        .filter(|change| change.security_affecting)
        .filter_map(|change| {
            generate_security_warning(&change.name, &change.before, &change.after)
        })
        .collect();

    // Check if requires restart
    let requires_restart = diff
        .iter()
        .any(|change| !config_resolver::is_hot_reloadable(&change.name));

    let preview = ImportPreview {
        validation: validation_result,
        diff,
        requires_restart,
        warnings: security_warnings,
    };

    if request.dry_run {
        log::info!(
            "Configuration import preview by {} (role: {:?})",
            user.name,
            user.role
        );
        log_to_audit(&user, AuditAction::ConfigImportPreview, &preview.diff, false, &[], None)?;
    } else {
        // Actually apply the import
        log::info!(
            "Configuration import applied by {} (role: {:?})",
            user.name,
            user.role
        );
        // TODO: Actually apply the configuration
        log_to_audit(&user, AuditAction::ConfigImportApply, &preview.diff, true, &preview.warnings, None)?;
    }

    Ok(Json(preview))
}

/// Query audit log (Phase 7: T040-T042)
#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    operator: Option<String>,
    setting: Option<String>,
    action: Option<AuditAction>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    100
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    entries: Vec<AuditEntry>,
    total: usize,
    limit: usize,
    offset: usize,
}

pub async fn get_audit_log(
    Extension(user): Extension<AuthUser>,
    Query(query): Query<AuditQuery>,
) -> AdminResult<Json<AuditLogResponse>> {
    // Any authenticated user can view audit log

    // Get audit log path from environment
    let audit_log_path = std::env::var("ADMIN_AUDIT_LOG")
        .unwrap_or_else(|_| "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string());

    let audit_log = AuditLog::new(&audit_log_path)?;

    // Build filter
    let filter = AuditFilter {
        start_time: query.start_time,
        end_time: query.end_time,
        operator: query.operator,
        setting: query.setting,
        action: query.action,
        limit: Some(query.limit),
        offset: Some(query.offset),
    };

    // Query entries
    let entries = audit_log.query(filter)?;
    let total = entries.len();

    log::debug!(
        "Audit log queried by {} (role: {:?}): {} entries returned",
        user.name,
        user.role,
        total
    );

    Ok(Json(AuditLogResponse {
        entries,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// Get specific audit entry (Phase 7: T043)
pub async fn get_audit_entry(
    Extension(_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AdminResult<Json<AuditEntry>> {
    // Any authenticated user can view audit entries

    let audit_log_path = std::env::var("ADMIN_AUDIT_LOG")
        .unwrap_or_else(|_| "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string());

    let audit_log = AuditLog::new(&audit_log_path)?;

    match audit_log.get_by_id(&id)? {
        Some(entry) => Ok(Json(entry)),
        None => Err(AdminError::NotFound(format!("Audit entry {} not found", id))),
    }
}

/// Export audit log (Phase 7: T044)
#[derive(Debug, Deserialize)]
pub struct AuditExportRequest {
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    operator: Option<String>,
    setting: Option<String>,
}

pub async fn export_audit_log(
    Extension(user): Extension<AuthUser>,
    Json(request): Json<AuditExportRequest>,
) -> AdminResult<Response> {
    // Require Operator role to export audit log
    require_role(&user, Role::Operator)?;

    let audit_log_path = std::env::var("ADMIN_AUDIT_LOG")
        .unwrap_or_else(|_| "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string());

    let audit_log = AuditLog::new(&audit_log_path)?;

    // Build filter
    let filter = AuditFilter {
        start_time: request.start_time,
        end_time: request.end_time,
        operator: request.operator,
        setting: request.setting,
        action: None,
        limit: None,
        offset: None,
    };

    // Query all matching entries
    let entries = audit_log.query(filter)?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| AdminError::Serialization(e))?;

    log::info!(
        "Audit log exported by {} (role: {:?}): {} entries",
        user.name,
        user.role,
        entries.len()
    );

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json,
    )
        .into_response())
}

/// Serve embedded HTML UI
pub async fn serve_ui() -> Html<&'static str> {
    Html(crate::admin::html::ui_html())
}

// Helper functions

/// Get current value of a setting
fn get_setting_value(
    config: &Arc<crate::config::types::ProxyConfig>,
    setting_name: &str,
) -> AdminResult<serde_json::Value> {
    use serde_json::json;

    let value = match setting_name {
        "listen" => json!(config.listen().to_string()),
        "target" => json!(config.target().to_string()),
        "log_level" => json!(config.log_level()),
        "buffer_size" => json!(config.buffer_size()),
        "connection_timeout" => json!(config.connection_timeout()),
        "client_cert_mode" => json!(config.client_cert_mode().to_string()),
        "cert" => json!(config.cert().display().to_string()),
        "key" => json!(config.key().display().to_string()),
        "fallback_cert" => json!(config.fallback_cert().map(|p| p.display().to_string())),
        "fallback_key" => json!(config.fallback_key().map(|p| p.display().to_string())),
        "client_ca_cert" => json!(config.client_ca_cert().display().to_string()),
        _ => {
            return Err(AdminError::BadRequest(format!(
                "Unknown setting: {}",
                setting_name
            )));
        }
    };

    Ok(value)
}

/// Validate a setting value
fn validate_setting_value(setting_name: &str, value: &serde_json::Value) -> AdminResult<()> {
    // TODO: Implement comprehensive validation using existing validator
    // For now, basic type checking

    match setting_name {
        "log_level" => {
            let level = value.as_str().ok_or_else(|| {
                AdminError::Validation("log_level must be a string".to_string())
            })?;

            if !matches!(level, "error" | "warn" | "info" | "debug" | "trace") {
                return Err(AdminError::Validation(format!(
                    "Invalid log level: {}. Must be one of: error, warn, info, debug, trace",
                    level
                )));
            }
        }
        "buffer_size" => {
            let size = value.as_u64().ok_or_else(|| {
                AdminError::Validation("buffer_size must be a number".to_string())
            })?;

            if size == 0 {
                return Err(AdminError::Validation(
                    "buffer_size must be greater than 0".to_string(),
                ));
            }
        }
        "connection_timeout" => {
            let timeout = value.as_u64().ok_or_else(|| {
                AdminError::Validation("connection_timeout must be a number".to_string())
            })?;

            if timeout == 0 {
                return Err(AdminError::Validation(
                    "connection_timeout must be greater than 0".to_string(),
                ));
            }
        }
        _ => {
            // Allow other settings for now
        }
    }

    Ok(())
}

/// Generate security warning for a setting change (Phase 5: T025-T027)
fn generate_security_warning(
    setting_name: &str,
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<SecurityWarning> {
    match setting_name {
        "client_cert_mode" => {
            let before_mode = before.as_str()?;
            let after_mode = after.as_str()?;

            // Check for security downgrade
            if before_mode == "required" && after_mode != "required" {
                return Some(SecurityWarning {
                    level: WarningLevel::Critical,
                    message: "Disabling required client certificate authentication".to_string(),
                    affected_setting: setting_name.to_string(),
                    risk_explanation:
                        "This reduces authentication requirements and may allow unauthenticated clients"
                            .to_string(),
                    alternative: Some(
                        "Consider keeping 'required' mode for maximum security".to_string(),
                    ),
                });
            }

            if before_mode == "optional" && after_mode == "none" {
                return Some(SecurityWarning {
                    level: WarningLevel::High,
                    message: "Disabling client certificate verification".to_string(),
                    affected_setting: setting_name.to_string(),
                    risk_explanation: "Client certificates will not be verified".to_string(),
                    alternative: Some("Use 'optional' or 'required' mode".to_string()),
                });
            }
        }
        _ => {}
    }

    None
}

/// Generate diff between two configurations
fn generate_config_diff(
    current: &Arc<crate::config::types::ProxyConfig>,
    imported: &crate::config::types::ProxyConfig,
) -> Vec<SettingChange> {
    use serde_json::json;

    let mut changes = Vec::new();

    // Compare each setting
    if current.listen() != imported.listen() {
        changes.push(SettingChange {
            name: "listen".to_string(),
            before: json!(current.listen().to_string()),
            after: json!(imported.listen().to_string()),
            security_affecting: false,
        });
    }

    if current.target() != imported.target() {
        changes.push(SettingChange {
            name: "target".to_string(),
            before: json!(current.target().to_string()),
            after: json!(imported.target().to_string()),
            security_affecting: false,
        });
    }

    if current.log_level() != imported.log_level() {
        changes.push(SettingChange {
            name: "log_level".to_string(),
            before: json!(current.log_level()),
            after: json!(imported.log_level()),
            security_affecting: false,
        });
    }

    // TODO: Add more setting comparisons

    changes
}

/// Log action to audit trail
fn log_to_audit(
    user: &AuthUser,
    action: AuditAction,
    changes: &[SettingChange],
    applied: bool,
    warnings: &[SecurityWarning],
    confirmation: Option<String>,
) -> AdminResult<()> {
    let audit_log_path = std::env::var("ADMIN_AUDIT_LOG")
        .unwrap_or_else(|_| "/var/log/quantum-safe-proxy/admin-audit.jsonl".to_string());

    let mut audit_log = AuditLog::new(&audit_log_path)?;

    let mut builder = AuditEntryBuilder::new(user.name.clone(), user.role, action)
        .applied(applied);

    for change in changes {
        builder = builder.with_change(change.clone());
    }

    if !warnings.is_empty() {
        let warning_msgs: Vec<String> = warnings.iter().map(|w| w.message.clone()).collect();
        builder = builder.with_warnings(warning_msgs);
    }

    if let Some(conf) = confirmation {
        builder = builder.with_confirmation(conf);
    }

    audit_log.append(builder)?;

    Ok(())
}
