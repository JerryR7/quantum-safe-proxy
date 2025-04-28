//! Protocol detection module
//!
//! This module provides functionality for detecting the protocol of a connection
//! by examining the first few bytes of data. It is designed to be efficient and
//! non-blocking, similar to how NGINX and HAProxy implement protocol detection.
//!
//! The module uses Rust's trait system to provide a clean, extensible interface
//! for protocol detection.

mod detector;

pub use detector::{ProtocolDetector, TlsDetector, ProtocolInfo, DetectionResult};
