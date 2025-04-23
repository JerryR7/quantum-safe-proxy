//! TLS 處理模組
//!
//! 這個模組處理 TLS 連接和證書相關的功能。

mod acceptor;
mod cert;
pub mod strategy;

pub use acceptor::create_tls_acceptor;
pub use cert::{is_hybrid_cert, get_cert_subject, get_cert_fingerprint, load_cert};
