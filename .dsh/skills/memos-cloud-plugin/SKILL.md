---
name: memos-cloud-plugin
description: >
  @memtensor/memos-cloud-dsh-plugin 与 dsh 0.1.2 的导出不兼容问题、本地补丁、部署方式及 0.1.1 回归上游后的处理。
whenToUse: >
  memos-cloud-dsh-plugin 兼容、`installSettingsSection`/`settingsNamespace` 报错、该插件部署或迁移
---

# memos-cloud-dsh-plugin 兼容修复

## 背景

web profile 的 `@memtensor/memos-cloud-dsh-plugin@0.1.0` 依赖 `@deepseek-ai/dsh-settings` 旧导出（`installSettingsSection` / `settingsNamespace`），dsh 0.1.2-alpha.2 已移除 → `dsh web` 启动即挂（`does not provide an export named 'installSettingsSection'`）。临时方案是用 `--profile resolve`（不含该插件）。

## 上游历史

npm 最新长期停在 0.1.0（2026-08-16）；GitHub `MemTensor/MemOS-Cloud-Dsh-Plugin` 最后提交同日（0.1.0-beta.1），`MemOS-Cloud-OpenClaw-Plugin` 已删掉 `packages/dsh` 子包。因此曾把补丁固化在 `assets/memos-cloud-dsh-plugin-fixed/`（详见其 `PATCH-NOTES.md`）。

## 补丁内容（历史）

- `installSettingsSection(ctx, ...)` → `ctx.inject(["settings"], (sctx) => sctx.settings.installSection(ctx, ...))`
- `settingsNamespace(ns)` → 直接字符串
- 其余 peer 导入（`launchEnvironmentOf` / `isAppendSurfaceEvent` / `credentialRef` / `createUserMessage`）当时仍在，未动。

## 部署（历史，勿再执行）

`pwsh scripts\apply-memos-plugin-fix.ps1`（复制到 `~/.dsh/profiles/web/vendor/` + package.json 改 `link:` 依赖 + `pnpm install`）。补丁随 `link:` 依赖在重建 node_modules 后保留；`dsh plugin add/remove` 不破坏它，但**再次 `dsh plugin add @memtensor/memos-cloud-dsh-plugin`（registry 版本）会替换 `link:` 依赖**，需重跑脚本。

## ✅ 2026-09-10 已回归上游

上游 `0.1.1`（npm 2026-09-07 发布）改用新 API `settings.register(ns, schema, {base, validate})`，不再导入 `installSettingsSection` / `settingsNamespace`。web profile 依赖已改为 `"@memtensor/memos-cloud-dsh-plugin": "^0.1.1"`（registry 包）。

**`vendor/memos-cloud-dsh-plugin` + `scripts/apply-memos-plugin-fix.ps1` + `assets/memos-cloud-dsh-plugin-fixed` 仅作历史兜底，勿再执行**（执行会把依赖改回 `link:`）。

命名空间仍是 `memos-cloud`，`~/.dsh/settings.yaml` 的 `memos-cloud:` 配置（`apiKeyEnv` / `userId`）原样沿用，无需迁移。

## 验证注意

本地验证 web profile 完整启动若与正在运行的本机会话冲突，会报 `task-board ledger is already owned by process <pid>`（task-board 单实例锁）。用 `--patch` 临时 overlay 禁用 `web-ui-task-board` 条目即可验证；`--patch` 必须放在 `--no-open` 等透传参数**之前**。
