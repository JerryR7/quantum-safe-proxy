//! Embedded HTML UI Module
//!
//! This module provides the embedded HTML user interface for the admin API.

/// Return the embedded HTML UI
pub fn ui_html() -> &'static str {
    include_str!("../../web/admin-ui.html")
}
