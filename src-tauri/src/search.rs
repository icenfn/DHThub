//! 多源磁力搜索：并发请求订阅源站点，按 CSS 选择器规则解析结果

use crate::models::{ExpressionModel, FieldSpec, MagnetItem, SiteConfig, SiteOutcome};
use scraper::{ElementRef, Html, Selector};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

/// 最大并发请求数（站点多时并发拉满，减少整体耗时）
const MAX_CONCURRENCY: usize = 8;

/// 并发执行多站点搜索：每站一个 tokio 任务 + 信号量限流，
/// 任一站点失败/超时都不影响其他站点，结果按完成顺序返回
pub async fn search_multi(
    client: &reqwest::Client,
    sites: Vec<SiteConfig>,
    keyword: &str,
    page: u32,
) -> Vec<SiteOutcome> {
    if sites.is_empty() {
        return vec![];
    }
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();
    for site in sites {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let keyword = keyword.to_string();
        tasks.spawn(async move {
            // 信号量关闭（不可能发生）时也继续执行，避免整批搜索卡死
            let _permit = semaphore.acquire().await.ok();
            search_one(&client, &site, &keyword, page).await
        });
    }

    let mut out = Vec::with_capacity(tasks.len());
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(o) => out.push(o),
            // 站点任务异常（如解析 panic）：单独失败，不影响整体
            Err(e) => out.push(SiteOutcome {
                site_id: String::new(),
                site_name: "未知站点".into(),
                success: false,
                elapsed_ms: 0,
                items: vec![],
                error: Some(format!("任务异常: {e}")),
                is_default: false,
            }),
        }
    }
    out
}

/// 单站点搜索（一次性请求，不重试；尽力而为）
async fn search_one(
    client: &reqwest::Client,
    site: &SiteConfig,
    keyword: &str,
    page: u32,
) -> SiteOutcome {
    let started = Instant::now();
    let url = build_url(site, keyword, page);
    let site_id = site.id.clone().unwrap_or_default();
    let site_name = site.name.clone();
    let is_default = site.is_default;

    match fetch_html(client, site, &url).await {
        Ok(html) => {
            let mut items = parse_html(&html, &site.expression_model);
            // 站点常给相对路径（如磁力狗的 /information/xxx），前端拿到后既打不开也无法
            // 回源解析：这里统一补全成绝对地址
            let origin = site_origin(site);
            if !origin.is_empty() {
                for it in items.iter_mut() {
                    it.url = normalize_detail_url(&it.url, &origin);
                }
            }
            // 列表页本身没有磁力的站点（如磁力狗、ØMagnet 只在详情页给）：
            // 搜索阶段 **不再** 回源抓详情页，避免每个源多打十几次请求。
            // 真实 magnet 由前端在进入资源详情页时调用 resolve_magnet 按需解析。
            // 兜底：不合法的「磁力」（含短链）清空，避免详情页把它当磁力展示/复制
            for it in items.iter_mut() {
                if !it.magnet.is_empty() && !looks_like_magnet(&it.magnet) {
                    it.magnet.clear();
                }
            }
            SiteOutcome {
                site_id,
                site_name,
                success: true,
                elapsed_ms: started.elapsed().as_millis() as u64,
                items,
                error: None,
                is_default,
            }
        }
        Err(e) => SiteOutcome {
            site_id,
            site_name,
            success: false,
            elapsed_ms: started.elapsed().as_millis() as u64,
            items: vec![],
            error: Some(e),
            is_default,
        },
    }
}

/// 判断是否是可用的磁力链接：magnet:? 且带 xt=urn:btih
fn looks_like_magnet(s: &str) -> bool {
    let t = s.trim();
    t.len() > 20 && t.to_ascii_lowercase().starts_with("magnet:?") && t.contains("xt=urn:btih")
}

/// 按需解析单条资源的真实磁力：只抓一个详情页。
///
/// 搜索阶段只拿到列表页数据（很多站点列表页并不给真实 magnet），
/// 进入资源详情页时再调用本函数回源，避免搜索时对每个站点发起十几次额外请求。
pub(crate) async fn resolve_one_magnet(
    client: &reqwest::Client,
    site: &SiteConfig,
    detail_url: &str,
) -> Option<String> {
    let spec = site.expression_model.magnet_detail.as_ref()?;
    let origin = site_origin(site);
    let target = normalize_detail_url(detail_url, &origin);
    if target.trim().is_empty() {
        return None;
    }
    let html = fetch_html(client, site, &target).await.ok()?;
    let magnet = extract_field_doc(&html, &spec);
    if looks_like_magnet(&magnet) {
        Some(magnet.trim().to_string())
    } else {
        None
    }
}

