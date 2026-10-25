//! BitTorrent 下载：内嵌 librqbit（rqbit 的库形态）
//!
//! 设计要点：
//! - Session 懒初始化：首次调用下载相关命令时才创建，避免拖慢启动；
//! - 只用库 API，不启用 http-api 特性，减少交叉编译（Android/Windows）风险；
//! - DHT 持久化文件路径必须显式指定到应用数据目录：librqbit 默认走
//!   `directories` crate 解析项目目录（com.rqbit.dht），在 Android 上无法解析，
//!   会导致「error initializing persistent DHT: cannot determine project directory」；
//! - 内核配置（DHT/限速/端口/追踪器等）由前端设置页下发，修改后可重启会话生效；
//! - 统计信息直接把 librqbit 的 `TorrentStats`（实现了 Serialize）转成 JSON 透出，
//!   前端按需取字段，避免在后端重复建模导致上游字段改动即编译失败。

use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use librqbit::api::TorrentIdOrHash;
use librqbit::dht::DhtPersistenceConfig;
use librqbit::limits::LimitsConfig;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, ManagedTorrent, Session,
    SessionOptions,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 下载内核配置（由前端设置页下发；数值为 0 / 空 = 使用内核默认或不限速）
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    /// 启用 DHT（分布式哈希表，无 tracker 时找 peer）
    pub dht_enabled: bool,
    /// DHT 监听端口，0 = 随机
    pub dht_port: u32,
    /// 下载限速 KB/s，0 = 不限
    pub download_limit_kbps: u32,
    /// 上传限速 KB/s，0 = 不限
    pub upload_limit_kbps: u32,
    /// 每个任务的 peer 上限，0 = 内核默认
    pub peer_limit: u32,
    /// 并发初始化任务数，0 = 内核默认
    pub concurrent_init_limit: u32,
    /// 禁用 tracker 通信（纯 DHT 模式）
    pub disable_trackers: bool,
    /// 仅使用 IPv4
    pub ipv4_only: bool,
    /// 追加到每个任务的 tracker 列表
    pub extra_trackers: Vec<String>,
}

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

/// 下载管理器：持有输出目录、内核配置与惰性初始化的会话
pub struct DownloadManager {
    output_dir: PathBuf,
    /// 应用数据目录（存放 dht-state.json 等内核状态文件）
    state_dir: PathBuf,
    config: Mutex<DownloadConfig>,
    session: Mutex<Option<Arc<Session>>>,
}

/// KB/s → 字节/秒（0 视为不限速）
fn kbps_to_bps(kbps: u32) -> Option<NonZeroU32> {
    if kbps == 0 {
        None
    } else {
        NonZeroU32::new(kbps.saturating_mul(1024))
    }
}

impl DownloadManager {
    pub fn new(output_dir: PathBuf, state_dir: PathBuf) -> Self {
        Self {
            output_dir,
            state_dir,
            config: Mutex::new(DownloadConfig::default()),
            session: Mutex::new(None),
        }
    }

    /// 更新内核配置；返回 true 表示会话已初始化，需要重启会话才能生效
    pub async fn configure(&self, cfg: DownloadConfig) -> bool {
        *self.config.lock().await = cfg;
        self.session.lock().await.is_some()
    }

    /// 重启下载会话（丢弃当前任务列表），下次操作时按新配置重建
    pub async fn restart(&self) -> Result<(), String> {
        let old = self.session.lock().await.take();
        if let Some(session) = old {
            session.stop().await;
        }
        Ok(())
    }

    /// 由当前配置构造 SessionOptions
    fn build_options(&self, cfg: &DownloadConfig) -> SessionOptions {
        // DHT：开启时显式指定持久化文件到应用数据目录，
        // 避免 directories crate 在 Android 上解析项目目录失败
        let dht = if cfg.dht_enabled {
            Some(DhtSessionConfig {
                port: u16::try_from(cfg.dht_port).ok().filter(|p| *p > 0),
                persistence: Some(DhtPersistenceConfig {
                    config_filename: Some(self.state_dir.join("dht-state.json")),
                    ..Default::default()
                }),
                ..Default::default()
            })
        } else {
            None
        };

        let trackers: HashSet<url::Url> = cfg
            .extra_trackers
            .iter()
            .filter_map(|t| t.trim().parse::<url::Url>().ok())
            .collect();

        SessionOptions {
            dht,
            // 不做会话持久化：进程退出即清空任务列表
            persistence: None,
            fastresume: false,
            ratelimits: LimitsConfig {
                download_bps: kbps_to_bps(cfg.download_limit_kbps),
                upload_bps: kbps_to_bps(cfg.upload_limit_kbps),
            },
            trackers,
            disable_trackers: cfg.disable_trackers,
            ipv4_only: cfg.ipv4_only,
            peer_limit: usize::try_from(cfg.peer_limit).ok().filter(|n| *n > 0),
            concurrent_init_limit: usize::try_from(cfg.concurrent_init_limit)
                .ok()
                .filter(|n| *n > 0),
            ..Default::default()
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
        tokio::fs::create_dir_all(&self.state_dir)
            .await
            .map_err(|e| format!("创建状态目录失败: {e}"))?;
        let cfg = self.config.lock().await.clone();
        let opts = self.build_options(&cfg);
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
