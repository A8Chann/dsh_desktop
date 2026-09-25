---
name: web-plugin-upgrade
description: >
  web profile 插件的正确升级命令、pnpm 冷静期放行方式、兼容性判断三件套与无浏览器验证方法。
whenToUse: >
  web profile 插件安装/升级/版本、`minimumReleaseAge` 冷静期、兼容性判断与无浏览器验证
---

# web profile 插件版本与升级

> 基线：2026-09-24，dsh **0.1.7-rc.1**（受管版本 `v18d385b479a67f08`；全局 npm dsh 仍是 0.1.5-rc.2，勿混用）。

## dsh 0.1.7-rc.1 适配矩阵（2026-09-24 实测）

| 插件 | 0.1.5 时代 | 0.1.7-rc.1 目标版 | 判据 |
|---|---|---|---|
| `@linxin666/dsh-web-all` | 0.3.22 | **0.4.1** | `engines.dsh >=0.1.7-rc.1`；子包 0.4.1 / 0.3.25（pet、skin-center、community-plugins） |
| `@mars-sea/dsh-commandcode-provider` | 0.11.8 | **0.11.13** | `engines.dsh ^0.1.7-rc.1`、`dshReleases.0.1.7-rc.1=compatible`；0.11.8 的范围 `^0.1.6-alpha.2||…` **不含** 0.1.7-rc.1 |
| `dsh-context` | 0.54.2 | **0.55.0** | `dshReleases` 新增 `0.1.7-alpha.2: compatible` |
| `dsh-cost-meter` | 1.7.19 | **1.7.35** | ⚠️ 见下「typert codec」——1.7.19 在 0.1.7 上**直接让 typert-loader 条目激活失败** |
| `dshmarket` | 1.54.0 | 1.64.0 | 无 engines 声明，随生态升级 |
| `@liustack/modsearch` | 5.10.3 | 5.10.5 | 无 dsh 声明，小版本 |
| 保持不动 | memos-cloud 0.1.1 / opencode-session 0.1.1 / rule-manager 0.1.0（均无更新版本）；cnb-refresh、pet-live2d 为本地包 | | |

**0.1.7 的 typert codec 断崖**：`dsh-typert-loader` 的 `requireStrictCodec` 要求 codec 有 `create()` 工厂（`lib/index.js` L211）。旧插件 codec 形态 `{mode,typeSymbol,schema}` 会报
`invocation "…" result codec has no create() factory` → `typert-loader` 条目不激活（启动日志 `warning: 1 entry did not activate`）。
cost-meter 从 master 起改为 `strictCodec = (name, schema) => ({ mode:'strict', typeSymbol:…, schema, create: () => schema })`（新旧宿主通吃），npm 1.7.35 已含。**冒烟测试看到这条错就必须升 cost-meter。**

### web-all 0.4.x 的依赖面变化（升级前须知）

- 剪掉 `@linxin666/dsh-doctor`、`@linxin666/dsh-tool-describe-image`、`dsh-better-sidebar`（0.3.22 时代有）；`plugin add` 输出 `+23 -173` 属正常清理。
- `~/.dsh/skins/` 下的市场皮肤（patches.css 等）**不在 node_modules**，升级 web-all/skin-center 不会覆盖。
- 「梁神模式」预设靠「目录移出扫描区 + 插件行默认 disabled」双保险，升级 dsh-liangshen 不会复活。

## 升级命令

用**受管版本**的 dsh，别用全局 0.1.5-rc.2：

```
node "%APPDATA%\DSH Desktop\versions\<id>\node_modules\@deepseek-ai\dsh\lib\bin.js" plugin --profile web <pnpm 参数>
```

示例：

- `... plugin --profile web add "@linxin666/dsh-web-all@0.4.1"`
- `... plugin --profile web up --latest`

`dsh plugin` 只是 pnpm 转发器，成功后按**已安装状态**重建 `dsh.profile.bundles`。

## 桌面壳切到 minimal 保底时的升级流程

用户在环境面板把 dsh 切到 0.1.7-rc.1 并切到 `minimal` profile（无插件）后，后端跑 `--profile minimal`，**插件 watcher 只盯 minimal 目录**——改 web profile 不会出蓝色药丸。升级完 web profile 插件后需用户在环境面板**切回 web profile**（重启后端）才生效；切回前先用 3099 冒烟（见下）确认能启动，避免切回去起不来。

