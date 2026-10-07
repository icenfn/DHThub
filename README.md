<p align="center">
  <img src="app-icon.png" width="120" height="120" alt="DHThub" />
</p>

<h1 align="center">DHThub</h1>

<p align="center">
  跨平台（Linux / Windows / Android）<strong>多源磁力链接聚合搜索</strong>工具
  <br />
  一次搜索，多站并发聚合，本地存储，无广告、无账号
</p>

<p align="center">
  <a href="https://github.com/icenfn/DHThub/releases"><img src="https://img.shields.io/github/v/release/icenfn/DHThub?color=blue" alt="Release" /></a>
  <a href="https://github.com/icenfn/DHThub/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/icenfn/DHThub/release.yml?label=build" alt="Build" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/icenfn/DHThub" alt="License" /></a>
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20Android-lightgrey" alt="Platform" />
</p>

---

## 功能特性

- **多源聚合搜索**：内置 9 个订阅源，一次搜索并发请求所选站点，结果多源合并展示（全部 / 单选 / 多选），支持合并分页与排序
- **订阅源管理**：仓库订阅源一键拉取（支持导入 / 导出 / 重置）、自定义站点、默认源、站点级开关，长按（右键）可修改 / 设为默认 / 删除
- **自定义 DNS**：内置 AliDNS / DNSPod / Cloudflare / 114DNS 可选可测速，支持自定义添加，解决部分搜索源域名解析失败
- **GitHub 镜像**：直连失败时可切换镜像源拉取订阅与检测更新，内置直连 + 4 个镜像，支持自定义与测速
- **自动检测更新**：启动静默检查 GitHub Release，新版本弹窗 / snackbar 提醒，支持手动检查
- **数据本地存储**：搜索历史（仅浏览记录）、站点与设置全部本地保存，支持设置备份 / 恢复与一键清理
- **沉浸式交互**：首页滑动窗口（搜索 / 搜索源 / 历史三页），搜索结果独立页（搜索框内嵌应用栏）支持多源合并、合并分页与一键复制磁力
- **多端自动构建**：GitHub Actions 自动打包 Linux（deb / rpm，arm64）、Windows（NSIS）、Android（arm64）

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面 / 移动框架 | [Tauri 2.12](https://tauri.app)（Rust） |
| 前端 | Vue 3.5 · [Vuetify 4.2](https://vuetifyjs.com)（Material Design 3） |
| 状态 / 路由 | Pinia · Vue Router · @vueuse/core |
| 滑动窗口 | @zebra-ui/swiper |
| 并发搜索 | Rust tokio（JoinSet + 信号量限流）· scraper（CSS 选择器解析） |
| 自定义 DNS | hickory-resolver（保留 TLS SNI） |
| CI / CD | GitHub Actions（release.yml） |

## 快速开始

### 下载安装

从 [Releases](https://github.com/icenfn/DHThub/releases/latest) 下载对应平台安装包：

| 平台 | 文件 | 说明 |
| --- | --- | --- |
| Android | `DHThub-vX.Y.Z-android-arm64.apk` | 直接安装 |
| Linux | `DHThub-vX.Y.Z-linux-amd64.deb` / `.rpm` | Debian / RHEL 系发行版 |
| Windows | `DHThub-vX.Y.Z-windows-x64-setup.exe` | NSIS 安装包 |

### 从源码构建

前置要求：[Rust](https://rustup.rs) · [Node.js 20+](https://nodejs.org) · Tauri 平台依赖（参考 [Tauri 环境准备](https://tauri.app/start/prerequisites/)）

```bash
# 安装前端依赖
npm install

# 开发模式（桌面）
npm run tauri dev

# 前端类型检查 / 构建
npm run typecheck
npm run build

# 打包桌面产物
npm run tauri build
```

Android 打包（arm64）由 CI 完成，本地参考 `.github/workflows/release.yml`。

## 配置

### 订阅源

- 默认订阅仓库：`https://raw.githubusercontent.com/icenfn/DHThub/main/sites/default.json`
- 订阅源管理（搜索源页右下角悬浮按钮）：输入任意 GitHub Raw / JSON 地址拉取站点列表，支持导入 / 导出 / 重置；重新拉取采用**合并策略**，已有站点保留你的开关设置，新站点默认启用
- 站点 schema 见 [`sites/default.json`](sites/default.json)：`request`（方法 / 搜索地址模板 / 请求头 / 超时）+ `expression_model`（CSS 选择器提取规则）

### DNS

设置 → DNS 服务器：内置 4 个公共 DNS 可选可测速，支持自定义（IP 或 IP:端口）；搜索请求经所选 DNS 解析，未配置时使用系统默认。

### GitHub 镜像

设置 → GitHub 镜像：拉取订阅、检测更新等 GitHub 请求走所选镜像（内置直连 + 4 个：gh-proxy.com、axisnow.gh-proxy.org、cdn.gh-proxy.org、gh.dpik.top），支持自定义前缀代理与全部测速。

## 目录结构

```
DHThub/
├── sites/
│   ├── default.json          # 默认订阅源（9 站，CSS 选择器 schema）
│   └── hotwords.json         # 热门推荐热词总表
├── src/                      # Vue3 前端
│   ├── views/                # Home / SearchResults / Sites / History / Settings / About
│   ├── components/           # AppLayout（滑动窗口框架）/ 磁力详情
│   ├── stores/               # 站点状态（Pinia）+ 设置持久化
│   └── lib/                  # tauri / http / update / mirrors / dns
├── src-tauri/
│   ├── src/                  # Rust：search / sites / history / dns
│   ├── capabilities/         # 桌面 / 移动端权限
│   └── tauri.conf.json
├── android/                  # 稳定 debug 签名（覆盖安装不冲突）
└── .github/workflows/release.yml   # 自动构建发布
```

## 贡献

欢迎提交 Issue 与 PR：

1. Fork 本仓库并创建特性分支
2. 修改后本地通过 `npm run typecheck` 与 `cargo fmt --check`
3. 提交 PR，说明改动内容；涉及订阅源 / 打包的改动请在 PR 中附验证结果

## 免责声明

本项目仅用于技术学习与研究，搜索结果来源于第三方公开站点，请遵守当地法律法规，尊重版权。

## License

[MIT](LICENSE)
