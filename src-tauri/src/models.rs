//! 数据模型：站点配置（订阅源 schema）、搜索结果、更新信息、历史记录

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 字段提取规则：纯字符串表示取元素文本；对象表示取元素的指定属性
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum FieldSpec {
    /// 选择器字符串，取匹配元素的文本
    Text(String),
    /// 取匹配元素指定属性的值
    Attr { sel: String, attr: String },
}

/// 请求配置
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RequestConfig {
    /// 请求方法：GET / POST
    #[serde(default = "default_method")]
    pub method: String,
    /// 搜索地址模板，支持 [keyword] [page] 占位符
    pub search_url: String,
    /// 请求头（兼容旧字段名 header）
    #[serde(default, alias = "header")]
    pub headers: HashMap<String, String>,
    /// 单站超时（毫秒），默认 12000
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_method() -> String {
    "GET".into()
}
fn default_timeout() -> u64 {
    12000
}

/// XPath/CSS 提取规则（订阅源 schema）
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ExpressionModel {
    /// 结果条目分组选择器
    pub group: String,
    pub title: Option<FieldSpec>,
    pub date: Option<FieldSpec>,
    pub size: Option<FieldSpec>,
    pub url: Option<FieldSpec>,
    pub magnet: Option<FieldSpec>,
}

/// 站点配置（与订阅源 JSON 一一对应）
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SiteConfig {
    /// 站点 ID（订阅源自带；自定义站点为空由程序生成）
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub info: String,
    /// 源内开关（订阅源默认状态，用户开关以 enabled 覆盖为准）
    #[serde(default = "default_true")]
    pub state: bool,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub update_time: String,
    #[serde(default)]
    pub request: RequestConfig,
    #[serde(default)]
    pub expression_model: ExpressionModel,
    /// 是否为本地自定义站点
    #[serde(default, skip)]
    pub is_custom: bool,
    /// 用户开关覆盖（运行时计算，随 API 返回前端）
    #[serde(default)]
    pub enabled: bool,
    /// 是否为默认搜索源（运行时计算，随 API 返回前端）
    #[serde(default)]
    pub is_default: bool,
}

fn default_true() -> bool {
    true
}

/// 单条磁力搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagnetItem {
    pub title: String,
    pub date: String,
    pub size: String,
    pub url: String,
    pub magnet: String,
}

/// 单个站点搜索的完整结果（含 B2 搜索统计所需字段）
#[derive(Debug, Clone, Serialize)]
pub struct SiteOutcome {
    pub site_id: String,
    pub site_name: String,
    pub success: bool,
    pub elapsed_ms: u64,
    pub items: Vec<MagnetItem>,
    pub error: Option<String>,
    /// 是否为默认搜索源
    #[serde(default)]
    pub is_default: bool,
}

/// 站点连接测试结果（自定义站点弹窗「测试连接」）
#[derive(serde::Serialize)]
pub struct SiteTestResult {
    pub ok: bool,
    pub elapsed_ms: u64,
    pub items: usize,
    pub error: Option<String>,
    pub samples: Vec<String>,
}

/// 站点持久化数据（sites.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SitesData {
    /// 订阅仓库 URL
    #[serde(default)]
    pub subscribe_url: String,
    /// 订阅源拉取的站点
    #[serde(default)]
    pub subscribed: Vec<SiteConfig>,
    /// 本地自定义站点
    #[serde(default)]
    pub custom: Vec<SiteConfig>,
    /// 用户开关覆盖：key 为站点 id
    #[serde(default)]
    pub enabled: HashMap<String, bool>,
    /// 默认搜索源 id
    #[serde(default)]
    pub default_id: Option<String>,
    /// 最近一次订阅拉取时间
    #[serde(default)]
    pub subscribed_at: Option<String>,
}

/// 历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub keyword: String,
    pub magnet: String,
    pub time: u64,
}
