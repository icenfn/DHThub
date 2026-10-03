//! DHThub Tauri 应用入口：插件注册、状态管理、命令路由

mod dns;
mod history;
mod models;
mod search;
mod sites;

use history::{HistoryStore, KIND_BROWSE, KIND_COPY, KIND_MAGNET};
use models::{SiteConfig, SiteOutcome, SiteTestResult};
use sites::SiteStore;
use std::time::{Duration, Instant};
use tauri::Manager;

/// 全局状态
pub struct AppState {
    pub http: reqwest::Client,
    pub sites: SiteStore,
    pub history: HistoryStore,
}

impl AppState {
    pub async fn new(app: &tauri::AppHandle) -> Self {
        let app_data = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        Self {
            http: reqwest::Client::builder()
                .user_agent(format!(
                    "DHThub/{env} (+https://github.com/icenfn/DHThub)",
                    env = env!("CARGO_PKG_VERSION")
                ))
                // DNS/建连挂起也有界，避免订阅/搜索等请求无限转圈
                .connect_timeout(Duration::from_secs(8))
                .timeout(Duration::from_secs(20))
                .build()
                .expect("构建 HTTP 客户端失败"),
            sites: SiteStore::new(app_data.clone()).await,
            history: HistoryStore::new(app_data).await,
        }
    }
}

// ---------- DNS 测速 ----------

/// 对一组 DNS 服务器并发测速（example.com 解析耗时），返回 [地址, 毫秒]
#[tauri::command]
async fn test_dns_latencies(servers: Vec<String>) -> Result<Vec<(String, u64)>, String> {
    let mut tasks = tokio::task::JoinSet::new();
    for srv in servers {
        let s = srv.clone();
        tasks.spawn(async move { (s.clone(), dns::measure_latency(&s).await) });
    }
    let mut out = Vec::new();
    while let Some(res) = tasks.join_next().await {
        if let Ok((srv, Ok(ms))) = res {
            out.push((srv, ms));
        }
    }
    Ok(out)
}

// ---------- 站点 ----------

#[tauri::command]
async fn get_sites(state: tauri::State<'_, AppState>) -> Result<Vec<SiteConfig>, String> {
    state.sites.all_sites().await
}

#[tauri::command]
async fn subscribe_sites(
    state: tauri::State<'_, AppState>,
    text: String,
    url: String,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.subscribe_from_text(&text, &url).await
}

#[tauri::command]
async fn add_custom_site(
    state: tauri::State<'_, AppState>,
    site: SiteConfig,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.add_custom(site).await
}

#[tauri::command]
async fn update_custom_site(
    state: tauri::State<'_, AppState>,
    site: SiteConfig,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.update_custom(site).await
}

#[tauri::command]
async fn delete_site(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.delete_site(&id).await
}

#[tauri::command]
async fn set_site_enabled(
    state: tauri::State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.set_enabled(&id, enabled).await
}

#[tauri::command]
async fn set_default_site(
    state: tauri::State<'_, AppState>,
    id: Option<String>,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.set_default(id).await
}

#[tauri::command]
async fn export_sites(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.sites.export().await
}

#[tauri::command]
async fn import_sites(
    state: tauri::State<'_, AppState>,
    json: String,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.import(&json).await
}

#[tauri::command]
async fn reset_sites(state: tauri::State<'_, AppState>) -> Result<Vec<SiteConfig>, String> {
    state.sites.reset().await
}

// ---------- 搜索 ----------

/// 多源并发搜索：一次性返回全部站点结果（v0.3.7 行为）
#[tauri::command]
async fn search_sites(
    state: tauri::State<'_, AppState>,
    keyword: String,
    site_ids: Option<Vec<String>>,
    page: u32,
    dns: String,
) -> Result<Vec<SiteOutcome>, String> {
    if keyword.trim().is_empty() {
        return Err("搜索关键词不能为空".into());
    }
    let mut sites = state.sites.all_sites().await?;
    // 仅保留启用站点
    sites.retain(|s| s.enabled);
    if let Some(ids) = site_ids {
        if !ids.is_empty() {
            sites.retain(|s| ids.iter().any(|i| i == s.id.as_deref().unwrap_or("")));
        }
    }
    if sites.is_empty() {
        return Err("没有启用的搜索源，请先在「搜索源」页启用或订阅".into());
    }
    let page = page.max(1);
    let client = match dns.trim() {
        d if d.is_empty() => state.http.clone(),
        d => dns::build_client(d)?,
    };
    Ok(search::search_multi(&client, sites, &keyword, page).await)
}

/// 退出应用（Android 返回键兜底：无弹窗且无历史时调用）
#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// 站点连接测试：用「test」关键词请求一次并解析，返回耗时/条目/样例
#[tauri::command]
async fn test_site(
    state: tauri::State<'_, AppState>,
    site: SiteConfig,
    dns: String,
) -> Result<SiteTestResult, String> {
    let client = match dns.trim() {
        d if d.is_empty() => state.http.clone(),
        d => dns::build_client(d)?,
    };
    let url = search::build_url(&site, "test", 1);
    let started = Instant::now();
    match search::fetch_html(&client, &site, &url).await {
        Ok(html) => {
            let items = search::parse_html(&html, &site.expression_model);
            Ok(SiteTestResult {
                ok: true,
                elapsed_ms: started.elapsed().as_millis() as u64,
                items: items.len(),
                error: None,
                samples: items.iter().take(5).map(|i| i.title.clone()).collect(),
            })
        }
        Err(e) => Ok(SiteTestResult {
            ok: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
            items: 0,
            error: Some(e),
            samples: vec![],
        }),
    }
}

// ---------- 历史 ----------

#[tauri::command]
async fn add_history(
    state: tauri::State<'_, AppState>,
    kind: String,
    keyword: String,
    magnet: String,
) -> Result<(), String> {
    if kind != KIND_MAGNET && kind != KIND_COPY && kind != KIND_BROWSE {
        return Err("未知历史类型".into());
    }
    if keyword.is_empty() && magnet.is_empty() {
        return Ok(());
    }
    state.history.add(&kind, &keyword, &magnet).await
}

#[tauri::command]
async fn get_history(
    state: tauri::State<'_, AppState>,
    kind: String,
) -> Result<Vec<models::HistoryEntry>, String> {
    if kind != KIND_MAGNET && kind != KIND_COPY && kind != KIND_BROWSE {
        return Err("未知历史类型".into());
    }
    Ok(state.history.list(&kind).await)
}

#[tauri::command]
async fn clear_history(state: tauri::State<'_, AppState>, kind: String) -> Result<(), String> {
    state.history.clear(&kind).await
}

#[tauri::command]
async fn clear_all_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.history.clear_all().await
}

// ---------- 更新 ----------

// 更新检测已在纯前端实现（src/lib/update.ts，plugin-http fetch releases/latest + snackbar），Rust 侧不再参与

// ---------- 应用启动 ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = tauri::async_runtime::block_on(AppState::new(app.handle()));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sites,
            test_dns_latencies,
            subscribe_sites,
            add_custom_site,
            update_custom_site,
            delete_site,
            set_site_enabled,
            set_default_site,
            export_sites,
            import_sites,
            reset_sites,
            search_sites,
            test_site,
            exit_app,
            add_history,
            get_history,
            clear_history,
            clear_all_history
        ])
        .run(tauri::generate_context!())
        .expect("DHThub 启动失败");
}
