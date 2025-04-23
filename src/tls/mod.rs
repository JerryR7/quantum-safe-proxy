//! TLS handling module
//!
//! This module handles TLS connections and certificate-related functionality.

mod acceptor;
mod cert;
pub mod strategy;

pub use acceptor::create_tls_acceptor;
pub use cert::{is_hybrid_cert, get_cert_subject, get_cert_fingerprint, load_cert};
