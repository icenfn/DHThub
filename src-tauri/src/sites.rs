//! 站点服务：订阅源管理、自定义站点、开关/默认引擎、导入导出

use crate::models::{SiteConfig, SitesData};
use chrono::Local;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::Mutex;

pub struct SiteStore {
    path: PathBuf,
    pub data: Mutex<SitesData>,
}

impl SiteStore {
    pub async fn new(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("sites.json");
        let data = if let Ok(text) = tokio::fs::read_to_string(&path).await {
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            SitesData::default()
        };
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    async fn save(&self, data: &SitesData) -> Result<(), String> {
        let text = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            let _ = tokio::fs::create_dir_all(dir).await;
        }
        tokio::fs::write(&self.path, text)
            .await
            .map_err(|e| format!("写入失败: {e}"))
    }

    /// 合并后的全部站点（已应用用户开关与默认标记），返回轻量列表供前端展示
    pub async fn all_sites(&self) -> Result<Vec<SiteConfig>, String> {
        let data = self.data.lock().await;
        Ok(build_merged(&data))
    }

    /// 导入订阅文本并替换 subscribed 列表（网络请求由前端 plugin-http 完成）
    pub async fn subscribe_from_text(
        &self,
        text: &str,
        url: &str,
    ) -> Result<Vec<SiteConfig>, String> {
        let parsed: Value =
            serde_json::from_str(text).map_err(|e| format!("订阅源 JSON 解析失败: {e}"))?;

        let mut list: Vec<SiteConfig> = if parsed.is_array() {
            serde_json::from_value(parsed).map_err(|e| format!("站点列表格式错误: {e}"))?
        } else if parsed.get("sites").is_some() {
            serde_json::from_value(parsed["sites"].clone())
                .map_err(|e| format!("站点列表格式错误: {e}"))?
        } else {
            return Err("订阅源中未找到站点数组".into());
        };

        // 补全缺失 id
        for (i, site) in list.iter_mut().enumerate() {
            if site.id.is_none() || site.id.as_ref().unwrap().trim().is_empty() {
                site.id = Some(format!("s{}", i + 1));
            }
        }

        let mut data = self.data.lock().await;
        data.subscribe_url = url.to_string();
        data.subscribed = list.clone();
        data.subscribed_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        // 订阅即默认启用：清空这些站点的开关覆盖，回落到源内 state（true）
        let ids: std::collections::HashSet<String> =
            list.iter().filter_map(|s| s.id.clone()).collect();
        data.enabled.retain(|k, _| !ids.contains(k));
        self.save(&data).await?;
        drop(data);
        Ok(build_merged(&*self.data.lock().await))
    }

    /// 新增自定义站点
    pub async fn add_custom(&self, mut site: SiteConfig) -> Result<Vec<SiteConfig>, String> {
        if site.name.trim().is_empty() || site.request.search_url.trim().is_empty() {
            return Err("站点名称与搜索地址不能为空".into());
        }
        let mut data = self.data.lock().await;
        // 生成唯一 id
        let next = data.custom.len() + 1;
        let mut id = format!("c{next}");
        loop {
            if data
                .subscribed
                .iter()
                .any(|s| s.id.as_deref() == Some(id.as_str()))
                || data
                    .custom
                    .iter()
                    .any(|s| s.id.as_deref() == Some(id.as_str()))
            {
                id = format!("c{}", next + data.custom.len());
            } else {
                break;
            }
        }
        site.id = Some(id.clone());
        site.is_custom = true;
        site.state = true;
        data.custom.push(site);
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }

    /// 更新站点：订阅源与自定义站点均可修改（保留原归属标记）
    pub async fn update_custom(&self, site: SiteConfig) -> Result<Vec<SiteConfig>, String> {
        let mut data = self.data.lock().await;
        let id = site.id.clone().unwrap_or_default();
        let mut found = false;
        if let Some(idx) = data
            .custom
            .iter()
            .position(|s| s.id.as_deref() == Some(id.as_str()))
        {
            data.custom[idx] = site;
            found = true;
        } else if let Some(idx) = data
            .subscribed
            .iter()
            .position(|s| s.id.as_deref() == Some(id.as_str()))
        {
            data.subscribed[idx] = site;
            found = true;
        }
        if !found {
            return Err("未找到该站点".into());
        }
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }

    /// 删除站点：订阅源与自定义站点均直接移除
    pub async fn delete_site(&self, id: &str) -> Result<Vec<SiteConfig>, String> {
        let mut data = self.data.lock().await;
        data.subscribed.retain(|s| s.id.as_deref() != Some(id));
        data.custom.retain(|s| s.id.as_deref() != Some(id));
        data.enabled.remove(id);
        if data.default_id.as_deref() == Some(id) {
            data.default_id = None;
        }
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }

    /// 设置开关
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<Vec<SiteConfig>, String> {
        let mut data = self.data.lock().await;
        data.enabled.insert(id.to_string(), enabled);
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }

    /// 设置默认搜索源
    pub async fn set_default(&self, id: Option<String>) -> Result<Vec<SiteConfig>, String> {
        let mut data = self.data.lock().await;
        data.default_id = id;
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }

    /// 导出完整站点数据（B8）
    pub async fn export(&self) -> Result<String, String> {
        let data = self.data.lock().await;
        serde_json::to_string_pretty(&*data).map_err(|e| e.to_string())
    }

    /// 导入站点数据（B8）：替换自定义站点与开关配置，保留当前订阅
    pub async fn import(&self, json: &str) -> Result<Vec<SiteConfig>, String> {
        let incoming: SitesData =
            serde_json::from_str(json).map_err(|e| format!("导入数据解析失败: {e}"))?;
        let mut data = self.data.lock().await;
        if !incoming.custom.is_empty() {
            data.custom = incoming.custom;
        }
        data.enabled.extend(incoming.enabled);
        // 订阅源站点导入后默认启用（清空开关覆盖回落 state）
        let ids: std::collections::HashSet<String> = data
            .subscribed
            .iter()
            .filter_map(|s| s.id.clone())
            .collect();
        data.enabled.retain(|k, _| !ids.contains(k));
        if incoming.default_id.is_some() {
            data.default_id = incoming.default_id;
        }
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }

    /// 恢复出厂：清空订阅与自定义（保留开关结构）
    pub async fn reset(&self) -> Result<Vec<SiteConfig>, String> {
        let mut data = self.data.lock().await;
        data.subscribed = vec![];
        data.custom = vec![];
        data.enabled.clear();
        data.default_id = None;
        data.subscribe_url = String::new();
        data.subscribed_at = None;
        let merged = build_merged(&data);
        self.save(&data).await?;
        drop(data);
        Ok(merged)
    }
}

/// 合并订阅 + 自定义，应用开关与默认标记
fn build_merged(data: &SitesData) -> Vec<SiteConfig> {
    let mut merged: Vec<SiteConfig> = Vec::new();
    for mut s in data.subscribed.clone() {
        s.is_custom = false;
        s.enabled = data
            .enabled
            .get(s.id.as_deref().unwrap_or(""))
            .copied()
            .unwrap_or(s.state);
        s.is_default = data.default_id.as_deref() == Some(s.id.as_deref().unwrap_or(""));
        merged.push(s);
    }
    for mut s in data.custom.clone() {
        s.is_custom = true;
        s.enabled = data
            .enabled
            .get(s.id.as_deref().unwrap_or(""))
            .copied()
            .unwrap_or(true);
        s.is_default = data.default_id.as_deref() == Some(s.id.as_deref().unwrap_or(""));
        merged.push(s);
    }
    merged
}
