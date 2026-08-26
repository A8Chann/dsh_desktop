# DSH Desktop 🖥️

DeepSeek Harness 的 Windows 桌面端。把 `dsh web` 从「cmd 敲命令 + 浏览器开标签页」变成双击即用的桌面应用：

- **自动拉起后端**：应用启动时自动找到 dsh（手动指定 / PATH / npx 缓存兜底），找不到时**先测速多个 npm 镜像源、挑最快的一个，再用 npm 自动安装**（不再用 npx），随后以子进程拉起 `dsh web`，解析就绪 URL 后直接内嵌加载 GUI——没有 cmd 窗口，不用手动开浏览器。
- **旧实例智能接管**：如果 3080 端口已经有一个 dsh web 在跑（比如之前 cmd 启动的），应用会**直接接管复用**它，你的会话无缝延续；该实例消失后自动接管拉起自己的后端。
- **插件变更提示手动重启**：监控 profile 目录的 `package.json` / `pnpm-lock.yaml`。插件安装/移除完成后，应用**不再自动重启**，只弹出系统通知提示你手动重启（悬浮条「重启」或 `Ctrl+Shift+R`）来加载新插件。
- **崩溃自愈**：后端意外退出时按退避策略（1s→30s）自动重启，悬浮条实时显示状态。
- **端口冲突自愈**：端口被非 dsh 服务占用时自动回退 `--port 0`（系统随机端口）。

## 快速开始

```bash
npm install        # 安装依赖（electron 等）
npm start          # 开发模式启动
```

或直接使用打包产物：`dist/DSH-Desktop-1.3.0-portable.exe`（免安装，双击即用）。

> 只需本机装有 **Node.js**。首次启动若未找到 dsh，应用会**同时测速多个 npm 镜像源、挑最快的一个**，再用 `npm install -g @deepseek-ai/dsh`（而非 npx）**自动安装**（需联网，稍等片刻），并把装好的入口写回设置，下次直接复用。

重新打包：

```bash
npm run icon      # （可选）用 make-icon.js 重新生成 assets/icon.png
npm run icon:ico  # （可选）从 assets/icon.png 生成全尺寸 assets/icon.ico（打包 exe 图标用）
npm run dist      # 打包 Windows portable 单文件 exe
```

## 使用

| 操作 | 方式 |
|---|---|
| 重启后端 | 悬浮条「重启」按钮 / 托盘菜单 / 菜单「应用 → 重启后端」/ `Ctrl+Shift+R`（重启后前端自动刷新） |
| 刷新前端页面 | 悬浮条「⟳」按钮 / 菜单「视图 → 重新加载页面」 |
| 在系统浏览器打开 | 悬浮条「↗」/ 托盘 / 菜单 / `Ctrl+Shift+O` |
| 查看日志 | 托盘或菜单「打开日志目录」 |
| 开发者工具 / 缩放 | 菜单「视图」 |

**悬浮条**（窗口顶部居中的小胶囊）显示后端状态并可拖动：

- 🟢 绿点 = 运行中（显示端口；外部实例会标注「外部实例」）
- 🟡 黄点 = 启动中 / 重启中
- 🔴 红点 = 后端异常（显示下次重试秒数）

**托盘图标**常驻系统托盘：显示主窗口、重启后端、在浏览器打开、打开日志目录、退出。

## 插件变更后的处理（提示手动重启）

1. 在 GUI 里让 agent 执行 `dsh plugin --profile web add <包名>`（或任何会改 profile 依赖的操作）；
2. 应用检测到 `~/.dsh/profiles/web/package.json` 或 `pnpm-lock.yaml` 变化；
3. 等待文件稳定并确认 pnpm 安装进程已结束；
4. 弹出系统通知「插件已变更 · 请手动重启后端」——**不会自动重启**；
5. 你手动点击悬浮条「重启」/ `Ctrl+Shift+R` 后，后端重启完成，前端页面**自动刷新**并加载新插件。

> 说明：`autoRestartAfterPluginChange` 字段已保留但不再启用自动重启，改为仅提示。`restartCountdownSec` 字段已不再使用。

## 插件安装控制通道（agent 协作）

**问题**：agent（dsh web 会话）在会话内直接跑 `dsh plugin add` 时，安装完成后后端会自动重启，把 agent 正在进行的回合掐断——用户看到"话说到一半就没了"。

**方案**：agent 把安装请求**交给 Electron 主进程执行**（主进程不随后端重启），安装完整结束后由现有插件变更流程提示用户手动重启；结果写回控制目录，agent 重启恢复会话后读取并汇报。

协议（agent 侧）：

1. 写控制目录 `cmd-<id>.json`：
   ```json
   {
     "cmd": "install-plugin",
     "spec": "dsh-open-in-vscode 或 https://.../archive/refs/tags/v0.1.5.tar.gz",
     "id": "inst-20260814-1800-abc",
     "profile": "web",
     "timeoutMs": 300000
   }
   ```
2. 轮询 `result-<id>.json` 直到 `state ∈ {done, failed, timeout}`；或直接结束回合，手动重启后端、会话恢复后读该文件汇报。

控制目录默认 `%APPDATA%\DSH Desktop\control`（可用环境变量 `DSH_DESKTOP_CONTROL_DIR` 覆盖，测试/多实例隔离用）。

推荐工作流：agent 发起安装后**立刻结束回合**（提示"安装进行中，请稍后手动重启后端"）→ 用户手动重启后会话恢复，agent 核对 `result-<id>.json` 与 `node_modules` 后汇报结果。

## 设置

配置文件：`%APPDATA%\DSH Desktop\settings.json`（首次运行自动生成）：

