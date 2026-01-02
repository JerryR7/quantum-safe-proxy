//! Audit Log Module
//!
//! This module provides append-only JSONL audit logging with SHA256 hash chaining
//! for tamper evidence. All configuration changes and security events are logged.

use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufRead, Write};
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use uuid::Uuid;

use crate::admin::types::{AuditEntry, AuditAction, SettingChange, Role};
use crate::admin::error::{AdminError, AdminResult};

/// Audit log manager
#[derive(Debug)]
pub struct AuditLog {
    /// Path to audit log file
    file_path: PathBuf,

    /// Last entry hash (for chaining)
    last_hash: Option<String>,
}

impl AuditLog {
    /// Create a new audit log at the specified path
    pub fn new<P: AsRef<Path>>(file_path: P) -> AdminResult<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Load last hash if file exists
        let last_hash = if file_path.exists() {
            Self::read_last_hash(&file_path)?
        } else {
            None
        };

        Ok(Self {
            file_path,
            last_hash,
        })
    }

    /// Append an entry to the audit log
    pub fn append(&mut self, entry: AuditEntryBuilder) -> AdminResult<AuditEntry> {
        // Build entry with hash chaining
        let prev_hash = self.last_hash.as_deref().unwrap_or("");
        let mut audit_entry = entry.build(prev_hash);

        // Calculate hash for this entry
        let hash = self.calculate_hash(&audit_entry)?;
        audit_entry.hash = hash.clone();

        // Append to file
        self.write_entry(&audit_entry)?;

        // Update last hash
        self.last_hash = Some(hash);

        Ok(audit_entry)
    }

    /// Query audit log entries with filtering
    pub fn query(&self, filter: AuditFilter) -> AdminResult<Vec<AuditEntry>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEntry>(&line) {
                Ok(entry) => {
                    if filter.matches(&entry) {
                        entries.push(entry);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse audit entry: {}", e);
                    continue;
                }
            }
        }

        // Apply pagination
        let start = filter.offset.unwrap_or(0);
        let end = start + filter.limit.unwrap_or(entries.len());

        Ok(entries.into_iter()
            .skip(start)
            .take(end - start)
            .collect())
    }

    /// Get a specific audit entry by ID
    pub fn get_by_id(&self, id: &Uuid) -> AdminResult<Option<AuditEntry>> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEntry>(&line) {
                Ok(entry) if entry.id == *id => return Ok(Some(entry)),
                Ok(_) => continue,
                Err(e) => {
                    log::warn!("Failed to parse audit entry: {}", e);
                    continue;
                }
            }
        }

        Ok(None)
    }

    /// Verify audit log integrity by checking hash chain
    pub fn verify_integrity(&self) -> AdminResult<bool> {
        if !self.file_path.exists() {
            return Ok(true);
        }

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut prev_hash = String::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: AuditEntry = serde_json::from_str(&line)
                .map_err(|e| AdminError::AuditLog(format!("Invalid entry: {}", e)))?;

            // Verify previous hash matches
            if entry.prev_hash != prev_hash {
                return Ok(false);
            }

            // Recalculate hash and verify
            let calculated_hash = self.calculate_hash(&entry)?;
            if entry.hash != calculated_hash {
                return Ok(false);
            }

            prev_hash = entry.hash.clone();
        }

        Ok(true)
    }

    /// Calculate SHA256 hash for an audit entry
    fn calculate_hash(&self, entry: &AuditEntry) -> AdminResult<String> {
        // Serialize entry without hash field
        let mut entry_clone = entry.clone();
        entry_clone.hash = String::new();

        let json = serde_json::to_string(&entry_clone)?;

        // Calculate SHA256(prev_hash || json)
        let mut hasher = Sha256::new();
        hasher.update(entry.prev_hash.as_bytes());
        hasher.update(json.as_bytes());
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    /// Write an entry to the audit log file
    fn write_entry(&self, entry: &AuditEntry) -> AdminResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;

        let json = serde_json::to_string(entry)?;
        writeln!(file, "{}", json)?;
        file.sync_all()?;

        Ok(())
    }

    /// Read the last hash from the audit log
    fn read_last_hash(file_path: &Path) -> AdminResult<Option<String>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        let mut last_line = None;
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.trim().is_empty() {
                    last_line = Some(line);
                }
            }
        }

        if let Some(line) = last_line {
            let entry: AuditEntry = serde_json::from_str(&line)
                .map_err(|e| AdminError::AuditLog(format!("Invalid last entry: {}", e)))?;
            Ok(Some(entry.hash))
        } else {
            Ok(None)
        }
    }

    /// Rotate audit log by removing entries older than retention period
    ///
    /// This implements T045: 90-day retention policy
    /// Entries older than 90 days are moved to an archive file
    pub fn rotate(&mut self, retention_days: u32) -> AdminResult<usize> {
        if !self.file_path.exists() {
            return Ok(0);
        }

        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);

        let mut kept_entries = Vec::new();
        let mut archived_entries = Vec::new();

        // Read all entries and separate old from new
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEntry>(&line) {
                Ok(entry) => {
                    if entry.timestamp < cutoff {
                        archived_entries.push(entry);
                    } else {
                        kept_entries.push(entry);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse audit entry during rotation: {}", e);
                    continue;
                }
            }
        }

        let archived_count = archived_entries.len();

        // If there are entries to archive, move them to archive file
        if !archived_entries.is_empty() {
            let archive_path = self.file_path.with_extension(
                format!("jsonl.archive.{}", Utc::now().format("%Y%m%d_%H%M%S"))
            );

            let mut archive_file = File::create(&archive_path)?;
            for entry in &archived_entries {
                let json = serde_json::to_string(entry)?;
                writeln!(archive_file, "{}", json)?;
            }
            archive_file.sync_all()?;

            log::info!(
                "Archived {} audit entries older than {} days to {:?}",
                archived_count,
                retention_days,
                archive_path
            );
        }

        // Rewrite main audit log with only kept entries
        if !kept_entries.is_empty() {
            let temp_path = self.file_path.with_extension("jsonl.tmp");
            let mut temp_file = File::create(&temp_path)?;

            for entry in &kept_entries {
                let json = serde_json::to_string(entry)?;
                writeln!(temp_file, "{}", json)?;
            }
            temp_file.sync_all()?;

            // Replace old file with new
            std::fs::rename(&temp_path, &self.file_path)?;

            // Update last hash
            if let Some(last_entry) = kept_entries.last() {
                self.last_hash = Some(last_entry.hash.clone());
            } else {
                self.last_hash = None;
            }
        } else {
            // No entries to keep, remove the file
            std::fs::remove_file(&self.file_path)?;
            self.last_hash = None;
        }

        Ok(archived_count)
    }

    /// Get statistics about the audit log
    pub fn stats(&self) -> AdminResult<AuditLogStats> {
        if !self.file_path.exists() {
            return Ok(AuditLogStats {
                total_entries: 0,
                oldest_entry: None,
                newest_entry: None,
                file_size_bytes: 0,
            });
        }

        let metadata = std::fs::metadata(&self.file_path)?;
        let file_size = metadata.len();

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);

        let mut total = 0;
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                total += 1;

                if oldest.is_none() || entry.timestamp < oldest.unwrap() {
                    oldest = Some(entry.timestamp);
                }
                if newest.is_none() || entry.timestamp > newest.unwrap() {
                    newest = Some(entry.timestamp);
                }
            }
        }

        Ok(AuditLogStats {
            total_entries: total,
            oldest_entry: oldest,
            newest_entry: newest,
            file_size_bytes: file_size,
        })
    }
}