/// 从站点搜索地址推导出 origin（如 https://0mag.me），用于补全详情页相对路径
fn site_origin(site: &SiteConfig) -> String {
    let raw = site.request.search_url.trim();
    // 取 scheme://host 部分
    let after_scheme = raw
        .split_once("://")
        .map(|(scheme, rest)| (scheme, rest))
        .or_else(|| raw.strip_prefix("//").map(|rest| ("https", rest)));
    match after_scheme {
        Some((scheme, rest)) => {
            let host = rest.split(['/', '?', '#']).next().unwrap_or("");
            if host.is_empty() {
                String::new()
            } else {
                format!("{scheme}://{host}")
            }
        }
        None => String::new(),
    }
}

/// 详情页地址归一化：补全协议、处理站点相对路径（如 0mag 的 /!xxxxx）
fn normalize_detail_url(raw: &str, origin: &str) -> String {
    let t = raw.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else if let Some(rest) = t.strip_prefix("//") {
        format!("https://{rest}")
    } else if t.starts_with('/') {
        if origin.is_empty() {
            t.to_string()
        } else {
            format!("{origin}{t}")
        }
    } else if t.is_empty() {
        String::new()
    } else if origin.is_empty() {
        t.to_string()
    } else {
        format!("{origin}/{t}")
    }
}

/// 从整篇文档按规则取值（详情页解析用，不局限于某个条目容器）
fn extract_field_doc(html: &str, spec: &FieldSpec) -> String {
    let doc = Html::parse_document(html);
    match spec {
        FieldSpec::Text(sel) => Selector::parse(sel)
            .ok()
            .and_then(|s| doc.select(&s).next())
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default(),
        FieldSpec::Attr { sel, attr } => Selector::parse(sel)
            .ok()
            .and_then(|s| doc.select(&s).next())
            .map(|e| e.value().attr(attr).unwrap_or_default().to_string())
            .unwrap_or_default(),
    }
}

/// 构造搜索地址：替换 [keyword] [page] 占位符；
/// 站点配置 keyword_encode = "base64" 时关键词先做 base64 编码（如 ØMagnet）
pub(crate) fn build_url(site: &SiteConfig, keyword: &str, page: u32) -> String {
    let kw = if site.request.keyword_encode.as_deref() == Some("base64") {
        // base64 结果可能含 +/= 等 URL 敏感字符，需再 URL 编码
        urlencoding::encode(&base64::encode(keyword.as_bytes())).to_string()
    } else {
        urlencoding::encode(keyword).to_string()
    };
    site.request
        .search_url
        .replace("[keyword]", &kw)
        .replace("[page]", &page.to_string())
}

/// 发起请求并取回 HTML 文本
pub(crate) async fn fetch_html(
    client: &reqwest::Client,
    site: &SiteConfig,
    url: &str,
) -> Result<String, String> {
    let mut req = client.get(url).timeout(std::time::Duration::from_millis(
        site.request.timeout_ms.clamp(3000, 10000),
    ));

    let mut headers = site.request.headers.clone();
    headers
        .entry("User-Agent".into())
        .or_insert_with(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36".into());
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }

    let method = site.request.method.trim().to_uppercase();
    let resp = if method == "POST" {
        let form = build_form(site, url);
        req.form(&form).send().await
    } else {
        req.send().await
    };

    let resp = resp.map_err(|e| format!("请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {e}"))?;
    if text.trim().is_empty() {
        return Err("空响应".into());
    }
    Ok(text)
}

/// POST 场景：解析模板中的 keyword/page 参数作为表单字段
fn build_form(site: &SiteConfig, url: &str) -> Vec<(String, String)> {
    let mut form: Vec<(String, String)> = vec![];
    let lower = site.request.search_url.to_lowercase();
    if lower.contains("[keyword]") {
        // 从 URL 中还原占位符位置再填充表单字段（字段名可用 keyword_field 覆盖）
        if let Some(kw) = extract_placeholder(&site.request.search_url, url, "[keyword]") {
            let field = site.request.keyword_field.as_deref().unwrap_or("keyword");
            form.push((field.to_string(), kw));
        }
    }
    if lower.contains("[page]") {
        if let Some(pg) = extract_placeholder(&site.request.search_url, url, "[page]") {
            form.push(("page".to_string(), pg));
        }
    }
    if form.is_empty() {
        form.push(("keyword".to_string(), String::new()));
    }
    form
}

