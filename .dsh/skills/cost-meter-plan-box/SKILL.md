---
name: cost-meter-plan-box
description: >
  Coding Plan 图框与 Go 图框版式不一致的根因、本地补丁做法、锚点唯一性要求与验证标准。
whenToUse: >
  Coding Plan 图框与 Go 图框版式不一致、修改 `client.js` 图框组件
---

# Coding Plan 图框与 Go 图框「不同款」＝ upstream 名不副实

> 2026-09-11，已本地补丁。

## 症状

README 写「侧边栏卡片**与 Go 额度同款**」，但 CommandCode 卡片 256×106、Go 卡片 256×90，**版式根本不同**。

## 根因

侧边栏聚合处按厂商分派**两个不同组件**：`x.map(A=>A==="minimax"?e(fn,…):e(xn,{id:A,…}))`——只有 MiniMax 走专用组件，**其余 9 家（含 commandcode）都走通用组件 `xn`**，而 `xn` ≠ Go 的 `Vt`：

| | Go `Vt` | Coding Plan `xn` |
|---|---|---|
| 标题 | `cm-bbox-head`：标题与主窗口%同一行 | `cm-mm-title`：标题**独占一行** |
| 进度条 | `cm-bbox-bar` **全宽** | `.cm-mm-row` 内短条 |
| 次要窗口 | `cm-bbox-line`：`周 28% · 月 99%` | 每窗口各自一行「标签+条+%」 |
| 重置时间 | `cm-bbox-line` | **完全没有** |

隐藏差异：`xn` 外层多一个 `cm-mm` 类，`.cm-mm{gap:4px}` 覆盖 `.cm-bbox{gap:6px}` → 行距差 2px。

## 上游没修

**1.7.20（npm 最新）与 1.7.19 的 `xn` 逐字节同构** → 升级插件解决不了。`client.js` 是 esbuild 产物、`scripts/` 未随包发布 → 只能打本地补丁。

## 补丁

`scripts\cost-meter-plan-box.mjs`（支持 `--check` / `--revert`，备份 `lib/client.js.orig-plan-box`），改 2 处：

1. 正文换成 Go 四段式；
2. 外层 className 去掉 `cm-mm`（对齐 `gap`）。

## ⚠️ 锚点必须唯一

`"cm-bbox cm-mm clickable"` 出现 **3 次**（`vn` / `xn` / `Nn` 三个图框），第 2 处的锚点须带 `+(M==="ok"?"":" "+M)`（`M` 只在 `xn` 作用域内）才唯一。**写这类补丁先数 `split(needle).length-1`。**

## 验证

`Page.reload {ignoreCache:true}` 后两卡片逐项一致（standard `256×90` = `256×90`、compact `256×78` = `256×78`、simple `35` vs `36` 为文字宽度舍入）；DOM 子节点同为 `cm-bbox-head/bar/line/line` 四段；控制台零 error/exception；`compact` / `simple` / `rail` 均不破版。

## 已知残留（有意未改）

rail（`wide===false`）仍不一致——Go 只显示主窗口一个%（`0%`），Coding Plan 显示前两个（`3% 1%`），是上游多窗口设计（`w=P.slice(0,2)`）。

## 生效

只需重载内容页（F5），**不用重启后端**。⚠️ `dsh-cost-meter` 升级会覆盖 `client.js` → 重跑脚本（幂等）。