```jsonc
{
  "port": 3080,                            // dsh web 监听端口；0 = 随机端口
  "workspace": "C:\\Users\\HWX",           // dsh 后端工作目录（agent 文件操作根目录）；null = 用户主目录
  "closeToTray": false,                    // 关闭窗口时最小化到托盘（true）还是退出（false）
  "autoRestartAfterPluginChange": false,   // 保留字段：插件变更后仅提示手动重启（不再自动重启）
  "restartCountdownSec": 6,                // 保留字段：旧版自动重启倒计时秒数（已不使用）
  "profile": "web",                        // 启动的 dsh profile
  "nodeBin": null,                         // 手动指定 node.exe 路径（null = 自动探测）
  "dshBin": null                           // 手动指定 dsh 入口（bin.js 或 dsh.cmd，null = 自动探测）
}
```

## 工作原理

```
┌──────────────────────────────────────────────┐
│  DSH Desktop (Electron 主进程)                │
│  ┌────────────────────────────────────────┐  │
│  │ BrowserWindow ← 加载 dsh web GUI        │  │
│  │  + 悬浮控制条(preload 注入)             │  │
│  └────────────────────────────────────────┘  │
│  后端管理器 (lib/backend.js)                 │
│   ├─ spawn node <dsh bin.js> web --port N    │
│   ├─ 解析 stdout "dsh web: http://127.0.0.1" │
│   ├─ 端口探测 → 接管外部实例 / 回退随机端口   │
│   ├─ fs.watch profile 依赖文件 → 提示手动重启  │
│   └─ 意外退出 → 退避重启                     │
│  托盘 / 菜单 / 系统通知                       │
└──────────────────────────────────────────────┘
```

关键点：

- dsh 入口自动探测顺序：`settings.dshBin` → PATH 上的 `dsh`（npm 全局安装）→ npx 缓存（旧安装兜底）；都找不到时**先测速选最快的 npm 源**，再用 `npm install -g @deepseek-ai/dsh` 自动安装并写回 `settings.dshBin`（不再用 npx，首次需联网）。
- 就绪信号 = dsh stdout 打印的 `dsh web: http://127.0.0.1:<port>`（`--port 0` 时也是从这行拿到真实端口）。
- 外部实例识别：向端口发 `GET /`，响应含 `__DSH_BOOT__` 即判定为 dsh web。
- 日志：`%APPDATA%\DSH Desktop\logs\main.log`（超 5MB 自动轮转）。

## 排障

| 现象 | 处理 |
|---|---|
| 首次启动提示正在自动下载 dsh | 正常：先测速选最快 npm 源再用 `npm install -g @deepseek-ai/dsh` 自动安装（需联网，装完写回 dshBin 直接复用） |
| 自动下载失败（无网络） | 联网后重试，或先在任意终端跑一次 `npm install -g @deepseek-ai/dsh`，或在 settings.json 指定 `dshBin` |
| 端口被占且不是 dsh | 应用自动用随机端口；也可在 settings.json 改 `port` |
| 插件变更后没提示 | 看日志是否「pnpm 仍在运行」卡住（等待安装结束会重新提示）；提示后手动用悬浮条「重启」/ `Ctrl+Shift+R` 重启 |
| 重启后页面还是旧的 | 已自动刷新（重启完成后强制 reload）；仍异常可用悬浮条「⟳」手动刷新 |
| 想恢复 cmd 方式 | 关掉应用后 `npm install -g @deepseek-ai/dsh && dsh web` 照常可用，互不影响 |
| 任务栏图标显示 Electron/空白 | 开发模式（npm start）显示 electron.exe 默认图标属正常；打包版请确认 `build.win.icon` 指向 `assets/icon.ico`（16~256 全尺寸）；portable 单文件每次解压到临时目录，任务栏图标偶发空白属已知现象，图标稳定可改用 `npm run dist:dir`（目录版固定路径） |
| 悬浮条碍事 | 按住拖动到别处，或点「–」折叠成小圆点 |

## 目录结构

```
main.js                  Electron 主进程（窗口/菜单/托盘/IPC/倒计时编排）
preload.js               悬浮控制条注入 + 状态页桥接
lib/
  backend.js             dsh 后端进程管理器（核心：拉起/接管/监控/重启）
  dsh-install.js         默认安装 dsh（测速选最快 npm 源 + npm 安装，不再用 npx）
  dsh-resolve.js         node.exe 与 dsh 入口自动探测
  settings.js            设置读写
  logger.js              文件日志
  status-page.html       后端未就绪时的本地状态页
  mock-backend.js        Mock dsh 后端（自动化自测用，DSH_DESKTOP_MOCK=1）
scripts/make-icon.js     图标生成（纯 Node PNG 编码，256x256）
scripts/make-ico.js      从 assets/icon.png 生成全尺寸（16~256）assets/icon.ico，打包 exe 用
assets/icon.png          应用图标
```

## 测试

内置自动化自测（Mock 模式 + 控制通道）：

```bash
$env:DSH_DESKTOP_MOCK='1'
$env:DSH_DESKTOP_PROFILE='<临时 profile 目录>'
$env:DSH_DESKTOP_TEST_CONTROL='<控制目录>'
npm start
# 向控制目录写入 JSON 命令：{"cmd":"screenshot","path":"..."} / {"cmd":"status","path":"..."} / {"cmd":"restart"} / {"cmd":"cancel"} / {"cmd":"quit"}
```

已覆盖：启动与 URL 解析、插件变更自动重启（含倒计时截图）、取消、手动重启、退出清理、外部实例接管（真实 3080 实例）。