/// 从已替换的 URL 反向提取占位符实际值（用于 POST 表单）
fn extract_placeholder(template: &str, final_url: &str, placeholder: &str) -> Option<String> {
    let idx = template.find(placeholder)?;
    let before = &template[..idx];
    let after = &template[idx + placeholder.len()..];
    let rest = final_url.strip_prefix(before)?;
    // after 可能以分隔符开头（如 &、?、/），关键词值 = rest 中到该分隔符为止；
    // 无分隔符时取余下整段
    let sep: String = after
        .chars()
        .take_while(|c| {
            !c.is_ascii_alphanumeric() && *c != '.' && *c != '-' && *c != '_' && *c != '~'
        })
        .collect();
    let value = if sep.is_empty() {
        rest
    } else {
        let e = rest.find(&sep).unwrap_or(rest.len());
        &rest[..e]
    };
    Some(urlencoding::decode(value).ok()?.to_string())
}

/// 按选择器规则解析 HTML
pub(crate) fn parse_html(html: &str, expr: &ExpressionModel) -> Vec<MagnetItem> {
    if expr.group.trim().is_empty() {
        return vec![];
    }
    let doc = Html::parse_document(html);
    let group_sel = match Selector::parse(&expr.group) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut items = Vec::new();
    for node in doc.select(&group_sel) {
        let title = extract_field(&doc, &node, expr.title.as_ref());
        if title.trim().is_empty() {
            continue;
        }
        let url = extract_field(&doc, &node, expr.url.as_ref());
        let magnet = extract_field(&doc, &node, expr.magnet.as_ref());
        if url.is_empty() && magnet.is_empty() {
            continue;
        }
        items.push(MagnetItem {
            title,
            date: extract_field(&doc, &node, expr.date.as_ref()),
            size: extract_field(&doc, &node, expr.size.as_ref()),
            url,
            magnet,
        });
    }
    items
}

/// 提取字段：文本或属性
fn extract_field(doc: &Html, root: &ElementRef<'_>, spec: Option<&FieldSpec>) -> String {
    let Some(spec) = spec else {
        return String::new();
    };
    match spec {
        FieldSpec::Text(sel) => Selector::parse(sel)
            .ok()
            .and_then(|s| root.select(&s).next())
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default(),
        FieldSpec::Attr { sel, attr } => {
            if sel.starts_with('/') {
                // 兼容原版 XPath 绝对路径：从整个文档中查找
                let _ = doc;
                String::new()
            } else {
                Selector::parse(sel)
                    .ok()
                    .and_then(|s| root.select(&s).next())
                    .map(|e| e.value().attr(attr).unwrap_or_default().to_string())
                    .unwrap_or_default()
            }
        }
    }
}

/// 流式多站点搜索：每完成一个站点就向前端推送一次结果，
/// 前端可立刻解析并渲染，不必等全部站点搜索完成后再统一显示。
pub async fn stream_multi(
    client: reqwest::Client,
    sites: Vec<SiteConfig>,
    keyword: String,
    page: u32,
    stream_id: String,
    app: tauri::AppHandle,
) {
    use tauri::Emitter;
    if sites.is_empty() {
        let _ = app.emit(
            "search-done",
            serde_json::json!({ "streamId": stream_id, "total": 0 }),
        );
        return;
    }
    let total = sites.len();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();
    for site in sites {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let keyword = keyword.clone();
        tasks.spawn(async move {
            let _permit = semaphore.acquire().await.ok();
            search_one(&client, &site, &keyword, page).await
        });
    }
    while let Some(res) = tasks.join_next().await {
        let outcome = match res {
            Ok(o) => o,
            Err(e) => SiteOutcome {
                site_id: String::new(),
                site_name: "未知站点".into(),
                success: false,
                elapsed_ms: 0,
                items: vec![],
                error: Some(format!("任务异常: {e}")),
                is_default: false,
            },
        };
        // stream_id 每轮都要用，必须借用而不是移动
        let _ = app.emit(
            "search-site",
            serde_json::json!({ "streamId": &stream_id, "outcome": outcome }),
        );
    }
    let _ = app.emit(
        "search-done",
        serde_json::json!({ "streamId": stream_id, "total": total }),
    );
}
