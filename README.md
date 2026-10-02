# DHThub

跨平台（Linux / Windows / Android）**多源磁力链接聚合搜索**工具。
一次搜索多站并发聚合

- 极致的体验：数据本地存储、无需账号、无广告 SDK
- 极少的权限：权限仅网络、剪贴板、打开外部链接、安装 APK（Android）
- 更新及时：内置自动检测更新（GitHub Release + 镜像），snackbar 提示新版本

# 技术栈

- Tauri 2.12（Rust）
- Vue 3.5
- Vuetify 4.2（Material Design 3）
- Pinia
- Vue Router
- Vite 8
- @zebra-ui/swiper（首页滑动窗口）
- @vueuse/core

---

# 目录结构

```
DHThub/
├── sites/
│   ├── default.json          # 默认订阅源（CSS 选择器 schema）
│   └── hotwords.json         # 热门推荐热词总表（首页热词，经镜像抓取 + 本地缓存）
├── src/                      # Vue3 前端
│   ├── views/                # Home / SearchResults / Sites / History / Settings / About
│   ├── components/           # AppLayout（滑动窗口框架）/ 磁力详情 / 搜索统计
│   ├── stores/               # 站点状态(Pinia) + 设置持久化
│   └── lib/                  # tauri / http（plugin-http 请求）/ update（更新检测）/ mirrors
├── src-tauri/
│   ├── src/                  # Rust：search / sites / history
│   ├── capabilities/         # 桌面 / 移动端权限（http 允许任意 http/https）
│   └── tauri.conf.json
└── .github/workflows/release.yml
```

# 功能

- 首页为 @zebra-ui/swiper 滑动窗口，内嵌「搜索 / 搜索源 / 历史」3 页，左右滑动切换；手机端底部 tab 栏、桌面端顶部 tab，自动跟随
- 搜索结果在独立页面展示：并发聚合、过滤、排序、分页、搜索统计、磁力详情
- 搜索源：右下角悬浮按钮管理订阅源（拉取/导入/导出/重置）与自定义站点，搜索源均可删除（长按/右键菜单）
- GitHub 镜像：内置直连 + gh-proxy.com，可自定义添加；用于更新检测、订阅拉取、热词拉取
- 自动检测更新：启动时静默检查 GitHub Release，新版本通过 snackbar 提示

# 本地开发

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

## 发布流程（GitHub Actions）

1. 更新 `CHANGELOG.md` 顶部为最新版本（格式 `## vX.Y.Z`），推送到 main。
2. 在 Actions 页手动触发 `Build and Release`（`workflow_dispatch`）。
3. 工作流读取 CHANGELOG 生成版本号与发布说明，自动构建并发布：
   deb / rpm / Windows 安装包 / Android APK（arm64 / armv7），产物统一命名 `DHThub-{版本}-{平台}-{架构}`。

### 可选配置

| Secret | 用途 |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 桌面更新签名（生成：`npx tauri signer generate -w ~/.dhthub/tauri.key`） |
| `ANDROID_KEY_BASE64` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD` | Android release 签名（生成：`keytool -genkeypair -v -keystore release.keystore -alias dhthub -keyalg RSA -keysize 2044 -validity 10000`，再 `base64 release.keystore`） |

不配置上述密钥时：Android 使用仓库内置的稳定 debug 签名（`android/debug.keystore`，alias `androiddebugkey`）打包——每次构建同一密钥并随版本递增 versionCode，保证旧版本覆盖升级安装不报“软件包冲突”；正式对外发布建议配置正式密钥。

# 免责声明

本工具仅用于技术学习与合法信息检索。搜索结果来自第三方站点，请遵守所在地法律法规，仅检索与传播您拥有合法权利的内容。
