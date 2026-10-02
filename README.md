# DHThub

跨平台（Linux / Windows / Android）**多源磁力链接聚合搜索**工具。
逆向重构自「闪电磁力」类应用：通过**订阅源**统一管理搜索站点，一次搜索多站并发聚合，全平台由 GitHub Actions 自动构建发布。

- 前端：Vue 3.5 + Vuetify 4.2（Material Design 3）+ Pinia + Vue Router + Vite 8
- 桌面/移动壳：Tauri 2.12（Rust 核心：reqwest 并发抓取 + scraper 解析 + 本地 JSON 持久化）
- 更新检测：GitHub Releases API + 桌面端 tauri-plugin-updater + Android APK 下载安装
- CI/CD：`.github/workflows/release.yml` 一键产出 Linux(deb/rpm)、Windows(nsis)、Android(APK×2 ABI: arm64/armv7)

> 说明：站点订阅源全部存放在本仓库 `sites/` 目录（或任意 GitHub Raw JSON 地址），应用内可随时「拉取订阅」更新；不内置任何私有站点列表。

---

## 功能清单（已实现）

> **v0.2.6**：更新检测改用 `releases/latest` 轻量实现（去掉更新弹窗，统一改用提示条 snackbar 提示，含自动检测）；修复 Android 仍编译 x86/x86_64 的问题（直接收紧 gradle 的 cargo.targets 并注入 abiFilters，仅 arm64/armv7）；**删除调试模式**；App.vue 不再写死页面框架（主框架抽为 AppLayout 嵌套路由，设置页使用独立页面框架）；修复订阅在线仓库拉取一直转圈（Rust HTTP 客户端增加 connect 超时 + 前端 IPC 超时兜底）。

> **v0.2.5**：设置页独立路由（Vue Router 跳转）；新增**自动检测更新**开关与**调试模式**；镜像测速重构为**纯前端实现**（no-cors + 超时兜底，不再转圈无结果）；移除 ghfast.top 内置镜像，镜像列表仅显示链接、直连带「直连」标签、添加镜像无需填写名称；历史记录仅保留**浏览记录**；打包目标调整（不再产出 AppImage / MSI，APK 仅 arm64 + armv7）。

> **v0.2.0**：UI 全面升级至 Vuetify 4 + Material Design 3（完整 MD3 色彩 token、顶栏布局）；新增独立设置页的 **GitHub 镜像** 配置（内置 2 个、可自定义、一键测速、手动/自动选最快），用于检查更新与订阅源拉取；新增**设置导出/导入**（JSON 备份恢复）。

### A 核心
| 编号 | 功能 | 说明 |
|---|---|---|
| A1 | 多源磁力搜索 | 订阅源驱动、6 路并发、GET/POST、CSS 选择器解析、分页 |
| A2 | 站点管理 | 订阅源拉取/开关/默认引擎/自定义站点/重置 |
| A3 | 结果展示 | 按站点分组卡片、失败站点提示、空态/加载态 |
| A4 | 磁力操作 | 复制、打开（系统处理）、分享、详情弹窗 |
| A5 | 搜索历史+热词 | 本地 20 条历史、热词「换一换」、清空 |
| A6 | 浏览历史页 | 仅记录浏览（详情查看）记录，去重、上限 500、清空 |
| A7 | GitHub 更新检测 | releases/latest 轻量检查（支持镜像、提示条提示）、启动自动检测（可开关） |
| A8 | 设置页 | 独立页面框架：主题、GitHub 镜像（内置 2 个/自定义/测速/手动或自动选最快）、自动更新开关、设置导出导入（JSON）、热词管理、数据清除、订阅源、关于 |
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
# 环境：JDK 17 + Android SDK + NDK r26d + Rust android 目标（仅 arm64 / armv7，不编译 x86）
rustup target add aarch64-linux-android armv7-linux-androideabi
npx tauri android init
cargo tauri android build --apk --split-per-abi
```

> ⚠️ Android 端「自动安装 APK」依赖 `tauri-plugin-shell` 拉起系统安装器，
> CI 中已注入 `REQUEST_INSTALL_PACKAGES` 权限；首次使用请在真机上验证安装流程。

## 发布流程（GitHub Actions）

1. 推送 tag 触发发布：
   ```bash
   git tag v0.2.5 && git push origin v0.2.5
   ```
2. 或在 Actions 页手动触发 `workflow_dispatch`。
3. 产物自动上传 GitHub Release：deb / rpm / nsis / APK（arm64 / armv7）。

### 可选配置（建议设置）

| Secret | 用途 |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 桌面更新签名（生成：`npx tauri signer generate -w ~/.dhthub/tauri.key`） |
| `ANDROID_KEY_BASE64` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD` | Android release 签名（生成：`keytool -genkeypair -v -keystore release.keystore -alias dhthub -keyalg RSA -keysize 2048 -validity 10000`，再 `base64 release.keystore`） |

不配置上述密钥时：桌面端跳过自动更新通道（手动下载安装包）、Android 自动使用生成的 debug 签名打包（可正常安装；正式对外发布建议配置正式密钥）。

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
- 更新检测仅访问 `github.com/icenfn/DHThub/releases/latest`（官方页面）

## 免责声明

本工具仅用于技术学习与合法信息检索。搜索结果来自第三方站点，请遵守所在地法律法规，仅检索与传播您拥有合法权利的内容。详见应用内「免责声明」页。
