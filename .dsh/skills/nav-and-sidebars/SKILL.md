---
name: nav-and-sidebars
description: >
  改顶部导航栏 / 左右侧边栏底色与面板渲染层级（display:contents 宿主坑）。
---

# 顶部导航栏 / 右侧侧边栏 / 左侧栏底色规则

## 左侧栏底色（基准色）

颜色直接取壳层 token **`--dsw-specific-sidebar-fill`** —— 左侧导航栏用的就是它，且**自带主题切换**：
```
亮  rgba(242, 245, 250, calc(1 - <背景遮挡> * .5))     ← 通道 242/245/250
暗  rgba(29,  37,  57,  calc(1 - <背景遮挡> * .45))    ← 通道 29/37/57
```

```css
[data-slot="main.conversation"] header,
[data-slot="rightbar.session"] > *,
[class*="rightbarCol"] [class*="panel"] {
  background: rgb(from var(--dsw-specific-sidebar-fill) r g b / calc(0.95 - var(--dsw-skin-scrim, .5) * 0.5));
}
```
- **颜色**用相对颜色语法取 token 的 `r g b` → 清爽地跟着主题走，**一条规则覆盖亮/暗**，不用再写 `body[data-ds-dark-theme]` 变体。
- **alpha** 自己给：**线性联动「背景遮挡」`--dsw-skin-scrim`，0.95 → 0.45**（遮挡 0 → 0.95，遮挡 1 → 0.45）。2026-09-14 用户定。
- 顶部另有 1px 分隔线（亮 `rgba(28,37,70,.08)` / 暗 `border-bottom-color: rgba(160,180,230,.12)`）。

### ⚠️ alpha 跟「背景遮挡」，**不跟**「气泡不透明度」
2026-09-11 用户明确：「气泡不透明度不应该联动顶部导航和右侧边栏」。外框是框架不是气泡。
- 曾经写成 `rgb(242 245 250 / calc(var(--dsh-skin-bubble-alpha, .5) * 1.5))`，被否掉。
- 也曾短暂写成**固定** `.75`；2026-09-14 用户改要求：**跟「背景遮挡」**（0.95→0.45）。
- 验证方法：把 `--dsh-skin-bubble-alpha` 设成 `0 / .9` → 导航栏与右侧栏**不变**；
  把 `--dsw-skin-scrim` 设成 `0 / .5 / 1` → 应得 `.95 / .7 / .45`。

### ⚠️ 删规则时别留**孤立逗号**
2026-09-14 我删掉旧的深色 `background` 规则时留下一个只有 `,` 的行，把它后面那条
`body[data-ds-dark-theme] … header { border-bottom-color: … }` 的选择器列表变成非法 →
**整条规则被浏览器静默丢弃**（暗色分隔线失效），而 `{`/`}` 计数、括号配平全都看不出来。
- 删规则后务必扫一遍 `^\s*,\s*$` 与 `^\}\s*,\s*$`；
- 更稳的是**验证关键声明是否真的生效**（读 computed），别只看语法。

## 右侧侧边栏面板（`P3OORG_panel`）—— 与顶部导航栏**分开处理**
2026-09-14 用户澄清：顶部导航栏与右侧栏**不是一回事**。

- **底色用 `--dsw-alias-bg-layer-1`**：官方 `.P3OORG_panel` 写的是 `var(--dsw-alias-bg-base)`，
  但那个太透（实测 `rgba(255,255,255, calc(0.5 * .45))` ≈ **0.22**，插画明显透出来）。
  2026-09-14 用户先要求 bg-base、随后更正为 **`--dsw-alias-bg-layer-1`**（标准面板层色，
  亮 `rgba(243,245,251, calc(1 - <遮挡> * .5))`、暗 `rgba(26,34,56, calc(1 - <遮挡> * .45))`，
  默认 50% 遮挡时 ≈ .75 / .776，自带主题与遮挡联动，不用写变体）。
  教训：**先问清是哪个 token，别自己二选一** —— 我按 bg-base 改完又被更正了一次。
- **分割线与左栏对齐**：官方给的是 `border-left: .5px solid var(--dsw-alias-border-l4)`，
  而左侧栏是 `border-right: .5px solid var(--dsw-alias-border-l3)` —— **l4 比 l3 深一档**
  （实测 α 0.42 vs 0.32），两侧看着不一致。改成 `border-left-color: var(--dsw-alias-border-l3)`。
