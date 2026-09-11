---
name: liangshen-preset
description: >
  「梁神模式」agent 预设的来源、正确移除方式与遗留会话的表现。
whenToUse: >
  梁神模式（liangshen）预设的移除或残留
---

# 「梁神模式」预设的移除

> 2026-09-10。

## 来源

它来自 agent 预设目录 `~/.dsh/.agent-presets/liangshen/`（`preset.yml` 的 `name: 梁神模式`），由 `@linxin666/dsh-liangshen` 的宿主半边在启动时同步。

**该插件行在 `@linxin666/dsh-web-all` 聚合层里默认就是 `disabled: true`（id `web-ui-liangshen`）**，所以移走预设目录后不会被重建。

## 移除方式

把 `~/.dsh/.agent-presets/liangshen` 移到扫描目录之外（本次移到 `~/.dsh/_disabled-agent-presets/liangshen` 留底），刷新页面即从预设选择器消失。

## 遗留会话

**已被该预设创建的会话仍可正常打开**：会话头里的 `agentPreset: liangshen` 失去对应预设只会掉徽章，不报错。
