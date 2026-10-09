---
name: environments
description: >
  环境管理：版本与 Profile 面板、切换 Loading、/env 与 env-* 动作、profile 参数名冲突。
---

# 环境管理（environments.rs）

- 入口：⋯ 菜单「环境管理」→ 外壳层 `popup-env` 弹层（版本 + Profile 双列）。Rust 侧通过 `GET /env`（只读）与 `/action?name=env-*`（需令牌）交互；长耗时安装/更新走后台任务，`AppState.env_task` 存进度，`window.__dshdEnvChanged` 通知面板刷新。
- **不使用前端轮询**：面板打开时拉一次 `/env`，之后全部由事件驱动——安装进度/结果用 `__dshdEnvChanged`，切换完成用 `backend-status`（Rust `emit`/`publish` 会同步 eval 到 chrome WebView）。不要加 `setInterval` 轮询。
- **切换环境的全屏 Loading 是注入到 DSH 内容 WebView**（`controls.rs::switch_loading_js` / `switch_loading`），只覆盖内容 WebView 区域、不盖标题栏；面板内另一条「切换中」横幅，后端 `running/external/error/stopped` 状态事件到达后自动收起，不要用整窗外壳层弹层做切换遮罩。
- **版本**：每个受管版本安装到 `%APPDATA%\DSH Desktop\versions\<id>\`（`npm install --prefix <dir> @deepseek-ai/dsh@<spec>`），切换只改 `settings.dsh_bin`，**不影响当前正在运行的实例**（下次重启/切换才生效）；手动路径版本只展示/使用，**不能**由桌面端删除或更新。
- **全局 dsh 可以就地更新（2026-10-09 起）**：`update_version` 对 `source == "global"` 的条目走 `npm install -g --prefix <prefix> @deepseek-ai/dsh@<spec>`；未填 spec 默认 `latest`。
  - 前缀一律由 **bin.js 反推**（`prefix_of_bin` / `global_prefix_of`）：`Path::parent()` 作用在**文件**路径上，第一次返回的就是 `lib` 目录，**再退 4 级**即到 `<prefix>`（lib→dsh→@deepseek-ai→node_modules→prefix）。
    - ⚠️ 这里连错两次（3 级 → `...\@deepseek-ai`、5 级 → 前缀的上一级），且坏值被写进过 `settings.json` 的 `dir`。**唯一可靠判据是：结果目录下必须同时存在 `node_modules\@deepseek-ai\dsh`**，别用数层数的办法。
    - ⚠️ 用 PowerShell `Split-Path -Parent` 数次数会**多算一级**（它对文件和对目录取父的起点不同），别拿它跟 Rust 的 `parent()` 对照。要验证就用 Rust 小程序直接打印。
    - `global_prefix_of` 现在对**历史脏 `dir`** 有兜底：从 `dir` 逐级上溯，找到第一个含 `node_modules\@deepseek-ai\dsh` 的祖先。实测本机 `dir` 存的是 `...\npm\node_modules\@deepseek-ai`（错的），靠这条兜底能纠正。
  - 显式传 `--prefix`（而非依赖 npm 全局前缀推断）保证「升级的就是面板上这一个」；实测该值与 `npm prefix -g` 一致，且跨版本升级是就地 `change` 而非新增。
  - **更新后会重读 `installed`、刷新 `bin`/`dir`，并在标签里同步新版本号**（否则面板一直显示旧版本）；若该全局版本正是当前 `dsh_bin`，一并回写。
  - ⚠️ **千万不要在持有 `settings.lock()` 时再 `settings.lock()`**：`std::sync::Mutex` 不可重入 → **死锁**。`update_version` 开头就拿了读锁，写回前必须先 `drop(s)`（managed 分支本来就有，global 分支 2026-10-09 漏了，实测死锁）。
    - **症状极具欺骗性**：npm 安装成功并打出「全局 dsh 更新完成」→ 之后**既无成功行也无失败行**，settings.json **不更新**（mtime 停在启动那一刻），`env_task` 永不结束（面板一直"处理中"）。
    - 根因难查是因为 `save_settings` 和 `Logger::write` 都是 `let _ = ...` / `if let Ok(...)`，**失败全被吞掉**。排查时以 **settings.json 的 mtime** 为准，别只信日志。
    - 另外别把读锁跨在 npm 安装（几十秒）上，会让 `/env` 与面板整体卡住。
  - **前端更新表单的标签框要留空**：`showVerForm` 原先把 `v.label` 预填进输入框，全局条目更新时会把旧标签（含旧版本号）原样回传，后端就不再自动生成新标签 → 面板永远显示旧版本。已改为全局条目在 `update` 模式下 `labelVal = ''`。
  - 全局安装改的是共享 npm 前缀，用 `global_lock()` + `GlobalUpdateGuard`（持有 `MutexGuard`）串行化。
  - 全局条目仍**不提供重命名/删除**（那是系统安装的，删了会连带毁掉其它全局包）。
  - **面板里全局条目的 `installed` 可能过期**：实测本机全局实际是 `0.2.1-alpha.1`，而 `settings.json` 里仍写着 `0.1.7-rc.2`（种子登记后从没重读过）。`ensure_seed_versions` 只在「bin 路径命中已有条目」时刷新 `installed`，手工 `npm i -g` 升级后不会自动更新；点一次「更新」（或切换后重启）即会纠正。
- **沙箱下单测夹具不要写 `%TEMP%`**：DSH 沙箱会让子进程 `create_dir_all` 报 `PermissionDenied (os error 5)`；改用 `env!("CARGO_MANIFEST_DIR")/target/...` 并在结尾自行删除。crate 是 bin-only，单测要跑 `cargo test --bin dsh-desktop`（`--lib` 会报 no library targets）。
- **Profile**：以 `$DSH_HOME/profiles/<name>` 目录为准扫描（读 package.json 的 `dsh.profile.bundles`/dependencies）。新建=创建 `@deepseek-ai/dsh-base` + `@deepseek-ai/dsh-web-app` 的空模板（立即可启动）；复制可带 node_modules（完整现场改造/修复），复制时修正 manifest 的 `name`。
- **当前使用中的版本/Profile 不再显示 使用/重命名/删除 按钮**，Rust 侧同样拒绝（防误操作）。
- 首次启动若没有任何版本，后端会改为安装一个「桌面端管理」版本（不再 npm -g）；启动时 `environments::ensure_seed_versions` 会把当前 dsh（全局/手动）登记进列表。
- 插件变更监控随 profile 切换自动重新 watch（每循环读取 settings.profile，变化即重建 watcher）。
