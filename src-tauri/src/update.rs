//! 更新服务：请求 GitHub Releases latest 页（跟随重定向），从最终地址解析最新版本号

use crate::models::UpdateCheckResult;
use std::time::Duration;

/// GitHub Releases latest 页（302 重定向到最新 tag 页，如 /releases/tag/v0.2.5）
const RELEASES_LATEST_URL: &str = "https://github.com/icenfn/DHThub/releases/latest";

/// 将 GitHub 原始 URL 应用到镜像前缀（直连时返回原 URL）
pub fn apply_mirror(mirror_base: Option<&str>, url: &str) -> String {
    match mirror_base.map(str::trim).filter(|b| !b.is_empty()) {
        Some(base) => format!(
            "{}/{}",
            base.trim_end_matches('/'),
            url.trim_start_matches('/')
        ),
        None => url.to_string(),
    }
}

/// 检查最新版本：请求 releases/latest，跟随重定向后解析最终 URL 中的 tag 版本号
pub async fn check_update(
    client: &reqwest::Client,
    current_version: &str,
    mirror_base: Option<&str>,
) -> Result<UpdateCheckResult, String> {
    let url = apply_mirror(mirror_base, RELEASES_LATEST_URL);
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(15))
        .header(
            "User-Agent",
            format!(
                "DHThub/{env} (+https://github.com/icenfn/DHThub)",
                env = env!("CARGO_PKG_VERSION")
            ),
        )
        .send()
        .await
        .map_err(|e| format!("请求 GitHub 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub 页面 HTTP {}", resp.status()));
    }
    // 重定向后的最终地址形如 https://github.com/icenfn/DHThub/releases/tag/v0.2.5
    let final_url = resp.url().as_str().to_string();
    let latest_version = extract_version_from_url(&final_url).unwrap_or_default();
    let has_update = version_gt(&latest_version, current_version);
    Ok(UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version,
        has_update,
    })
}

/// 从 URL 最后一段解析版本号（如 v0.2.5 → 0.2.5）
fn extract_version_from_url(url: &str) -> Option<String> {
    let seg = url
        .rsplit('/')
        .find(|s| s.starts_with('v') && s.len() > 1)?;
    let v = seg.trim_start_matches('v').to_string();
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() >= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        Some(v)
    } else {
        None
    }
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
