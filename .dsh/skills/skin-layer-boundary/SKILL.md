---
name: skin-layer-boundary
description: >
  判断能否改 skin.json / hooks.mjs、皮肤分层 L1/L2/L3、fork 与发布路线选择。
---

# 改动边界：只碰 patches.css

## 皮肤分层模型
- **L1** token 重映射（`skin.css`）
- **L2** 语义属性选择器（`skin.css`）
- **L3** `patches.css` = 官方明说的「任意选择器、脆弱性作者自负」层
- 我们那 485 行追加正是 L3 的正当用法，**不需要 fork 上游**。
- 皮肤中心只提供"自定义主题"（仅审计过的 token，不接受任意 CSS），**没有**皮肤之外的 L3 叠加层，所以任意 CSS 只能写在某个皮肤的 `patches.css` 里。

## 为什么不能碰 skin.json / hooks.mjs
- 官方市场安装靠 `dsh-market.provenance.json` 做**字节校验**，钉的是 `skin.json` + hooks 入口。
- README 原文：「任何被修改、改名、手工投放或篡改的目录都会继续拒绝 hooks facet，声明式部分仍正常加载」。
- 改 `patches.css` **不影响**（实测 `GET /api/skin-center/v2/catalog`：blue-fantasy 只有一条 `shadows the built-in "blue-fantasy" skin` 提示，**零 provenance/hooks 拒绝告警**）。
- 一旦为了让版本号好看去改 `skin.json`，hooks 立刻被拒。

## blue-fantasy 内置皮肤说明
- blue-fantasy 是 skin-center 包内内置皮肤，用户目录同 id 会遮蔽内置（那条 warning 就是这意思）。
- 其 `hooks.mjs` 只有 25 行、**只注入 favicon**（`assets/whale-icon.png`）。
- 背景壁纸是**声明式** `contributes.backgroundMedia`、由皮肤中心渲染——所以即使自建皮肤 id 把 hooks 弄丢，代价也仅是一个 favicon。

## 路线选择
- **就地维护** → 现状（市场皮肤 + L3 + 仓库存档 + `skin-patches.ps1`）
- **彻底解耦** → 复制成自己的皮肤 id（改 `skin.json` 的 id/name；代价：与原皮肤列表并存、丢 hooks=favicon、上游对 `skin.css`/资源的改进要手动搬）
- **分享/发布** → 一次性 PR 到 `github.com/zhu1090093659/dsh-web` 的 `skins/`（市场条目由该仓库生成），**不必长期 fork**