/// Statistics about the audit log
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuditLogStats {
    pub total_entries: usize,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
    pub file_size_bytes: u64,
}

/// Builder for creating audit entries
#[derive(Debug)]
pub struct AuditEntryBuilder {
    pub operator: String,
    pub role: Role,
    pub action: AuditAction,
    pub changes: Vec<SettingChange>,
    pub applied: bool,
    pub warnings_shown: Vec<String>,
    pub confirmation: Option<String>,
}

impl AuditEntryBuilder {
    /// Create a new audit entry builder
    pub fn new(operator: String, role: Role, action: AuditAction) -> Self {
        Self {
            operator,
            role,
            action,
            changes: Vec::new(),
            applied: false,
            warnings_shown: Vec::new(),
            confirmation: None,
        }
    }

    /// Add a setting change
    pub fn with_change(mut self, change: SettingChange) -> Self {
        self.changes.push(change);
        self
    }

    /// Set applied flag
    pub fn applied(mut self, applied: bool) -> Self {
        self.applied = applied;
        self
    }

    /// Add warnings shown
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings_shown = warnings;
        self
    }

    /// Set confirmation message
    pub fn with_confirmation(mut self, confirmation: String) -> Self {
        self.confirmation = Some(confirmation);
        self
    }

    /// Build the audit entry with hash chaining
    fn build(self, prev_hash: &str) -> AuditEntry {
        AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operator: self.operator,
            role: self.role,
            action: self.action,
            changes: self.changes,
            applied: self.applied,
            warnings_shown: self.warnings_shown,
            confirmation: self.confirmation,
            prev_hash: prev_hash.to_string(),
            hash: String::new(), // Will be calculated by AuditLog
        }
    }
}

