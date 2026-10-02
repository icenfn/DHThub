//! DHThub Tauri 应用入口：插件注册、状态管理、命令路由

mod history;
mod models;
mod search;
mod sites;
mod update;

use history::{HistoryStore, KIND_BROWSE, KIND_COPY, KIND_MAGNET};
use models::{SiteConfig, SiteOutcome, UpdateCheckResult};
use sites::SiteStore;
use std::time::Duration;
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

// ---------- 站点 ----------

#[tauri::command]
async fn get_sites(state: tauri::State<'_, AppState>) -> Result<Vec<SiteConfig>, String> {
    state.sites.all_sites().await
}

#[tauri::command]
async fn subscribe_sites(
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<Vec<SiteConfig>, String> {
    state.sites.subscribe(&state.http, &url).await
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

#[tauri::command]
async fn fetch_text(state: tauri::State<'_, AppState>, url: String) -> Result<String, String> {
    search::fetch_text(&state.http, &url).await
}

// ---------- 搜索 ----------

#[tauri::command]
async fn search_sites(
    state: tauri::State<'_, AppState>,
    keyword: String,
    site_ids: Option<Vec<String>>,
    page: u32,
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
        return Err("没有启用的搜索源，请先在「站点管理」中启用或订阅".into());
    }
    let page = page.max(1);
    Ok(search::search_multi(&state.http, sites, &keyword, page).await)
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

#[tauri::command]
async fn check_update(
    state: tauri::State<'_, AppState>,
    mirror_base: Option<String>,
) -> Result<UpdateCheckResult, String> {
    update::check_update(
        &state.http,
        env!("CARGO_PKG_VERSION"),
        mirror_base.as_deref(),
    )
    .await
}

// ---------- 应用启动 ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = tauri::async_runtime::block_on(AppState::new(app.handle()));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sites,
            subscribe_sites,
            add_custom_site,
            update_custom_site,
            delete_site,
            set_site_enabled,
            set_default_site,
            export_sites,
            import_sites,
            reset_sites,
            fetch_text,
            search_sites,
            add_history,
            get_history,
            clear_history,
            clear_all_history,
            check_update,
        ])
        .run(tauri::generate_context!())
        .expect("DHThub 启动失败");
}
