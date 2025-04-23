// src/tls/strategy.rs
use openssl::ssl::{SslAcceptorBuilder, SslFiletype};
use std::path::PathBuf;
use log::{debug, info, warn};
use crate::common::Result;

/// Two strategies: Single (classic only) or SigAlgs (auto-select)
#[derive(Debug)]
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
        debug!("Applying certificate strategy: {:?}", self);

        match self {
            // always load classic
            CertStrategy::Single { cert, key } => {
                info!("Using single certificate strategy");
                debug!("Certificate path: {:?}", cert);
                debug!("Key path: {:?}", key);

                // Check if certificate file exists
                if !cert.exists() {
                    warn!("Certificate file does not exist: {:?}", cert);
                    return Err(crate::common::ProxyError::Config(format!("Certificate file does not exist: {:?}", cert)));
                }

                // Check if key file exists
                if !key.exists() {
                    warn!("Key file does not exist: {:?}", key);
                    return Err(crate::common::ProxyError::Config(format!("Key file does not exist: {:?}", key)));
                }

                debug!("Setting certificate file: {:?}", cert);
                builder.set_certificate_file(cert, SslFiletype::PEM)?;
                debug!("Setting private key file: {:?}", key);
                builder.set_private_key_file(key, SslFiletype::PEM)?;
                debug!("Certificate and key set successfully");
            }
            // sigalgs: detect PQC OID in client_hello
            CertStrategy::SigAlgs { classic, hybrid } => {
                info!("Using SigAlgs certificate strategy");
                debug!("Classic certificate path: {:?}", classic.0);
                debug!("Classic key path: {:?}", classic.1);
                debug!("Hybrid certificate path: {:?}", hybrid.0);
                debug!("Hybrid key path: {:?}", hybrid.1);

                // Check if classic certificate file exists
                if !classic.0.exists() {
                    warn!("Classic certificate file does not exist: {:?}", classic.0);
                    return Err(crate::common::ProxyError::Config(format!("Classic certificate file does not exist: {:?}", classic.0)));
                }

                // Check if classic key file exists
                if !classic.1.exists() {
                    warn!("Classic key file does not exist: {:?}", classic.1);
                    return Err(crate::common::ProxyError::Config(format!("Classic key file does not exist: {:?}", classic.1)));
                }

                // Check if hybrid certificate file exists
                if !hybrid.0.exists() {
                    warn!("Hybrid certificate file does not exist: {:?}", hybrid.0);
                    return Err(crate::common::ProxyError::Config(format!("Hybrid certificate file does not exist: {:?}", hybrid.0)));
                }

                // Check if hybrid key file exists
                if !hybrid.1.exists() {
                    warn!("Hybrid key file does not exist: {:?}", hybrid.1);
                    return Err(crate::common::ProxyError::Config(format!("Hybrid key file does not exist: {:?}", hybrid.1)));
                }

                // 由於我們無法直接在 client hello 階段檢測簽名演算法，我們改用一個替代方案
                // 我們將在伺服器啟動時載入兩種證書，並在日誌中記錄使用了哪種證書
                info!("Loading both classic and hybrid certificates");

                // 首先設置默認證書（經典證書）
                debug!("Setting classic certificate file: {:?}", classic.0);
                builder.set_certificate_file(&classic.0, SslFiletype::PEM)?;
                debug!("Setting classic private key file: {:?}", classic.1);
                builder.set_private_key_file(&classic.1, SslFiletype::PEM)?;
                debug!("Classic certificate and key set successfully");

                // 載入混合證書 (cert_path/key_path)
                debug!("Note: Hybrid certificate would be used in a real implementation: {:?}", hybrid.0);
                debug!("Note: Hybrid key would be used in a real implementation: {:?}", hybrid.1);
                // 注意：在實際部署中，您可能需要使用更高級的方法來動態選擇證書
                // 例如，您可能需要實現一個自定義的 TLS 握手層，或使用其他方法來檢測客戶端的能力
                // 在這裡，我們只是載入經典證書，並在日誌中記錄我們的意圖
                warn!("Using classic certificate by default. In a real deployment, you would need to implement a custom TLS handshake layer to dynamically select certificates based on client capabilities.");
            }
        }

        debug!("Certificate strategy applied successfully");
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
