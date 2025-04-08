//! 文件系統相關工具函數
//!
//! 這個模組提供了文件系統相關的工具函數。

use std::path::Path;
use std::fs;

use super::error::{ProxyError, Result};

/// 檢查文件是否存在
///
/// # 參數
///
/// * `path` - 文件路徑
///
/// # 返回
///
/// 如果文件存在，返回 `Ok(())`，否則返回錯誤。
pub fn check_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(ProxyError::Config(format!(
            "文件不存在: {:?}",
            path
        )));
    }
    
    if !path.is_file() {
        return Err(ProxyError::Config(format!(
            "路徑不是文件: {:?}",
            path
        )));
    }
    
    Ok(())
}

/// 讀取文件內容
///
/// # 參數
///
/// * `path` - 文件路徑
///
/// # 返回
///
/// 返回文件內容的字節數組。
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    check_file_exists(path)?;
    
    fs::read(path).map_err(|e| ProxyError::Io(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_check_file_exists() {
        // 測試存在的文件
        let path = PathBuf::from("Cargo.toml");
        let result = check_file_exists(&path);
        assert!(result.is_ok(), "應該能夠檢查存在的文件");
        
        // 測試不存在的文件
        let path = PathBuf::from("non_existent_file.txt");
        let result = check_file_exists(&path);
        assert!(result.is_err(), "應該無法檢查不存在的文件");
    }
    
    #[test]
    fn test_read_file() {
        // 測試讀取存在的文件
        let path = PathBuf::from("Cargo.toml");
        let result = read_file(&path);
        assert!(result.is_ok(), "應該能夠讀取存在的文件");
        
        if let Ok(content) = result {
            assert!(!content.is_empty(), "文件內容不應為空");
        }
        
        // 測試讀取不存在的文件
        let path = PathBuf::from("non_existent_file.txt");
        let result = read_file(&path);
        assert!(result.is_err(), "應該無法讀取不存在的文件");
    }
}
