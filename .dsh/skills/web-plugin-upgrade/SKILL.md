---
name: web-plugin-upgrade
description: >
  web profile 插件的正确升级命令、pnpm 冷静期放行方式、兼容性判断三件套与无浏览器验证方法。
whenToUse: >
  web profile 插件安装/升级/版本、`minimumReleaseAge` 冷静期、兼容性判断与无浏览器验证
---

# web profile 插件版本与升级

> 基线：2026-09-10，dsh 0.1.5-rc.1。

## 升级命令

用**受管版本**的 dsh，别用全局 0.1.2：

```
node "%APPDATA%\DSH Desktop\versions\<id>\node_modules\@deepseek-ai\dsh\lib\bin.js" plugin --profile web <pnpm 参数>
```

示例：

- `... plugin --profile web add "@linxin666/dsh-web-all@0.3.20"`
- `... plugin --profile web up --latest`

`dsh plugin` 只是 pnpm 转发器，成功后按**已安装状态**重建 `dsh.profile.bundles`。

## ⚠️ pnpm 11 冷静期

pnpm 11 默认 `minimumReleaseAge` 24h（供应链冷静期）。插件的「兼容新 dsh」版本往往和 dsh 同一天发布，会被静默降级到能通过冷静期的旧版本（日志：`[... was updated to X, not Y, to match the version preferred by your manifests]`）。

放行方式：在 `~/.dsh/profiles/web/pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 里按 `包名@版本` 逐条加入（该 profile 一直这么用），否则 `--latest` 拿不到真正的新版本。

## `--profile web add` 里不要写 `^`

dsh 在 Windows 用 `shell: true` 转发给 cmd.exe，`^` 是 cmd 转义符会被吃掉。给精确版本号（如 `@0.3.20`）即可，pnpm 会自己按 manifest 风格写入范围。

## 兼容性判断三件套

1. 插件 package.json 的 `dsh.engines.dsh`（如 `>=0.1.5-rc.1`）与 `dsh.compatibility.dshReleases`；
2. 宿主是否还导出插件 import 的 `@deepseek-ai/*` 符号；
3. 客户端 `require()` 的模块是否在「平台种子表」里。0.1.5-rc.1 的种子 = `react` / `react-dom` / `@deepseek-ai/cordis` / `dsh-client-store` / `dsh-client-ui-slots` / `dsh-client-ui-primitives` / `dsh-client-ui-dockkit`（来自 `dsh-web-frontend/dist` 的 staticModules）。

## 无浏览器验证

1. 另起端口启动：`node <bin.js> --profile web --no-open --port 3099`；
2. `curl -sL -c cookies "http://127.0.0.1:3099/?token=<打印的 token>"` 拉首页 → 触发客户端模块组装（失败会在宿主日志报 `client ... failed to compose` / `client bundles not found`）；
3. 首页里的 `/plugins/??<id>/client.js&rev=...` 组合 URL 用 cookie 请求应得 200 且体积正常；
4. 宿主侧看启动日志有没有 `failed to import loader entry`。

## 输出解读

`dsh plugin add` 输出里的 `Packages: +1 -12` 是 pnpm 清理陈旧存储，**不是删插件**。核对 `node_modules` 各包版本 + `package.json` 的 `dependencies` / `dsh.profile.bundles` 即可确认完整。