- 右侧栏真面板是 `.panel`，两层 slot 宿主（`rightbar` / `rightbar.session`）都是 `display:contents`，
  写它们等于没写。

## 底部面板展开时右侧竖线「断层」—— 对称补一条右边框
**现象**：底部面板（终端等）展开后，右侧那根竖分割线在 y = 底部面板顶 以下消失了。
**根因**（实测）：底部面板范围为 `x 280..867`，而右侧栏面板左边框在 `x 866` ——
右边缘比它**多 1px**，正好把那 1px 竖线盖住。左侧栏没这问题，因为底部面板从它右边（280）开始。
**修法**（对齐左栏做法，不碰任何宽度）：
```css
[class*="bottomPanel"] {
  box-sizing: border-box;                                 /* 边框画进既有宽度内，零位移 */
  border-right: 1px solid var(--dsw-alias-border-l3);     /* 与左栏/右栏同色 */
}
```
⚠️ 关闭右侧栏时底部面板会延伸到窗口右缘，这条边框就落在窗口边缘（几乎不可见，可接受）。
不要为此去改底部面板的宽度 —— 那会像 2026-09-14 那次一样把行宽也改了。

### hover 提示气泡（`[role="tooltip"]`）同属「外框」，也固定浓度
壳层规则：`._bubble_1nw3t_1 { background: var(--dsw-alias-tooltip-bg); color: var(--dsw-static-neutral-bluish-00) }`。
- ⚠️ `--dsw-alias-tooltip-bg` 在**上游是分主题的**（design-platform 亮/暗各一档）。
  **把它钉成亮暗同值 `#ffffe1` 的是本皮肤自己的 `skin.css`**（亮 L87-88 / 暗 L178-179，连 `--dsw-alias-tooltip-fg: #1a2238` 一并钉死），
  自 v1 皮肤移植期就在，属于**本皮肤的既有配色取舍**。
  > 📌 **归因更正（2026-09-14，维护者 @Aa728848 在 issue #1574 里指出）**：
  > 我原先写的「壳层不做主题区分」**是错的** —— 壳层主题化了，是皮肤自己钉的同值。
  > 教训：**看到「亮暗同值」先搜自己皮肤的 `skin.css` 有没有定义该 token，别急着归因壳层。**
  想彻底修应该改 `skin.css` 的 token（正确层），而不是在 `patches.css` 里覆盖 `[role="tooltip"]`（治标）。
- 皮肤基线区（L30 附近）只把文字色换成了 `--dsw-alias-tooltip-fg`。
- ✅ **2026-09-15 定稿：改在 token 层（`skin.css`），`patches.css` 里不再覆盖 `[role="tooltip"]`**：
  ```
  skin.css 亮  --dsw-alias-tooltip-bg: #f3f5fb;  --dsw-alias-tooltip-fg: #1a2238;
  skin.css 暗  --dsw-alias-tooltip-bg: #1a2238;  --dsw-alias-tooltip-fg: #dbe2f2;
  ```
  取的是皮肤的 `--dsw-specific-tip` 家族（浮层色）。壳层那条
  `[role="tooltip"] { background: var(--dsw-alias-tooltip-bg); color: var(--dsw-alias-tooltip-fg) }` 直接就对了，
  **自动跟随主题**，不需要发丝边 / blur / 深色变体。
  实测：亮 `rgb(243,245,251)`/`rgb(26,34,56)`、暗 `rgb(26,34,56)`/`rgb(219,226,242)`。
  - 为什么不留在 `patches.css`：那是治标。真正的开关就是本皮肤的 token，
    改 token 一行搞定、且**下一个人不会再误判成「壳层的锅」**。
  - ⚠️ **`skin.css` 也是上游文件**，市场更新会整文件覆盖 → 改过之后必须归档
    （`assets/skins/blue-fantasy/skin.css`；`scripts/skin-patches.ps1` 只管 `patches.css`，要手动带这一份）。
