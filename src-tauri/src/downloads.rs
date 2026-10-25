//! BitTorrent 下载：内嵌 librqbit（rqbit 的库形态）
//!
//! 设计要点：
//! - Session 懒初始化：首次调用下载相关命令时才创建，避免拖慢启动；
//! - 只用库 API，不启用 http-api 特性，减少交叉编译（Android/Windows）风险；
//! - DHT 持久化文件路径必须显式指定到应用数据目录：librqbit 默认走
//!   `directories` crate 解析项目目录（com.rqbit.dht），在 Android 上无法解析，
//!   会导致「error initializing persistent DHT: cannot determine project directory」；
//! - 内核配置（DHT / UPnP / uTP / 限速 / 追踪器）由前端设置页下发，
//!   修改后自动重启会话即可生效；
//! - 添加磁力分两步：先 `list_only` 探测种子内文件清单，由用户勾选后再真正开始下载；
//! - 统计信息直接把 librqbit 的 `TorrentStats`（实现了 Serialize）转成 JSON 透出。

use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use librqbit::api::TorrentIdOrHash;
use librqbit::dht::DhtPersistenceConfig;
use librqbit::limits::LimitsConfig;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, ListenerMode,
    ListenerOptions, ManagedTorrent, Session, SessionOptions,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// 下载内核配置（由前端设置页下发；限速为 0 表示不限速）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DownloadConfig {
    /// 启用 DHT（分布式哈希表，无 tracker 时找 peer）
    pub dht_enabled: bool,
    /// 启用 UPnP 端口映射
    pub upnp_enabled: bool,
    /// 启用 uTP（基于 UDP 的传输）
    pub utp_enabled: bool,
    /// 下载限速 KB/s，0 = 不限
    pub download_limit_kbps: u32,
    /// 上传限速 KB/s，0 = 不限
    pub upload_limit_kbps: u32,
    /// 追加到每个任务的 tracker 列表
    pub extra_trackers: Vec<String>,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            dht_enabled: true,
            upnp_enabled: true,
            utp_enabled: true,
            download_limit_kbps: 0,
            upload_limit_kbps: 0,
            extra_trackers: Vec::new(),
        }
    }
}

/// 种子内的单个文件（供「下载前选择文件」弹窗使用）
#[derive(Serialize)]
pub struct TorrentFile {
    /// 文件序号，添加任务时用于 only_files
    pub id: usize,
    /// 种子内的相对路径
    pub name: String,
    /// 字节数
    pub length: u64,
}

/// 磁力探测结果
#[derive(Serialize)]
pub struct ProbeResult {
    pub info_hash: String,
    /// 种子名称
    pub name: String,
    /// 总字节数
    pub total_length: u64,
    pub files: Vec<TorrentFile>,
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

/// KB/s → 字节/秒（0 = 不限速）
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

    /// 更新内核配置；返回 true 表示会话已在运行，调用方应立即重启会话
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
                persistence: Some(DhtPersistenceConfig {
                    config_filename: Some(self.state_dir.join("dht-state.json")),
                    ..Default::default()
                }),
                ..Default::default()
            })
        } else {
            None
        };

        // 监听选项：uTP 开关直接决定监听模式；UPnP 由内核向路由器申请端口映射
        let listen = ListenerOptions {
            mode: if cfg.utp_enabled {
                ListenerMode::TcpAndUtp
            } else {
                ListenerMode::TcpOnly
            },
            enable_upnp_port_forwarding: cfg.upnp_enabled,
            // utp_opts 留空时内核内部 unwrap_or_default()，无需引入 librqbit-utp 类型
            ..Default::default()
        };

        let trackers: HashSet<url::Url> = cfg
            .extra_trackers
            .iter()
            .filter_map(|t| t.trim().parse::<url::Url>().ok())
            .collect();

        SessionOptions {
            dht,
            listen: Some(listen),
            // 不做会话持久化：进程退出即清空任务列表
            persistence: None,
            fastresume: false,
            ratelimits: LimitsConfig {
                download_bps: kbps_to_bps(cfg.download_limit_kbps),
                upload_bps: kbps_to_bps(cfg.upload_limit_kbps),
            },
            trackers,
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

    /// 仅解析磁力（不开始下载），返回种子内文件清单供用户勾选
    pub async fn probe(&self, magnet: String) -> Result<ProbeResult, String> {
        let magnet = magnet.trim().to_string();
        if magnet.is_empty() {
            return Err("磁力链接不能为空".into());
        }
        let session = self.session().await?;
        let opts = AddTorrentOptions {
            list_only: true,
            ..Default::default()
        };
        let resp = session
            .add_torrent(AddTorrent::Url(magnet.into()), Some(opts))
            .await
            .map_err(|e| format!("解析磁力失败: {e:#}"))?;
        let list = match resp {
            AddTorrentResponse::ListOnly(list) => list,
            // 已在任务列表中的种子：直接复用其元信息列出文件
            AddTorrentResponse::AlreadyManaged(_, handle) => {
                let name = handle.name().unwrap_or_else(|| "未命名种子".into());
                let mut files = Vec::new();
                let _ = handle.with_metadata(|m| {
                    for (id, f) in m.file_infos.iter().enumerate() {
                        files.push(TorrentFile {
                            id,
                            name: f.relative_filename.to_string_lossy().to_string(),
                            length: f.len,
                        });
                    }
                });
                let total_length = files.iter().map(|f| f.length).sum();
                return Ok(ProbeResult {
                    info_hash: String::new(),
                    name,
                    total_length,
                    files,
                });
            }
            AddTorrentResponse::Added(_, _) => return Err("解析模式不应创建任务".into()),
        };

        let files = list
            .info
            .iter_file_details()
            .enumerate()
            .map(|(id, d)| TorrentFile {
                id,
                name: d.filename.to_pathbuf().to_string_lossy().to_string(),
                length: d.len,
            })
            .collect::<Vec<_>>();
        let total_length = files.iter().map(|f| f.length).sum();
        Ok(ProbeResult {
            info_hash: list.info_hash.as_string(),
            name: list.info.name().map(|n| n.to_string()).unwrap_or_default(),
            total_length,
            files,
        })
    }

    /// 正式添加下载任务；`only_files` 为空表示下载全部文件
    pub async fn add(&self, magnet: String, only_files: Vec<usize>) -> Result<usize, String> {
        let magnet = magnet.trim().to_string();
        if magnet.is_empty() {
            return Err("磁力链接不能为空".into());
        }
        let session = self.session().await?;
        let opts = AddTorrentOptions {
            // 允许续传/播种时覆盖已存在文件
            overwrite: true,
            only_files: if only_files.is_empty() {
                None
            } else {
                Some(only_files)
            },
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
