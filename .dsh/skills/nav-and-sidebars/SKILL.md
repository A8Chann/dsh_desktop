---
name: nav-and-sidebars
description: >
  改顶部导航栏 / 左右侧边栏底色与面板渲染层级（display:contents 宿主坑），
  以及左侧栏底部区（用量摘要 / 动作区 / 设置整行）的布局规则。
---

# 顶部导航栏 / 右侧侧边栏 / 左侧栏底色与底部区布局规则

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

### 侧边栏行的 hover 浮卡（HoverCard）——「又黑了」= 哈希漂移（2026-09-24 二次踩坑）

`[class*="sessionRow"]` / `projectRow` 悬停会弹一张 244 宽的浮卡（`dsh-client-ui-workspace` 的
`Rows.module.css` 提供内容 + `dsh-client-ui-primitives` 的 `HoverCard.module.css` 提供卡面）。
**它的 `role` 是 `button` 不是 `tooltip`**，所以 `[role="tooltip"]` 那套管不到它。
壳层卡面固定深色：`.card{--dsw-hovercard-bg:#2C2C2E; position:fixed; width:244px; padding:12px 16px;
border-radius:12px; background:var(--dsw-hovercard-bg)}`（是 figma 值、**亮暗同值**，故只能皮肤侧覆盖）。
卡内文字衬深色：`hoverTitle{color:#fff}`、`hoverPath/hoverTime{color:#cfd3d6}`、`hoverStatus{color:#adb2b8}`。

**⚠️ 2026-09-24 用户报「这个框又黑了」**：原来钉的是 0.1.5 的 `[class*="_card_1b2ny"]`，
0.1.7 里同一张卡变成 **`_card_178vx_13`**（**局部名 `card` 不变、只有哈希段换**）→ 卡面规则失效、
外壳深底回来；而卡内三行字的 `[class*="hoverTitle"]` 规则**仍然生效**（深字），
于是「深字压近黑底」= 又黑了。**教训：钉死「完整哈希名」会随构建漂移，`[class*="<局部名>"]` 才抗漂。**

**修法（改用组件写在元素上的行内变量作锚点）**：
```css
div[style*="--dsh-hover-preview-fade"] { --dsw-hovercard-bg: #f3f5fb; background: #f3f5fb; }
body[data-ds-dark-theme] div[style*="--dsh-hover-preview-fade"] { --dsw-hovercard-bg: #1a2238; background: #1a2238; }
[class*="hoverTitle"] { color: #1d2539; }
[class*="hoverPath"], [class*="hoverTime"], [class*="hoverStatus"] { color: #5a6a8c; }
/* 深色：标题 #dbe2f2、路径/时间/状态 #a8b6d4 */
```
- 锚点来自 HoverCard 自己的 JSX：`style={{...pos, "--dsh-hover-preview-fade": `${PREVIEW_FADE_MS}ms`}}`
  → **每张卡都带这个行内变量，与哈希无关**；实测同一时刻全页命中数 = 1（不 hover 时 0）。
- `aria-label`（`"复制: <text>"`）是本地化文案，**不能**当锚点。
- ❌ 也不要用 `[class*="_card_"]`：`card` 这个局部名在别的模块也可能是 `.card`（如 primitives 的 CodeCard），
  子串会撞车（见 `hash-selector-pitfalls`）。
- `hoverPath` 是旧补丁漏掉的一行（工作区/项目行才有）：外壳给 `#cfd3d6`，压浅底基本看不见，务必一起收。

**取值**（冷色浮层家族，= 皮肤的 `--dsw-specific-tip`）：亮 `#f3f5fb` / 暗 `#1a2238`，用实色；
标题 `#1d2539` / `#dbe2f2`，路径·时间·状态 `#5a6a8c` / `#a8b6d4`。实测（F5 后）：
亮 `bg=rgb(243,245,251)`、`--dsw-hovercard-bg=#f3f5fb`；暗 `bg=rgb(26,34,56)`；console error 0。