/// Filter for querying audit log entries
#[derive(Debug, Default)]
pub struct AuditFilter {
    /// Filter by start time
    pub start_time: Option<DateTime<Utc>>,

    /// Filter by end time
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by operator name
    pub operator: Option<String>,

    /// Filter by setting name
    pub setting: Option<String>,

    /// Filter by action type
    pub action: Option<AuditAction>,

    /// Pagination: limit
    pub limit: Option<usize>,

    /// Pagination: offset
    pub offset: Option<usize>,
}

impl AuditFilter {
    /// Check if an entry matches the filter
    fn matches(&self, entry: &AuditEntry) -> bool {
        // Check timestamp range
        if let Some(start) = self.start_time {
            if entry.timestamp < start {
                return false;
            }
        }
        if let Some(end) = self.end_time {
            if entry.timestamp > end {
                return false;
            }
        }

        // Check operator
        if let Some(ref operator) = self.operator {
            if entry.operator != *operator {
                return false;
            }
        }

        // Check setting
        if let Some(ref setting) = self.setting {
            if !entry.changes.iter().any(|c| c.name == *setting) {
                return false;
            }
        }

        // Check action
        if let Some(action) = self.action {
            if entry.action != action {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_audit_log_creation() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");

        let log = AuditLog::new(&log_path);
        assert!(log.is_ok());
    }

    #[test]
    fn test_audit_entry_append() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");

        let mut log = AuditLog::new(&log_path).unwrap();

        let builder = AuditEntryBuilder::new(
            "test-user".to_string(),
            Role::Admin,
            AuditAction::ConfigChange,
        ).applied(true);

        let entry = log.append(builder);
        assert!(entry.is_ok());

        let entry = entry.unwrap();
        assert_eq!(entry.operator, "test-user");
        assert_eq!(entry.prev_hash, "");
        assert!(!entry.hash.is_empty());
    }

    #[test]
    fn test_hash_chaining() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");

        let mut log = AuditLog::new(&log_path).unwrap();

        // First entry
        let builder1 = AuditEntryBuilder::new(
            "user1".to_string(),
            Role::Admin,
            AuditAction::ConfigChange,
        );
        let entry1 = log.append(builder1).unwrap();

        // Second entry should chain to first
        let builder2 = AuditEntryBuilder::new(
            "user2".to_string(),
            Role::Operator,
            AuditAction::ConfigChange,
        );
        let entry2 = log.append(builder2).unwrap();

        assert_eq!(entry2.prev_hash, entry1.hash);
    }

    #[test]
    fn test_integrity_verification() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("audit.jsonl");

        let mut log = AuditLog::new(&log_path).unwrap();

        // Add multiple entries
        for i in 0..5 {
            let builder = AuditEntryBuilder::new(
                format!("user{}", i),
                Role::Admin,
                AuditAction::ConfigChange,
            );
            log.append(builder).unwrap();
        }

        // Verify integrity
        assert!(log.verify_integrity().unwrap());
    }
}
