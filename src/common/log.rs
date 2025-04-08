//! 日誌相關工具函數
//!
//! 這個模組提供了日誌系統相關的工具函數。

/// 初始化日誌系統
///
/// # 參數
///
/// * `level` - 日誌級別
pub fn init_logger(level: &str) {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", level);
    
    env_logger::init_from_env(env);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_init_logger() {
        // 測試初始化日誌系統
        // 注意：這個測試可能會影響其他測試，因為它初始化了全局日誌系統
        // 所以我們只是簡單地確保函數不會崩潰
        init_logger("debug");
    }
}
