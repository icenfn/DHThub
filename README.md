# DHThub

跨平台（Linux / Windows / Android）**多源磁力链接聚合搜索**工具。
一次搜索多站并发聚合

- 极致的体验：数据本地存储、无需账号、无广告 SDK
- 极少的权限：权限仅网络、剪贴板、打开外部链接、安装 APK（Android）
- 更新及时：无需访问github或其他应用市场即可获取最新版本

# 技术栈

- Vue 3.5
- Vuetify 4.2
- Pinia
- Vue Router
- Vite 8

---

# 目录结构

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

1. 推送 tag 触发发布：
   ```bash
   git tag v0.2.5 && git push origin v0.2.5
   ```
2. 或在 Actions 页手动触发 `workflow_dispatch`。
3. 产物自动上传 GitHub Release：deb / rpm / nsis / APK（arm64 / armv7）。

### 可选配置

| Secret | 用途 |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 桌面更新签名（生成：`npx tauri signer generate -w ~/.dhthub/tauri.key`） |
| `ANDROID_KEY_BASE64` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD` | Android release 签名（生成：`keytool -genkeypair -v -keystore release.keystore -alias dhthub -keyalg RSA -keysize 2048 -validity 10000`，再 `base64 release.keystore`） |

不配置上述密钥时：桌面端跳过自动更新通道（手动下载安装包）、Android 自动使用生成的 debug 签名打包（可正常安装；正式对外发布建议配置正式密钥）。

# 免责声明

本工具仅用于技术学习与合法信息检索。搜索结果来自第三方站点，请遵守所在地法律法规，仅检索与传播您拥有合法权利的内容。
