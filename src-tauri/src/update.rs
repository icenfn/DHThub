//! 更新服务：通过 GitHub Releases API 检查更新；Android 端下载 APK

use crate::models::{AssetInfo, UpdateInfo};
use futures::StreamExt;
use serde_json::Value;
use std::time::Instant;
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;

/// DHThub 发布仓库（可在设置页通过环境变量/配置覆盖）
pub const REPO: &str = "icenfn/DHThub";

/// 检查最新版本
pub async fn check_update(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(20))
        .header("User-Agent", "DHThub/0.1 (+https://github.com/icenfn/DHThub)")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub 失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API HTTP {}", resp.status()));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("解析更新信息失败: {e}"))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let notes = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let published_at = json
        .get("published_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut assets: std::collections::HashMap<String, Vec<AssetInfo>> = Default::default();
    if let Some(list) = json.get("assets").and_then(|v| v.as_array()) {
        for a in list {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let url = a
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let size = a.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            if name.is_empty() || url.is_empty() {
                continue;
            }
            let platform = classify_asset(&name);
            if let Some(p) = platform {
                assets.entry(p.to_string()).or_default().push(AssetInfo {
                    name,
                    url,
                    size,
                });
            }
        }
    }

    let has_update = version_gt(&tag, current_version);
    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: tag,
        has_update,
        notes,
        published_at,
        assets,
    })
}

/// 按文件名归类平台资产
fn classify_asset(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    if lower.ends_with(".apk") {
        return Some("android");
    }
    if lower.ends_with(".aab") {
        return Some("android");
    }
    if lower.ends_with(".deb") || lower.ends_with(".rpm") || lower.ends_with(".appimage") {
        return Some("linux");
    }
    if lower.ends_with(".msi") || lower.ends_with(".exe") || lower.ends_with(".nsis") {
        return Some("windows");
    }
    if lower.ends_with(".dmg") || lower.ends_with(".tar.gz") {
        // tar.gz 同时用于 linux AppImage 与 mac 更新包，归入 linux 便于展示
        return Some("linux");
    }
    None
}

/// 简单版本号比较（支持 x.y.z / x.y.z-b）
fn version_gt(new: &str, cur: &str) -> bool {
    if new.is_empty() {
        return false;
    }
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-'])
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };
    let a = parse(new);
    let b = parse(cur);
    for i in 0..a.len().max(b.len()) {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        if av != bv {
            return av > bv;
        }
    }
    false
}

/// 下载 APK 到应用缓存目录，期间通过事件上报进度
pub async fn download_apk(
    app: tauri::AppHandle,
    client: &reqwest::Client,
    url: &str,
) -> Result<String, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("无法获取缓存目录: {e}"))?
        .join("update");
    let _ = std::fs::create_dir_all(&dir);

    let file_name = url
        .rsplit('/')
        .next()
        .filter(|n| n.ends_with(".apk"))
        .unwrap_or("DHThub.apk");
    let dest = dir.join(file_name);

    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(120))
        .header("User-Agent", "DHThub/0.1 (+https://github.com/icenfn/DHThub)")
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("下载失败: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| format!("创建文件失败: {e}"))?;
    let mut received: u64 = 0;
    let started = Instant::now();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断: {e}"))?;
        received += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|e| format!("写入失败: {e}"))?;
        let percent = if total > 0 {
            (received as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        let _ = app.emit(
            "apk-download-progress",
            serde_json::json!({
                "received": received,
                "total": total,
                "percent": percent,
                "speed": if started.elapsed().as_secs() > 0 {
                    (received as f64 / started.elapsed().as_secs_f64() / 1024.0) as u64
                } else { 0 }
            }),
        );
    }
    file.flush().await.map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

/// 安装 APK：Android 上通过系统安装器；其他平台不支持
#[tauri::command]
pub async fn install_apk(app: tauri::AppHandle, path: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_shell::ShellExt;
        app.shell()
            .open(path, None)
            .map_err(|e| format!("拉起系统安装器失败: {e}"))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (&app, path);
        Err("当前平台不支持直接安装 APK，请手动安装下载的安装包".into())
    }
}