## 改完插件如何生效

`dsh plugin ...` 只改 `~/.dsh/profiles/<profile>` 下的依赖，**运行中的后端不会重新加载**：
文件变更被 `start_plugin_watcher` 检测到（6s 安静期）后，标题栏状态药丸会变成蓝色的
「点击重启更新插件」——点它即重启后端（等价菜单「重启 Web 服务」），同时会弹一条系统 toast；
重启就绪后内容页自动刷新，新插件才真正生效。

## ⚠️ pnpm 11 冷静期

pnpm 11 默认 `minimumReleaseAge` 24h（供应链冷静期）。插件的「兼容新 dsh」版本往往和 dsh 同一天发布，会被静默降级到能通过冷静期的旧版本（日志：`[... was updated to X, not Y, to match the version preferred by your manifests]`）。

放行方式：在 `~/.dsh/profiles/web/pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 里按 `包名@版本` 逐条加入（该 profile 一直这么用），否则 `--latest` 拿不到真正的新版本。

**聚合包子依赖也要逐条放行**：web-all 0.4.1 钉死 14 个 `@linxin666/*@0.4.1` + 3 个 `^0.3.25` 浮动包，精确钉住的版本撞冷静期会直接解析失败（无法像范围那样降级）。2026-09-24 的做法：先查清各子包最新版（registry `/latest`），在 yaml 里补一整块 `包名@版本` 行（scoped 包名带单引号），再跑 `plugin add`。pnpm 11.24 命中冷静期时也会自己往该文件追加一行并打印 `Added 1 entry to minimumReleaseAgeExclude`，但显式先加更稳。

## `--profile web add` 里不要写 `^`

dsh 在 Windows 用 `shell: true` 转发给 cmd.exe，`^` 是 cmd 转义符会被吃掉。给精确版本号（如 `@0.3.20`）即可，pnpm 会自己按 manifest 风格写入范围。

## 兼容性判断三件套

1. 插件 package.json 的 `dsh.engines.dsh`（如 `>=0.1.7-rc.1`）与 `dsh.compatibility.dshReleases`；
2. 宿主是否还导出插件 import 的 `@deepseek-ai/*` 符号；
3. 客户端 `require()` 的模块是否在「平台种子表」里。0.1.7-rc.1 的种子与 0.1.5 相同（9 项）：`react` / `react/jsx-runtime` / `react-dom` / `react-dom/client` / `@deepseek-ai/cordis` / `dsh-client-store` / `dsh-client-ui-slots` / `dsh-client-ui-primitives` / `dsh-client-ui-dockkit`（来自 `dsh-web-frontend/dist/assets/index-*.js` 里的 `staticModules` 函数）。
   `dsh.client.inject` 里声明的 `@deepseek-ai/dsh-api-remotes` 等**不是种子词**，而是 web-app bundle 图里的包行（graph row），由加载器按包名装载——0.1.7 与 0.11.8/0.11.13 的 inject 列表一致，无新增风险。

## 无浏览器验证

1. 另起端口启动：`node <bin.js> --profile web --no-open --port 3099`（用 `run_in_background` 的 pwsh job 起，**不要** `Start-Process`——本机 NO_PROXY 键冲突会直接抛异常）；
2. `curl -sL -c cookies "http://127.0.0.1:3099/?token=<打印的 token>"` 拉首页 → 触发客户端模块组装（失败会在宿主日志报 `client ... failed to compose` / `client bundles not found`）；
3. 首页里的 `/plugins/??<id>/client.js&rev=...` 组合 URL 用 cookie 请求应得 200 且体积正常（2026-09-24 实测两条组合包 8.2 MB / 2.7 MB）；
4. 宿主侧看启动日志有没有 `failed to import loader entry`、`warning: N entry did not activate`、`codec has no create() factory`——**最后一条专杀 cost-meter ≤1.7.19**。
5. 收尾：`job_kill` + `Stop-Process` 残留 node，确认端口释放、主实例 `/status` 仍 `running`。

## 输出解读

`dsh plugin add` 输出里的 `Packages: +1 -12` 是 pnpm 清理陈旧存储，**不是删插件**。核对 `node_modules` 各包版本 + `package.json` 的 `dependencies` / `dsh.profile.bundles` 即可确认完整。