**要点**：
- 只改底色会把白字变成「白字压浅底」→ **标题/路径/时间/状态四行必须一起改**。
- 这些类的样式**不在静态 CSS 里**，是运行时注入的 `<style>`；靠**特异性**取胜即可
  （皮肤系统给每条规则自动加 `html[data-dsh-skin="blue-fantasy"]` 前缀，高于壳层的单类选择器），
  不必关心注入顺序。
- 找这类问题的通用手法：悬停后遍历 `document.styleSheets`，打印命中元素且带 `background` 的规则并**标出来源**
  （`OURS` / `inline[data-plugin…]` / 哪个包）。
- ⚠️ 判来源时**别只看「亮暗同值」就归因壳层**：hovercard 的 `#2C2C2E` 确实是壳层写死（`HoverCard.module.css` 注释写明 figma 值、亮暗同值），
  但 tooltip 的 `#ffffe1` 是**本皮肤 `skin.css` 自己钉的**。两者现象一样、来源相反 —— 见上面 tooltip 段的归因更正。
- 无头验证时**CDP 的 `mouseMoved` 逼不出这张卡**（组件是 JS 按 enter/delay 建的），
  要对行标题派发 `pointerover/pointerenter/mouseover/mouseenter/mousemove` 才出卡 —— 见 `headless-verification`。

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

## 左侧栏「底部区」：设置在最底，且与 WebUI 图标行**同一行**（2026-09-24）

**症状**：升级 dsh 0.1.7-rc.1 + `@linxin666/dsh-web-all@0.4.1` 后用户反馈「左侧边栏下方的
UI 乱了，设置应该在最下侧独立成行」——实测「设置」被挤成左侧 102px 宽、**纵向居中**卡在侧栏中部，
右边是 cost-meter 额度栈的 417px 窄柱，整块还压到会话列表上。

**用户澄清的「正确形态」**：最底一行 = 设置（左，吃掉剩余宽度）+ WebUI 自己的
「检查更新 / 远程访问」图标行（右，贴身宽）；其余条目各占整行。设置仍在最底部。

**结构**（外壳 `@deepseek-ai/dsh-client-ui-sidebar`，哈希前缀 `hHd-Xa_`）：
```
div.hHd-Xa_root              ← flex column；会话列表 flex:1
  …header / nav / 会话列表…
  div.hHd-Xa_footArea        ← 官方 flex-direction:column
    ├ div.hHd-Xa_footerActions          ← 官方 display:flex
    │   └ div[data-slot="sidebar.footer.action"]   ← 槽位宿主，display:contents（0×0）
    │       ├ cm-footer-stack    （cost-meter 额度栈）
    │       ├ fThDlq_entryRow    （WebUI 的检查更新 / 远程访问）
    │       └ lc-ov-entry        （dsh-context 上下文洞察）
    ├ (无类名 div)             ← dsh-usage「今日用量」摘要，**直系**子元素
    └ div.hHd-Xa_settingsArea
        └ div[data-slot="sidebar.settings"]        ← 也是 display:contents
```
官方 CSS：`.hHd-Xa_footArea{flex-direction:column}`、`.hHd-Xa_settingsArea,.hHd-Xa_footerActions{width:100%}`。

**根因**：web-all 4.x 的 `lib/client.js` 内联了一段 `dsh-web-settings` 的**静态 CSS**（非配置项、无开关可关）：
```css
[data-dsh-frame]:not([data-sidebar-collapsed]) [class*=footArea]{flex-flow:wrap;align-items:center}
…[class*=footArea] > :not([class*=settingsArea]):not([class*=footerActions]){flex:100%}
…[class*=settingsArea]{flex:auto;order:1;width:auto}
…[class*=footerActions]{flex:none;order:2;align-items:center;width:auto}
```
它的**本意**是「最底一行 = 设置 + 我们的图标行」，即把 `footerActions` 当成自己那排小图标。
但 0.1.7 外壳把**所有**插件注入的 footer 条目都收进 `footerActions`（含 339px 高的额度栈、
上下文洞察）→ 「设置 + 图标行」那一行变成「设置 102px + 417px 窄柱并排」，
图标被推到柱子底部、设置被 `align-items:center` 纵向居中 → 就是看到的乱象。

