---
name: theme-bridge
description: >
  主题桥采样链路：标题栏底色不跟随（变黑 / 与页面不符）、启动页标题栏、/set-theme 与主题槽。
whenToUse: >
  标题栏底色、主题采样、theme_bridge、/set-theme、主题槽、启动页标题栏颜色
---

# 主题桥采样与标题栏底色

## 底色链路（当前实现）

内容页 `theme_bridge_js(src)` 采样 → `img /set-theme?src=dsh|deepseek&t={bg,fg}` → Rust 存进
`state.theme_dsh / theme_deepseek` → **只推「当前显示页」的主题** → `push_theme` → chrome WebView 设
`--dshd-bg / --dshd-fg`（内联变量）+ `apply_window_effect` 选材质明暗。

标题栏 `#bar` 背景 = `color-mix(in srgb, var(--dshd-bg) var(--dshd-tint), transparent)`，叠在窗口材质上；
所以「颜色对不对」看 `--dshd-bg`，「浓淡」看设置里的 `--dshd-tint`。

## 采样顺序（`controls.rs::theme_bridge_js` 的 read()）

1. `[data-pane="sidebar"]` 的 backgroundColor —— **有侧边栏时一律以它为准**（AGENTS.md 的 UI 配色基准）；
2. `meta[name="theme-color"]`（不透明时才可信）；
3. **没有侧边栏时**（启动页 loading.html、外部页 chat.deepseek.com）：`document.body` → `document.documentElement`
   的非透明 backgroundColor；
4. 常量兜底：按 `data-ds-dark-theme` 判断，暗 `#101624` / 亮 `#e8ecf5`。

上报前会剥掉 alpha（标题栏自己按 `--dshd-tint` 混，带 alpha 会叠乘变淡）；`fg` 由 `bg` 亮度推导（不看页面 color）。

## 历史根因：官方皮肤下「标题栏变黑」（2026-08 修）

`--dsw-alias-*` 只是皮肤（skin.css）变量；**官方默认皮肤下 html/body 背景是透明的**（底色由
`#root` / `[data-dsh-frame]` 等面板绘制）→ 桥若只采 html/body 会全透明 → 落到硬编码回退 `#0b1220`
（深蓝黑）→ 标题栏「变纯黑」、与浅色界面严重不匹配。现靠侧边栏采样 + 按明暗区分的兜底解决。

## 启动页（loading.html）标题栏不跟随（2026-09-20 修）

- **症状**：启动/加载阶段标题栏颜色与启动页底色不一致——亮色系统下标题栏偏灰蓝（`#e8ecf5` vs 页面纯白），
  暗色系统下更刺眼（页面 `#0b1220` 而标题栏被刷成亮色）。
- **根因**：启动页底色跟**系统深浅**（`@media (prefers-color-scheme: dark)` → 亮 `#ffffff` / 暗 `#0b1220`），
  但它**不设 `data-ds-dark-theme`**；而桥对 `src=dsh` 只看这个属性 → `dark=false` → 侧边栏（启动页没有）、
  `meta theme-color`（也没有）全落空 → 常量兜底 `#e8ecf5`（亮色）。**任何系统深浅下都报 `#e8ecf5`。**
- **第二层**：`chrome.html` 的 `--dshd-bg` 默认只有暗色 `#0b1220`，亮色系统下首帧（主题上报到达前）标题栏是黑的。
- **修法**（`controls.rs::theme_bridge_js` + `frontend/chrome.html`）：
  1. 采样顺序加第 3 环（见上）——启动页因此报出真实底色；
  2. `send()` 加闸门：**文档还没 body（且没侧边栏）就不上报**，并在 `DOMContentLoaded` 补报一次。
     否则会先拿常量兜底色刷一帧错的标题栏再纠正（闪烁）；
  3. `chrome.html` 加 `@media (prefers-color-scheme: light){ :root{ --dshd-bg:#ffffff; --dshd-fg:#1d2539 } }`：
     首帧跟随系统、与启动页一致；主题桥上报后用**内联变量**覆盖（内联优先级高于媒体查询）。
- **实测**（无头 Edge + `Emulation.setEmulatedMedia` + `Fetch.enable` 拦 19431）：

  | 场景 | 旧桥上报 | 新桥上报 | 页面实际 body 底色 |
  |---|---|---|---|
  | light | `#e8ecf5` ❌ | `rgb(255, 255, 255)` ✅ | `rgb(255, 255, 255)` |
  | dark | `#e8ecf5` ❌ | `rgb(11, 18, 32)` ✅ | `rgb(11, 18, 32)` |

  `chrome.html` 默认 `--dshd-bg`：light `#ffffff` / dark `#0b1220`（均与启动页一致）。

## 孤立验证套路（改采样逻辑必用，不动真机）

1. 起静态服务指到 `src-tauri/frontend`（如 19997），无头 Edge `--headless=new --remote-debugging-port=9224
   --user-data-dir=%TEMP%\...` 指向它；
2. 每一例用 `Target.createTarget` 开新 target，`Page.addScriptToEvaluateOnNewDocument` 注入桥脚本
   （从 `r##"..."##` 原文提取，`__SRC__` → `dsh` 全局替换），
   `Emulation.setEmulatedMedia({features:[{name:'prefers-color-scheme',value:'light'|'dark'}]})` **在导航前**设置；
3. `Fetch.enable({patterns:[{urlPattern:'*19431*'}]})` 只**记录** `Fetch.requestPaused` 的 URL、**不要**真的放行——
   请求打到真机 19431 会直接改用户当前标题栏的主题槽；
4. 断言：`/set-theme` 上报的 `bg` == 页面 `getComputedStyle(document.body).backgroundColor`。

## 排查注意

- 「刷新后好了、不刷新坏」具有迷惑性：刷新走 boot 页/重新加载的初采样（可能恰好正常），运行时切换才暴露真实采样。
- **要看 `/theme` 接口的最终值**（`http://127.0.0.1:19431/theme`），不要凭肉眼时序下结论。
- 改完采样逻辑**两个系统深浅都要验**（亮/暗各一次），只看当前系统会漏掉一半问题。
