# Changelog

## v0.3.0

- 搜索结果改为独立页面展示，首页仅保留搜索入口
- 首页改为滑动窗口内嵌 3 页（搜索 / 站点 / 历史），手机左右滑动切换，tab 跟随
- 应用栏左上角仅显示随 tab 变化的标题，右上角仅保留设置按钮
- 设置页新增自动检测更新开关（启动时静默检查，snackbar 提示）
- 整理设置页面结构（通用 / GitHub 镜像 / 订阅源 / 数据管理 / 关于）
- 修复更新检测/订阅请求被插件 HTTP 作用域拦截（url not allowed on the configured scope）
- 修复 Android 自适应图标被启动器放大裁切，按安全区重构图并匹配背景色
- README 更新至 v0.3.0

## v0.2.7

- 全平台（Android / Windows / Linux）应用图标更换为新版 HUB 图标
- README 更新为最新说明
- hotwords.json 迁移至 sites/ 目录，更新拉取路径
- 删除使用说明与免责声明相关代码与页面
- 更新检测与在线订阅重构：前端 plugin-http 直连拉取（DNS/建连不再挂起），统一 snackbar 提示，彻底移除弹窗
- 设置页删除 GitHub 镜像"选择方式"，镜像列表仅展示链接，直连保留「直连」标签
- CI 按 delin_ocr 方案重写：CHANGELOG 驱动版本号与发布说明，产物统一命名并汇总发布

## v0.2.6

- 更新检测改用 releases/latest 解析 + snackbar 提示，移除弹窗
- Android 仅编译 arm64 / armv7，移除 x86 架构
- 删除调试模式
- 设置页使用独立页面框架，App.vue 不再写死页面框架
- 修复在线订阅转圈问题
- 自动打包流程优化

## v0.2.5

- 设置页改为独立路由页面
- 自动检测更新开关；测速重构（参考 moretools 前端方案）
- GitHub 镜像删除 ghfast.top；镜像添加无需填写名称；列表仅显示链接，直连保留「直连」标签
- 删除调试模式
- Android 不再编译 x86 架构，不打包 AppImage / MSI
- 历史记录仅保留浏览记录