**修法**（已进 `patches.css`）：把 `footerActions` 的**盒子去掉**（`display:contents`），
让它的条目回到 footArea 的换行布局里（正是上游 CSS 假装的结构），再按上游本意排 order：
```css
[data-dsh-frame]:not([data-sidebar-collapsed]) [class*="footArea"]{flex-flow:row wrap;align-items:center}
…[class*="footArea"] > [class*="footerActions"]{display:contents}
…[class*="footArea"] > :not([class*="settingsArea"]):not([class*="footerActions"]){flex:1 1 100%;order:0}   /* 用量摘要 */
…[class*="footArea"] > [class*="footerActions"] > *,
…[class*="footArea"] > [class*="footerActions"] > * > *{flex:1 1 100%;order:1}                             /* 各条目整行 */
…[class*="footArea"] > [class*="footerActions"] > *[class*="entryRow"],
…[class*="footArea"] > [class*="footerActions"] > * > *[class*="entryRow"]{flex:0 0 auto;order:4;width:auto} /* 图标行 */
…[class*="footArea"] > [class*="settingsArea"]{flex:1 1 auto;order:3;width:auto;min-width:0}              /* 设置 */
```
**四个坑**（都实际踩过）：
1. `footerActions` 之下还有一层**槽位宿主** `div[data-slot="sidebar.footer.action"]`，它自己也是
   `display:contents` —— 真条目是它的**孙级**。只写 `> *` 会打到那层 0×0 空宿主上（`.cm-footer-stack` 纹丝不动）。
   所以 `> *` 与 `> * > *` 两级都要写（宿主是 contents 时前者无副作用，宿主若变实体盒也能兜住）。
2. `display:contents` 后 `footerActions` 自身的 `align-items`/`width` 全部失效（没有盒子），
   必须对**它的子项**写规则。
3. `[class*="entryRow"]` 指 WebUI 自己的图标行（`fThDlq_entryRow`），必须限定在
   `footArea > footerActions` 之下，避免误伤别处的行。
4. 那个**无类名的用量摘要**自带 `flex:1 1 100%` —— 横排语境里是「占满整行宽」，
   **不要改成竖排**（竖排时它变成「高度 100%」）；`align-items:center` 也别在竖排下留用，
   否则设置会被纵向居中（就是最初那个「卡在中部」的现象）。
- 必须带 `:not([data-sidebar-collapsed])`：收起成 rail 时官方另有居中布局（`width:auto;justify-content:center`）。
- 顺序规则（order 4 的图标行）写在默认规则（order 1）**之后**，同特异性下靠源码顺序取胜。

**验证**（无头 Edge，数值为主；改完必须 `Page.reload`，皮肤 CSS 只在页面加载时拉一次）：
```
footArea [12,239,256,529] flex-flow=row wrap
  用量摘要        [12,239,256, 98]  order 0
  cm-footer-stack [12,337,256,339]  order 1
  lc-ov-entry     [10,676,260, 42]  order 1
  settingsArea    [12,718,178, 50]  order 3  ← 与下一项同一行
  entryRow        [190,725, 78, 36] order 4  ← 同底行右侧，垂直中心 743 对齐
```
断言：`settings.left + settings.width == entryRow.left`、两者底边同为 footArea 底边。

**无头截图辅助**：桌宠 Live2D 的浮层是 `div.v78Lda_float`（`position:fixed; z-index:2147483000`），
底部截图时它正好压住设置行；临时注入 `.v78Lda_float{visibility:hidden!important}` 再截，截完删掉该 `<style>`。
（它的类名里**不含** "pet"，用 `[class*=pet]` 隐藏无效。）
