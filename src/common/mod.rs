//! 共享模組
//!
//! 這個模組包含了應用程序中共享的類型、錯誤和工具函數。

pub mod error;
pub mod types;
pub mod fs;
pub mod log;
pub mod net;

// 重新導出常用的類型和函數
pub use error::{ProxyError, Result};
pub use fs::{check_file_exists, read_file};
pub use log::init_logger;
pub use net::parse_socket_addr;
