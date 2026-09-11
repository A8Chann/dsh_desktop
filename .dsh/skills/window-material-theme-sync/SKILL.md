---
name: window-material-theme-sync
description: >
  毛玻璃材质（Acrylic/Mica）应用规则与切页主题自适应、标题栏透明度、前景色推导。
whenToUse: >
  毛玻璃 / Acrylic / Mica / windowMaterial / 标题栏透明度 / 切页主题自适应
---

# 标题栏毛玻璃与切页主题自适应

## 毛玻璃只能由 DWM 出

标题栏（chrome）与内容页（dsh/deepseek）是**各自独立的 WebView**。CSS `backdrop-filter` 只能模糊同一 WebView 内的下层内容，**跨 WebView 一律无效**（旧代码写了 blur 但从来没生效）。

真毛玻璃 = 窗口 `.transparent(true)` + `set_effects(...)`，由 DWM 在窗口背后合成。

## Acrylic 与 Mica 的本质区别（选错会被当成 bug）

- `Effect::Acrylic`（DWM backdrop=3）：**实时模糊窗口后面的一切**，包括其它应用窗口——用户说的「透出上面的应用程序」只能靠它。
- `Effect::Mica*`（backdrop=2）：**只采样桌面壁纸**做柔和底纹，下层窗口完全不参与合成；壁纸不变则标题栏颜色恒定。

已做成设置项 `windowMaterial`（acrylic / mica / none，默认 acrylic），`apply_window_effect()` 统一按设置 + 主题亮度应用，改完立即生效无需重启。

## 清除材质必须传 `None`

`set_effects(EffectsBuilder::new().build())` 传的是「空效果列表」，DWM 已设的 backdrop 会**原样保留**（实测切 none 后 backdrop 仍是 2）。

正确写法：`set_effects(None::<tauri::utils::config::WindowEffectsConfig>)`；且因窗口是 `transparent(true)`，清除后必须补 `set_background_color`，否则露底。

## 材质变体与回退

- 按主题亮度选变体：Mica 分 `MicaDark` / `MicaLight`（`0.299R+0.587G+0.114B < 140` 判暗）；Acrylic 无暗亮变体。
- `set_effects` 失败（Win10 等不支持）时回退 `set_background_color`。

## Rust 语法坑

`0.299 * r as f64 + … < 140.0` 会被解析成 `f64<…>` 泛型参数（报 "invalid const generic expression"，一处错误连带 6 个报错）。cast 后比较必须加括号：`0.114 * (b as f64) < 140.0`。

## 标题栏透明度

标题栏 CSS 用 `color-mix(in srgb, var(--dshd-bg) var(--dshd-tint), transparent)` 把主题色叠在材质上——完全透明会太素、不透明则盖掉材质。

浓度由设置面板的「标题栏透明度」滑块控制（`settings.json` 的 `titlebarTint`，0-100，默认 18）；拖动即时改 CSS 变量做预览，保存才落盘，取消则还原上次保存值。

## 切页时开关状态要由 Rust 回推

标题栏滑柄原先只在用户点开关本体时本地翻转，从 ⋯ 菜单/托盘等入口切换时不会动（`deepseek_shown` 是 Rust 侧真相源）。

`toggle-deepseek` 处理完后 `eval` 调 `window.__dshdSwitchState(on)` 同步滑柄与菜单文案。

## 切页主题自适应

- `AppState` 用**两个主题槽**（`theme_dsh` / `theme_deepseek`），主题桥注入时带 `src` 参数上报到对应槽；
- `/set-theme` 只在「上报页 == 当前显示页」时才 `push_theme`（隐藏页的上报只入缓存，不打扰当前标题栏）；
- `toggle-deepseek` 切换后立即用 `current_theme()` 推目标页缓存，无缓存则等桥上报。

## 前景色不能取外部页的 `body` color

chat.deepseek.com 的 body computed color 实测是 `rgb(128,0,128)`（链接紫），套到标题栏会让文字全变紫。

改为**按 bg 亮度推导** fg（暗底 `rgb(230,236,255)` / 亮底 `rgb(20,28,48)`）——标题栏是应用自身 UI，不需要跟页面文字色一致，只需可读且协调。
