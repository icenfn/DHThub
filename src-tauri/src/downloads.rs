//! BitTorrent 下载：内嵌 librqbit（rqbit 的库形态）
//!
//! 设计要点：
//! - Session 懒初始化：首次调用下载相关命令时才创建，避免拖慢启动；
//! - 只用库 API，不启用 http-api 特性，减少交叉编译（Android/Windows）风险；
//! - 统计信息直接把 librqbit 的 `TorrentStats`（实现了 Serialize）转成 JSON 透出，
//!   前端按需取字段，避免在后端重复建模导致上游字段改动即编译失败。

use std::path::PathBuf;
use std::sync::Arc;

use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
};
use serde::Serialize;
use tokio::sync::Mutex;

/// 一条下载任务的对外视图
#[derive(Serialize)]
pub struct DownloadRow {
    /// librqbit 内的任务 id（进程内有效）
    pub id: usize,
    pub name: String,
    pub paused: bool,
    /// 原始 TorrentStats 序列化结果
    pub stats: serde_json::Value,
}

/// 下载管理器：持有输出目录与惰性初始化的会话
pub struct DownloadManager {
    output_dir: PathBuf,
    session: Mutex<Option<Arc<Session>>>,
}

impl DownloadManager {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            session: Mutex::new(None),
        }
    }

    /// 取（必要时创建）下载会话
    async fn session(&self) -> Result<Arc<Session>, String> {
        let mut guard = self.session.lock().await;
        if let Some(s) = guard.as_ref() {
            return Ok(s.clone());
        }
        tokio::fs::create_dir_all(&self.output_dir)
            .await
            .map_err(|e| format!("创建下载目录失败: {e}"))?;
        // 保持默认（DHT/追踪器开启）；不做会话持久化，进程退出即清空任务列表。
        let opts = SessionOptions {
            persistence: None,
            fastresume: false,
            ..Default::default()
        };
        let session = Session::new_with_opts(self.output_dir.clone(), opts)
            .await
            .map_err(|e| format!("初始化下载会话失败: {e:#}"))?;
        *guard = Some(session.clone());
        Ok(session)
    }

    /// 添加磁力链接或种子文件 URL，返回任务 id
    pub async fn add(&self, magnet: String) -> Result<usize, String> {
        let magnet = magnet.trim().to_string();
        if magnet.is_empty() {
            return Err("磁力链接不能为空".into());
        }
        let session = self.session().await?;
        let opts = AddTorrentOptions {
            // 允许续传/播种时覆盖已存在文件
            overwrite: true,
            ..Default::default()
        };
        let resp = session
            .add_torrent(AddTorrent::Url(magnet.into()), Some(opts))
            .await
            .map_err(|e| format!("添加任务失败: {e:#}"))?;
        match resp {
            AddTorrentResponse::Added(id, _) => Ok(id),
            AddTorrentResponse::AlreadyManaged(id, _) => Ok(id),
            AddTorrentResponse::ListOnly(_) => Err("仅解析模式不会开始下载".into()),
        }
    }

    /// 列出全部任务及其统计
    pub async fn list(&self) -> Result<Vec<DownloadRow>, String> {
        let Some(session) = self.session.lock().await.clone() else {
            // 会话未创建 = 还没有任何任务
            return Ok(Vec::new());
        };
        let rows = session.with_torrents(|iter| {
            iter.map(|(id, handle)| DownloadRow {
                id,
                name: handle.name().unwrap_or_else(|| format!("任务 #{id}")),
                paused: handle.is_paused(),
                stats: serde_json::to_value(handle.stats()).unwrap_or(serde_json::Value::Null),
            })
            .collect::<Vec<_>>()
        });
        Ok(rows)
    }

    /// 暂停任务
    pub async fn pause(&self, id: usize) -> Result<(), String> {
        let session = self.session().await?;
        let handle = Self::find(&session, id).ok_or_else(|| format!("任务不存在: {id}"))?;
        session
            .pause(&handle)
            .await
            .map_err(|e| format!("暂停失败: {e:#}"))
    }

    /// 继续任务
    pub async fn resume(&self, id: usize) -> Result<(), String> {
        let session = self.session().await?;
        let handle = Self::find(&session, id).ok_or_else(|| format!("任务不存在: {id}"))?;
        session
            .unpause(&handle)
            .await
            .map_err(|e| format!("继续失败: {e:#}"))
    }

    /// 删除任务（可选同时删除已下载文件）
    pub async fn remove(&self, id: usize, delete_files: bool) -> Result<(), String> {
        let session = self.session().await?;
        session
            .delete(TorrentIdOrHash::Id(id), delete_files)
            .await
            .map_err(|e| format!("删除失败: {e:#}"))
    }

    /// 在会话内按 id 查找任务句柄
    ///
    /// `with_torrents` 回传的是 `&mut dyn Iterator`（未满足 `Sized`），
    /// 因此不能直接用 `Iterator::find`，这里改为显式循环。
    fn find(session: &Arc<Session>, id: usize) -> Option<Arc<ManagedTorrent>> {
        session.with_torrents(|iter| {
            for (tid, handle) in iter {
                if tid == id {
                    return Some(handle.clone());
                }
            }
            None
        })
    }
}
