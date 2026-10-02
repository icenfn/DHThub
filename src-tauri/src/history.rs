//! 历史记录服务：磁力/复制/浏览三类历史，JSON 持久化（每类保留最近 500 条）

use crate::models::HistoryEntry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

pub const KIND_MAGNET: &str = "magnet";
pub const KIND_COPY: &str = "copy";
pub const KIND_BROWSE: &str = "browse";

const MAX_PER_KIND: usize = 500;

pub struct HistoryStore {
    path: PathBuf,
    data: Mutex<HashMap<String, Vec<HistoryEntry>>>,
}

impl HistoryStore {
    pub async fn new(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("history.json");
        let data = if let Ok(text) = tokio::fs::read_to_string(&path).await {
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    async fn save(&self, data: &HashMap<String, Vec<HistoryEntry>>) -> Result<(), String> {
        let text = serde_json::to_string(data).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            let _ = tokio::fs::create_dir_all(dir).await;
        }
        tokio::fs::write(&self.path, text)
            .await
            .map_err(|e| format!("写入失败: {e}"))
    }

    pub async fn add(&self, kind: &str, keyword: &str, magnet: &str) -> Result<(), String> {
        let mut data = self.data.lock().await;
        let list = data.entry(kind.to_string()).or_default();
        let entry = HistoryEntry {
            keyword: keyword.to_string(),
            magnet: magnet.to_string(),
            time: now_ms(),
        };
        // 去重：同关键词同磁力则更新到最新
        if let Some(idx) = list
            .iter()
            .position(|e| e.keyword == entry.keyword && e.magnet == entry.magnet)
        {
            list.remove(idx);
        }
        list.insert(0, entry);
        if list.len() > MAX_PER_KIND {
            list.truncate(MAX_PER_KIND);
        }
        self.save(&data).await?;
        Ok(())
    }

    pub async fn list(&self, kind: &str) -> Vec<HistoryEntry> {
        self.data
            .lock()
            .await
            .get(kind)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn clear(&self, kind: &str) -> Result<(), String> {
        let mut data = self.data.lock().await;
        data.remove(kind);
        self.save(&data).await
    }

    pub async fn clear_all(&self) -> Result<(), String> {
        let mut data = self.data.lock().await;
        data.clear();
        self.save(&data).await
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
