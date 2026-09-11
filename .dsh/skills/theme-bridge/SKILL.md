---
name: theme-bridge
description: >
  主题桥采样链路与「标题栏变黑」的根因及修复方式。
whenToUse: >
  标题栏底色、主题采样、theme_bridge、/set-theme、主题槽
---

# 主题桥采样与「标题栏变黑」

## 底色链路

内容页 `theme_bridge_js` 采样 html/body 的**真实渲染色** → `/set-theme` → Rust 存 `state.theme` 并 `push_theme`（chrome WebView 设 `--dshd-bg/--dshd-fg` + `set_background_color`）。

标题栏本身透明，颜色 = 窗口底色。

## 根因

`--dsw-alias-*` 只是皮肤（skin.css）里的主题变量；**官方默认皮肤下 html/body 背景是透明的**（底色由 `#root` / `[data-dsh-frame]` 等面板绘制）。

桥若只采样 html/body，会全部透明 → 落到硬编码回退 `#0b1220`（深蓝黑）→ 标题栏看起来「变纯黑」，且与官方浅色界面严重不匹配。

皮肤激活时皮肤给 html 显式背景色（蓝色幻想 `#e8ecf5` / `#101624`）→ 采样正常 → 标题栏贴合皮肤。

所以「切回原始皮肤 → 标题栏变黑」的根因是**桥的回退值**，不是皮肤。

## 修复

html/body 透明时继续向下采样 `[data-dsh-frame]` / `#root` / `[data-dsh-app]` 的真实渲染色；最终兜底也按 `data-ds-dark-theme` 区分（暗 `#0b1220` / 亮 `#ffffff`）。

## 排查注意

「刷新后好了、不刷新坏」具有迷惑性：刷新后走的是 boot 页/重新加载的初采样（可能恰好正常），不刷新的运行时切换才暴露真实采样结果。

**要看 `/theme` 接口的最终值**（`http://127.0.0.1:19431/theme`），不要凭肉眼时序下结论。
