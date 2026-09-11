---
name: environments
description: >
  环境管理：版本与 Profile 面板、切换 Loading、/env 与 env-* 动作、profile 参数名冲突。
---

# 环境管理（environments.rs）

- 入口：⋯ 菜单「环境管理」→ 外壳层 `popup-env` 弹层（版本 + Profile 双列）。Rust 侧通过 `GET /env`（只读）与 `/action?name=env-*`（需令牌）交互；长耗时安装/更新走后台任务，`AppState.env_task` 存进度，`window.__dshdEnvChanged` 通知面板刷新。
- **不使用前端轮询**：面板打开时拉一次 `/env`，之后全部由事件驱动——安装进度/结果用 `__dshdEnvChanged`，切换完成用 `backend-status`（Rust `emit`/`publish` 会同步 eval 到 chrome WebView）。不要加 `setInterval` 轮询。
- **切换环境的全屏 Loading 是注入到 DSH 内容 WebView**（`controls.rs::switch_loading_js` / `switch_loading`），只覆盖内容 WebView 区域、不盖标题栏；面板内另一条「切换中」横幅，后端 `running/external/error/stopped` 状态事件到达后自动收起，不要用整窗外壳层弹层做切换遮罩。
- **版本**：每个受管版本安装到 `%APPDATA%\DSH Desktop\versions\<id>\`（`npm install --prefix <dir> @deepseek-ai/dsh@<spec>`），切换只改 `settings.dsh_bin`，**不影响当前正在运行的实例**（下次重启/切换才生效）；全局安装与手动路径只展示/使用，不能由桌面端删除或更新。
- **Profile**：以 `$DSH_HOME/profiles/<name>` 目录为准扫描（读 package.json 的 `dsh.profile.bundles`/dependencies）。新建=创建 `@deepseek-ai/dsh-base` + `@deepseek-ai/dsh-web-app` 的空模板（立即可启动）；复制可带 node_modules（完整现场改造/修复），复制时修正 manifest 的 `name`。
- **当前使用中的版本/Profile 不再显示 使用/重命名/删除 按钮**，Rust 侧同样拒绝（防误操作）。
- 首次启动若没有任何版本，后端会改为安装一个「桌面端管理」版本（不再 npm -g）；启动时 `environments::ensure_seed_versions` 会把当前 dsh（全局/手动）登记进列表。
- 插件变更监控随 profile 切换自动重新 watch（每循环读取 settings.profile，变化即重建 watcher）。
