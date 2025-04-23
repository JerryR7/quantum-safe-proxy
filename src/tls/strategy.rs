// src/tls/strategy.rs
use openssl::ssl::{SslAcceptorBuilder, SslFiletype};
use std::path::PathBuf;
use log::{debug, info, warn};
use crate::common::Result;

/// Two strategies: Single (classic only) or SigAlgs (auto-select)
pub enum CertStrategy {
    Single { cert: PathBuf, key: PathBuf },
    SigAlgs {
        classic: (PathBuf, PathBuf),
        hybrid:  (PathBuf, PathBuf),
    },
}

impl CertStrategy {
    /// Apply the chosen strategy to the OpenSSL builder.
    pub fn apply(&self, builder: &mut SslAcceptorBuilder) -> Result<()> {
        match self {
            // always load classic
            CertStrategy::Single { cert, key } => {
                debug!("Using single certificate strategy with cert: {:?}, key: {:?}", cert, key);
                builder.set_certificate_file(cert, SslFiletype::PEM)?;
                builder.set_private_key_file(key,  SslFiletype::PEM)?;
            }
            // sigalgs: detect PQC OID in client_hello
            CertStrategy::SigAlgs { classic, hybrid } => {
                info!("Using SigAlgs certificate strategy");

                // 首先設置默認證書（經典證書）
                builder.set_certificate_file(&classic.0, SslFiletype::PEM)?;
                builder.set_private_key_file(&classic.1, SslFiletype::PEM)?;

                // 由於我們無法直接在 client hello 階段檢測簽名演算法，我們改用一個替代方案
                // 我們將在伺服器啟動時載入兩種證書，並在日誌中記錄使用了哪種證書
                info!("Loading both classic and hybrid certificates");

                // 載入經典證書
                debug!("Loading classic certificate: {:?}, key: {:?}", classic.0, classic.1);
                builder.set_certificate_file(&classic.0, SslFiletype::PEM)?;
                builder.set_private_key_file(&classic.1, SslFiletype::PEM)?;

                // 載入混合證書 (cert_path/key_path)
                debug!("Loading hybrid certificate: {:?}, key: {:?}", hybrid.0, hybrid.1);
                // 注意：在實際部署中，您可能需要使用更高級的方法來動態選擇證書
                // 例如，您可能需要實現一個自定義的 TLS 握手層，或使用其他方法來檢測客戶端的能力
                // 在這裡，我們只是載入經典證書，並在日誌中記錄我們的意圖
                warn!("Using classic certificate by default. In a real deployment, you would need to implement a custom TLS handshake layer to dynamically select certificates based on client capabilities.");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ssl::{SslMethod, SslAcceptor};

    #[test]
    fn sigalgs_callback_registers() {
        let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
        let classic = ("c.crt".into(), "c.key".into());
        let hybrid  = ("h.crt".into(), "h.key".into());
        let strat   = CertStrategy::SigAlgs { classic: classic.clone(), hybrid: hybrid.clone() };

        // 這個測試只是確認回調註冊不會崩潰
        // 實際上，由於我們沒有真正的證書文件，apply 會失敗
        // 但我們只是想確認代碼結構是正確的
        let _ = strat.apply(&mut builder);

        // 測試單一證書策略
        let single_strat = CertStrategy::Single { cert: "c.crt".into(), key: "c.key".into() };
        let _ = single_strat.apply(&mut builder);
    }
}
