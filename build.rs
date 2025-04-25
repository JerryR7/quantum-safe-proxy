// build.rs
use std::path::Path;
use std::process::Command;
use std::env;

fn main() {
    // 檢查常見的 OpenSSL 3.5+ 安裝位置
    let openssl35_locations = [
        "/opt/openssl-3.5.0",
        "/opt/openssl35",
        "/usr/local/openssl-3.5.0",
        "/usr/local/openssl35",
        "/usr/local/opt/openssl-3.5.0",
        "/usr/local/opt/openssl35",
    ];

    // 首先檢查環境變量
    let openssl_dir = if let Ok(dir) = env::var("OPENSSL_DIR") {
        // 用戶指定了 OPENSSL_DIR，使用它
        dir
    } else {
        // 嘗試自動檢測 OpenSSL 3.5+
        let mut detected_dir = None;
        
        for &location in &openssl35_locations {
            let path = Path::new(location);
            if path.exists() {
                // 檢查是否為 OpenSSL 3.5+
                let openssl_bin = path.join("bin").join("openssl");
                if openssl_bin.exists() {
                    let output = Command::new(&openssl_bin)
                        .arg("version")
                        .output();
                        
                    if let Ok(output) = output {
                        let version = String::from_utf8_lossy(&output.stdout);
                        if version.contains("3.5") || version.contains("3.6") || version.contains("3.7") {
                            println!("cargo:warning=Detected OpenSSL 3.5+ at: {}", location);
                            detected_dir = Some(location.to_string());
                            break;
                        }
                    }
                }
            }
        }
        
        // 如果找不到 OpenSSL 3.5+，使用默認值
        detected_dir.unwrap_or_else(|| {
            println!("cargo:warning=OpenSSL 3.5+ not found, using default OpenSSL");
            "/opt/openssl".to_string()
        })
    };

    println!("cargo:warning=Using OpenSSL directory: {}", openssl_dir);
    
    // 檢查 lib 和 lib64 目錄
    let lib_dir = Path::new(&openssl_dir).join("lib");
    let lib64_dir = Path::new(&openssl_dir).join("lib64");
    
    // 設置庫搜索路徑
    if lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
    }
    
    if lib64_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
    }
    
    // 設置環境變量
    println!("cargo:rustc-env=OPENSSL_DIR={}", openssl_dir);
    
    if lib64_dir.exists() {
        println!("cargo:rustc-env=OPENSSL_LIB_DIR={}", lib64_dir.display());
    } else if lib_dir.exists() {
        println!("cargo:rustc-env=OPENSSL_LIB_DIR={}", lib_dir.display());
    }
    
    let include_dir = Path::new(&openssl_dir).join("include");
    if include_dir.exists() {
        println!("cargo:rustc-env=OPENSSL_INCLUDE_DIR={}", include_dir.display());
    }
    
    // 設置 rpath
    if lib_dir.exists() {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
    
    if lib64_dir.exists() {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib64_dir.display());
    }
    
    // 指定要鏈接的庫
    println!("cargo:rustc-link-lib=dylib=ssl");
    println!("cargo:rustc-link-lib=dylib=crypto");
    
    // 設置特性標誌，可以在代碼中使用 #[cfg(feature = "openssl35")]
    let openssl_bin = Path::new(&openssl_dir).join("bin").join("openssl");
    if openssl_bin.exists() {
        let output = Command::new(&openssl_bin)
            .arg("version")
            .output();
            
        if let Ok(output) = output {
            let version = String::from_utf8_lossy(&output.stdout);
            if version.contains("3.5") || version.contains("3.6") || version.contains("3.7") {
                println!("cargo:rustc-cfg=feature=\"openssl35\"");
                println!("cargo:warning=Enabled openssl35 feature flag");
                
                // 設置環境變量，使程序在運行時使用混合證書作為預設選項
                println!("cargo:rustc-env=USE_HYBRID_CERT_DEFAULT=true");
            }
        }
    }
    
    // 告訴 Cargo 如果 OpenSSL 文件改變，重新運行此腳本
    println!("cargo:rerun-if-changed={}", openssl_dir);
    println!("cargo:rerun-if-env-changed=OPENSSL_DIR");
}