- 改底色时**不要**顺手给 `[role="tooltip"]` 的**祖先**加 `backdrop-filter` —— 见 `hash-selector-pitfalls` 第 9 条（会劫持 fixed tooltip 的定位）。加在 tooltip 自身是安全的（它是叶子节点）。

### 侧边栏行的 hover 浮层（hovercard）是另一处「壳层写死深色」
`[class*="sessionRow"]` / `projectRow` 悬停会弹一张 244×96 的浮卡，**它的 role 是 `button` 不是 `tooltip`**，
所以 `[role="tooltip"]` 规则管不到它。壳层定义：`._card_1b2ny_13 { --dsw-hovercard-bg: #2C2C2E; background: var(--dsw-hovercard-bg); ... }`
—— **token 直接写在元素自身上**，且是「始终深色」设计（内部配白字 `#fff` / 灰字 `#cfd3d6`、`#adb2b8`）。
2026-09-14 用户反馈后改为冷色浮层：亮 `rgba(242,245,250,.96)` / 暗 `rgba(16,22,42,.96)`，
标题 `#1d2539` / `#dbe2f2`，时间与状态 `#5a6a8c` / `#a8b6d4`。

**要点**：
- 只改底色会把白字变成「白字压浅底」→ **标题/时间/状态三行必须一起改**（用 `[class*="hoverTitle"]` 等）。
- 这些类的样式**不在静态 CSS 里**，是插件运行时注入的 `<style>`；靠**特异性**取胜即可
  （皮肤系统给每条规则自动加 `html[data-dsh-skin="blue-fantasy"]` 前缀，高于壳层的单类选择器），
  不必关心注入顺序。
- 找这类问题的通用手法：悬停后遍历 `document.styleSheets`，打印命中元素且带 `background` 的规则并**标出来源**
  （`OURS` / `inline[data-plugin…]` / 哪个包）。
- ⚠️ 判来源时**别只看「亮暗同值」就归因壳层**：hovercard 的 `#2C2C2E` 确实是壳层写死（`HoverCard.module.css` 注释写明 figma 值、亮暗同值），
  但 tooltip 的 `#ffffe1` 是**本皮肤 `skin.css` 自己钉的**。两者现象一样、来源相反 —— 见上面 tooltip 段的归因更正。

## 导航栏内部各项：**不加任何背景**

（会话名 / 模式选择 / 对话 / 轨迹 / 上下文）—— 最终定稿为「不加任何背景」。

历程（三轮反转，记下来免得再绕）：
1. 按要求加贴合内容宽度的模糊底；
2. 用户反馈「文字底下的模糊很怪」→ 去掉 `backdrop-filter` 只留底色；
3. 用户明确「那几个字不要背景了」→ **整条规则删除**（背景 / 圆角 / 内边距 / 宽度都不加），恢复官方原始间距与下划线指示器。

实测：会话名 102x28、模式 71x22、tab 26/26/39，全部 `bg=rgba(0,0,0,0)`、`bf=none`、`padding 0`。

若以后又要小底，选择器用语义属性：
- 会话名 = `header nav[aria-label="会话层级"]`
- tab = `header [role="tablist"] button[role="tab"]`
- 模式选择没有稳定钩子，只能用 `[class*="headerActions"]`

## 右侧侧边栏

**是官方的 `@deepseek-ai/dsh-client-ui-sidebar-right`**（`rightbarCol` / `rightbar.session` / `P3OORG_panel` 都在它里面）。`dsh-better-sidebar` 只是通过 `dsh.client.inject` 往官方右侧栏里塞「文件/终端/浏览器」内容，**右侧栏本身不是它的**。

### 渲染层级

```
[class*="rightbarCol"]      （列）
  └ [data-slot="rightbar"]        （display:contents，0×0）
      └ [data-slot="rightbar.session"]  （同样 0×0）
          └ [class*="panel"]             （真正画出的面板，position:absolute）
```

**给前两个宿主上底等于没上**（第一版踩在这，用户反馈「侧边栏背景没生效」）。必须写到 `.panel` 那一层。

### 判断面板属于谁

直接在 `node_modules` 里：
```powershell
Select-String -Pattern "<哈希前缀>|<slot名>"
```
本次 `P3OORG_panel` / `rightbar.session` 只命中官方包，better-sidebar 里一次都没有。
