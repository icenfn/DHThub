# DHThub

跨平台（Linux / Windows / Android）**多源磁力链接聚合搜索**工具。
逆向重构自「闪电磁力」类应用：通过**订阅源**统一管理搜索站点，一次搜索多站并发聚合，全平台由 GitHub Actions 自动构建发布。

- 前端：Vue 3.5 + Vuetify 4.2（Material Design 3）+ Pinia + Vue Router + Vite 8
- 桌面/移动壳：Tauri 2.12（Rust 核心：reqwest 并发抓取 + scraper 解析 + 本地 JSON 持久化）
- 更新检测：GitHub Releases API + 桌面端 tauri-plugin-updater + Android APK 下载安装
- CI/CD：`.github/workflows/release.yml` 一键产出 Linux(deb/appimage/rpm)、Windows(nsis/msi)、Android(APK×4 ABI)

> 说明：站点订阅源全部存放在本仓库 `sites/` 目录（或任意 GitHub Raw JSON 地址），应用内可随时「拉取订阅」更新；不内置任何私有站点列表。

---

## 功能清单（已实现）

### A 核心
| 编号 | 功能 | 说明 |
|---|---|---|
| A1 | 多源磁力搜索 | 订阅源驱动、6 路并发、GET/POST、CSS 选择器解析、分页 |
| A2 | 站点管理 | 订阅源拉取/开关/默认引擎/自定义站点/重置 |
| A3 | 结果展示 | 按站点分组卡片、失败站点提示、空态/加载态 |
| A4 | 磁力操作 | 复制、打开（系统处理）、分享、详情弹窗 |
| A5 | 搜索历史+热词 | 本地 20 条历史、热词「换一换」、清空 |
| A6 | 三类历史页 | 磁力/复制/浏览记录，去重、上限 500、清空 |
| A7 | GitHub 更新检测 | Release 检查、桌面自动更新、Android APK 下载安装、启动静默检查 |
| A8 | 设置页 | 主题、热词管理、数据清除、订阅源、关于 |
| A9 | Actions 三端发布 | tag v* 触发，Linux/Windows/Android 自动打包 + Release |

### B 已选
| 编号 | 功能 |
|---|---|
| B2 | 搜索统计（每站结果数/耗时/成功状态） |
| B3 | 首启协议弹窗 + 协议/免责声明页 |
| B6 | 结果过滤 + 按大小/日期排序 |
| B7 | 深色模式（跟随系统/手动） |
| B8 | 站点配置导入/导出（JSON） |

---

## 目录结构

```
DHThub/
├── sites/
│   └── default.json          # 默认订阅源（CSS 选择器 schema）
├── src/                      # Vue3 前端
│   ├── views/                # Home / Sites / History / Settings / About / 协议 / 免责
│   ├── components/           # 磁力详情 / 搜索统计 / 更新弹窗
│   ├── stores/               # 站点状态(Pinia) + 设置持久化
│   └── lib/tauri.ts          # Tauri 环境检测与 IPC 封装（浏览器预览降级）
├── src-tauri/
│   ├── src/                  # Rust：search / sites / history / update
│   ├── capabilities/         # 桌面 / 移动端权限
│   └── tauri.conf.json
└── .github/workflows/release.yml
```

## 本地开发

环境要求：Node 22+、Rust stable、系统 WebKit 依赖（Linux 见下方）。

```bash
npm install
# Linux 首次需安装：
#   sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# 浏览器预览（仅 UI，搜索需在 Tauri 中运行）
npm run dev

# 桌面端运行
npm run tauri dev

# 桌面端打包（Linux）
npm run tauri build
```

## Android 构建

```bash
# 环境：JDK 17 + Android SDK + NDK r26d + Rust android 目标
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
npx tauri android init
cargo tauri android build --apk --split-per-abi
```

> ⚠️ Android 端「自动安装 APK」依赖 `tauri-plugin-shell` 拉起系统安装器，
> CI 中已注入 `REQUEST_INSTALL_PACKAGES` 权限；首次使用请在真机上验证安装流程。

## 发布流程（GitHub Actions）

1. 推送 tag 触发发布：
   ```bash
   git tag v0.1.0 && git push origin v0.1.0
   ```
2. 或在 Actions 页手动触发 `workflow_dispatch`。
3. 产物自动上传 GitHub Release：deb / AppImage / rpm / nsis / msi / APK（arm64/v7a/x86/x86_64）。

### 可选配置（建议设置）

| Secret | 用途 |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 桌面更新签名（生成：`npx tauri signer generate -w ~/.dhthub/tauri.key`） |
| `ANDROID_KEY_BASE64` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD` | Android release 签名（生成：`keytool -genkeypair -v -keystore release.keystore -alias dhthub -keyalg RSA -keysize 2048 -validity 10000`，再 `base64 release.keystore`） |

不配置上述密钥时：桌面端跳过自动更新通道（手动下载安装包）、Android 使用 debug 签名。

## 订阅源 Schema

`sites/default.json` 为站点数组，每个站点：

```jsonc
{
  "id": "10004",               // 站点唯一 ID
  "name": "磁力猫",
  "info": "站点说明",
  "state": true,               // 默认启用
  "request": {
    "method": "GET",           // GET / POST
    "search_url": "https://host/[keyword]?page=[page]",  // [keyword] [page] 占位符
    "headers": { "Referer": "https://host/" },
    "timeout_ms": 12000
  },
  "expression_model": {
    "group": "#Search_list_wrapper li",          // 结果条目容器
    "title": "a.SearchListTitle_result_title",   // 字符串=取文本
    "date": "div.Search_list_info",
    "size": "td.td-size",
    "url":  { "sel": "a.x", "attr": "href" },    // 对象=取属性
    "magnet": { "sel": "a#down-url", "attr": "href" }
  }
}
```

- 字段支持 CSS 选择器文本或 `{ sel, attr }` 属性提取（兼容原版 XPath 的常见结构）
- 支持数组外层包一层 `{ "sites": [...] }` 的对象格式
- 自定义站点可在应用内直接添加，无需改仓库

## 数据与隐私

- 全部数据本地存储（`app_data/sites.json`、`history.json`、`settings.json`），无账号、无遥测、无广告 SDK
- 权限收敛：仅网络、剪贴板、打开外部链接、安装 APK（Android）等必要项
- 更新检测仅访问 `api.github.com/repos/icenfn/DHThub` 官方接口

## 免责声明

本工具仅用于技术学习与合法信息检索。搜索结果来自第三方站点，请遵守所在地法律法规，仅检索与传播您拥有合法权利的内容。详见应用内「免责声明」页。
