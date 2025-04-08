//! 共享類型模組
//!
//! 這個模組包含了應用程序中共享的數據類型和結構。

/// 連接信息
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// 來源地址
    pub source: String,
    /// 目標地址
    pub target: String,
    /// 連接時間戳
    pub timestamp: std::time::SystemTime,
}

/// 證書信息
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    /// 證書主題
    pub subject: String,
    /// 證書指紋
    pub fingerprint: Option<String>,
    /// 是否為混合證書
    pub is_hybrid: bool,
}
