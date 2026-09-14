---
name: hash-selector-pitfalls
description: >
  写或排查 [class*="哈希"] 选择器时读：同前缀异模块、子串误伤、多层背景叠加。
---

# CSS-module 哈希选择器踩坑与修法

跨构建的 CSS-module 哈希（如 `uV2eYG_row`、`lats3W_row`、`o3BgMG_root`）**不稳定且易撞车**。以下是已踩过的坑与通用修法。

## 通用原则

写 `[class*="<hash>"]` 前，先问：**这个子串还会命中谁？** 使用前先数一遍该选择器实际命中几个元素、分别在哪个区域（`[data-composer-card]` / `[data-slot="conversation.session"]` / `[data-pane="sidebar"]` / 设置面板 …）。

## 已知踩坑类型

### 1. 同前缀异模块误伤
`[class*="<hash>_row"]` 匹配「正文行」时，`uV2eYG_` 是 **composer（输入框）模块**前缀，`uV2eYG_row` 是输入框下方那排按钮（+ / 附件 / 权限 / 模型 / 发送）容器，**不是正文行**。
给它加 `background + backdrop-filter + width:fit-content + margin:auto` → 该行塌缩到 **16px 宽**，按钮全部溢出白块。

**修法**：排除 composer 后代，正文行规则不动：
```css
[class*="uV2eYG_row"]:not([data-composer-card] *):not([data-composer-seat] *)
```
深色版同步修改。

### 2. 撞上「设置面板」的行
裸写的 `[class*="lats3W_row"]` 在另一构建里是**设置面板**「对话显示 / 紧凑」那一行的哈希 → 行宽从 **564px 塌成 282px**，描述文字被白块盖住。

**定位手法（可复用）**：无头 Edge 打开 设置 → 通用设置，找到目标行叶子元素，向上 5 层 dump `tag/class/rect/computed(background/padding/border-radius/width/box-shadow)`，**与邻近正常行逐项对比**（本次差异：282 vs 564、白底 vs 透明、radius 8 vs 0）；再对候选 `[class*="哈希"]` 做全文档命中统计并按区域归类。

**两条修法（都已落地）**：
1. 删掉该哈希条目；
2. 整块加作用域 `[data-slot="main.conversation"]` —— 实测**设置面板在它之外、会话正文/输入区（含 dock）都在它之内**。

回归验证：设置行恢复 564px / 透明 / radius 0（与同面板各行宽度数组 `[564×6]` 一致）；
会话侧需同时确认未回归 —— `callRow` 仍 `rgba(255,255,255,.5)+blur(10px) saturate(1.3)`，
`markdown>p` / `userStack bubble` / `[data-turn-tail] actions` / 配件胶囊 / 统计行 / `cm-root` 全部照旧。

### 3. 子串误伤（`X` 命中 `XSomething`）
计时 chip 类名 `EvIC1a_turnStatusClock` **包含** `turnStatus` 子串 → 连它一起吃到了 `[class*="turnStatus"]::before` 的白底，计时段变两层。

**修法**：
```css
[class*="turnStatus"]:not([class*="turnStatusClock"])
```
浅色 + 深色两条都要改。

### 4. 同一行多层背景自叠
`Pwsh · …` / `工具调用 · …` 这类行是**三层嵌套、尺寸几乎重合**：
- `[class*="callRow"]`（最外层 wrapper，含 1px 8px 内边距）
- `[class*="o3BgMG_root"]`（工具行本体）
- `[data-disclosure-row]`（可展开标题行）

「补漏」批次给三层都加了 `background + backdrop-filter + border-radius`，半透明底叠 3 次 → 越叠越白 → 用户看到「两层背景」。

**修法**：保留最外层 `callRow`，重置其内层：
```css
[data-slot="conversation.session"] [class*="callRow"] [class*="o3BgMG_root"],
[data-slot="conversation.session"] [class*="callRow"] [class*="o3BgMG_summary"],
[data-slot="conversation.session"] [class*="callRow"] [data-disclosure-row] {
  background: none;
  backdrop-filter: none;
  border-radius: 0;
  box-shadow: none;
  padding: 0;
  margin: 0;
  width: auto;
  max-width: none;
}
```
**必须再写一遍 `body[data-ds-dark-theme]` 前缀的同款选择器**，否则深色那条特异性更高、重置不掉。

**验证**：沿祖先链数 `backgroundColor !== 'rgba(0,0,0,0)'` 的元素个数，修复前 3 → 修复后 1。

### 5. 同模块的 `_root` / `_label` / `_chevron` 三兄弟一起中招
回合过程行「N 次工具调用 · M 条消息」原先挂在 `[class*="l_V-RG_"]` 上，而 `l_V-RG_` 是
**同一模块的三个类名共用前缀**：

| 元素 | 尺寸 | 后果 |
|---|---|---|
| `BUTTON.l_V-RG_root` | 117×32 | 本该只有它有底色 |
| `SPAN.l_V-RG_label` | 81×24 | 也吃白底 + blur → **内层那个小圆角白框** |
| `SVG.l_V-RG_chevron` | 16×16 | 图标底下也被刷一层 |

三层 `.5` 白相叠 → 内圈更白 → 肉眼就是「两层背景」（用户 2026-09-10 反馈）。

**与第 4 条的区别**：第 4 条是**不同模块多层嵌套**（callRow 套 o3BgMG 套 disclosure-row），
本条是**同一模块内** `_root` / `_label` / `_chevron` 共享前缀 —— 只要写了 `[class*="<前缀>_"]`
就会一次命中整族。故 `[class*="X_"]` 这种**带下划线的通配**尤其危险：它等价于
「X 模块的所有槽位」。

**修法（优先语义属性，别再补 `:not()`）**：该 button 上有稳定的
`data-turn-process`（值 = turn 序号，来自 `TurnProcessNodeView`），直接改挂它：
```css
[data-slot="main.conversation"] [data-turn-process] { /* 底色/模糊/圆角 */ }
body[data-ds-dark-theme] [data-slot="main.conversation"] [data-turn-process] { /* 深色底 */ }
```
属性选择器只精确匹配属性名，不会误命中同族的 `data-turn-process-member` /
`-hidden` / `-answer` / `-inline`。

**验证**：`bgLayers` 3 → 1，且行内后代 `descendantsWithBg` = 0。

**顺带**：上游 `.l_V-RG_root` 是 `height:33px; padding:0 0 8px; border-bottom:.5px solid`
（给「过程↔正文」做分隔的分隔行）。套毛玻璃胶囊后底边会多 8px 空白 + 一道压在圆角上的杂线，
故一并收成 `height:auto; padding:1px 8px; border-bottom:none`（与 `callRow` 对齐），
分隔线只在**展开态** `[data-turn-process][data-open]` 还给「过程↔正文」。

## 6. 「看着像 `<pre>` 其实不是」：围栏代码块
`[class*="markdown"] > pre` 这类**元素名**规则会漏掉围栏代码块 —— 壳层把它渲染成
**`<div class="_block_rsn9u_4 md-code-block">`**（`md-code-block` 是**稳定类名**，非哈希）：

```
_markdown_kcgor_5              ← markdown 容器
  └ DIV.md-code-block          bg=rgba(243,245,251,.58) br=12px  ← 壳层自带底，但无 backdrop-filter
      ├ DIV._banner_rsn9u_24
      └ PRE._plain_rsn9u_103
```

用户 2026-09-11 反馈「`class="_block_rsn9u_4 md-code-block"` 这个怎么没有背景模糊」——
根因就是上一版 PR 的 markdown `> pre` 规则从未命中它。修法：另写一条
`[class*="markdown"] [class*="md-code-block"] { backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3) }`，
**只补模糊、别覆盖壳层自带的填充色**（它是 `.58` 的 `rgb(243,245,251)`）。

⚠️ 调试时注意：这类元素只在**当前会话真的有代码块**时才存在 —— 用无头脚本遍历侧边栏会话
（`[data-pane="sidebar"]` 里的会话行逐个点）找 `document.querySelectorAll('[class*="md-code-block"]').length > 0` 的那个。

## 7. 表格：托底层撑满整栏 vs 表格贴内容宽度
壳层有两套表格形状（都在官方的 `_tableScroll_*` / `_tableFill_*` 模块里）：
```css
._tableScroll table { width: max-content; max-width: max-content; }  /* 贴内容 */
._tableFill    table { width: 100%;      max-width: none; }          /* 撑满整栏 */
```
`class="_tableScroll_kcgor_190 _tableFill_kcgor_236"` 这种**同时带两个类**的表会走「撑满」。
而我们的托底写在**外层容器**上，容器是 block → 也撑满 → 表格只占 528px 时，
旁边空出 **约 300px 的空玻璃**（用户 2026-09-11：「都超出了我规定拖拽可控的宽度」）。

**修法（两条一起）**：
```css
[class*="markdown"] [class*="tableFill"] table { width: max-content; max-width: 100%; }  /* 表格贴内容 */
[class*="markdown"] [class*="tableScroll"] { width: fit-content; max-width: 100%; margin-inline: auto; }  /* 托底跟着收 */
```
实测 托底 829 → 548px（表格 528 + 内边距）；宽表仍由容器自身 `overflow-x:auto` 横向滚动。

**「内容列」的宽度上限**由 `--dsh-chat-content-width` 决定：
`.wSkVaW_root { --dsh-chat-content-width: var(--dsh-chat-user-width, clamp(680px, calc(var(--dsh-conversation-column-width,0px) * .64), 920px)) }`，
落在 `.EvIC1a_column { max-width: var(--dsh-chat-content-width); width:100%; margin:0 auto }`（本机实测 829.44px）。
判断「有没有超宽」就以这个 column 的左右边界为基准量。

## 8. 别给 composer seat 加 `backdrop-filter` —— 被 skin-center 的 neutralizer 清掉
`[data-composer-seat]` 上写 `backdrop-filter` 是**死代码**。skin-center 注入的
`<style data-dsh-scene-neutralizer>` 里有：

```css
html[data-dsh-backdrop-active] [data-composer-seat],
html[data-dsh-backdrop-active] [data-composer-seat]::before {
  background: none !important; backdrop-filter: none !important;
}
html[data-dsh-backdrop-active][data-dsh-conversation-content] [data-composer-card] {
  backdrop-filter: blur(var(--dsh-input-card-blur, 10px)) !important;
}
```

即**它有意**把停靠区的模糊收归到「输入卡」一处（由 `--dsh-input-card-blur` 控制），
避免两层各自模糊。我们那条规则特异性一样但更靠前 → 被 `!important` 直接清成 `none`
（实测 `computed backdrop-filter = none`）。**要改输入区模糊，应该动 `--dsh-input-card-blur`
或 `[data-composer-card]`，不是 `[data-composer-seat]`。**
排查手法：遍历 `document.styleSheets`，打印所有 `el.matches(selectorText)` 且带
`backdrop-filter` 的规则 —— 一眼就能看出是谁用 `!important` 赢的。

## 9. ⭐ `backdrop-filter` 会创建 `position: fixed` 后代的包含块（tooltip 跑飞）
**这是本仓库最隐蔽的一类坑**（2026-09-14 用户报「hover 后的提示气泡歪了」）。

官方把 tooltip 直接渲染在**按钮所在行内部**：

```
DIV.xzv4MW_actions                      ← 操作行（复制/点赞/点踩/重新生成）
  └ SPAN._bubble_1nw3t_1「复制」  position: fixed   ← tooltip 就在行内！
```

给这一行加 `backdrop-filter` 之后，该行就会成为 **`position: fixed` 后代的包含块**
（CSS 规范：`transform` / `perspective` / `filter` / `backdrop-filter` /
`contain: paint|layout|strict` / `will-change: transform|filter` 都会）。
于是 tooltip 不再以**视口**定位，而是以这一行（几十像素的小盒子）为基准 →
实测从按钮旁 `1310,336` 直接跳到 `1524,748`，跑到屏幕另一头。

**修法：托底交给 `::before` 画，元素本身不要沾这些属性。**

```css
/* ✅ 行自身保持干净，只保留不影响定位的 width */
[data-turn-tail] [class*="actions"], ... {
  position: relative;      /* 仅供 ::before 定位；position:relative 不影响 fixed */
  padding: 0 8px;          /* 上游原值，原样保留 —— 见下方「只改必要的那一处」 */
  width: fit-content;
}
[data-turn-tail] [class*="actions"]::before, ... {
  content: ""; position: absolute; inset: 0; z-index: -1;
  border-radius: 10px;
  background: rgb(242 245 250 / calc(var(--dsh-skin-bubble-alpha, .5) * 1));
  backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
}
```

### ⚠️ 只改必要的那一处，别顺手"修正"icon 偏移
上面那条 `padding: 0 8px` 确实会让图标偏移 8px（该行在官方布局里
「助手尾部左对齐满宽 / 用户气泡右对齐贴边」，左对齐的右移 8px、右对齐的左移 8px；
实测助手行首图标 502→510、用户行 1310→1302）。

**但这是上游既有行为，不是 bug。** 2026-09-14 我顺手把 padding 去掉（以为在修"歪"），
用户立刻要求还原：**「我只要求改了 hover 弹窗的 bug 和样式」** ——
去掉 padding 让行盒窄了 16px（助手 475→459、用户 95→79），行宽和图标一起变了。

**教训**：诊断出 A（tooltip 跑飞，真 bug）和 B（图标偏移，既有设计）两件事时，
**只修用户报的 A**；把 B 顺手"修掉"= 未经授权的可见变更。
真要修 B 应当先说明、再动手。改回后台账：
`padding: 0 8px` + `width: fit-content`（上游原值）+ `::before { inset: 0 }`（贴合行盒），
行宽/图标与改动前逐像素一致，tooltip 依旧正位。

**排查手法（可复用）**：遍历我们所有带 `backdrop-filter` 的规则，对每个命中元素
查 `[...e.querySelectorAll('*')].filter(d => getComputedStyle(d).position === 'fixed')`
—— 命中即有问题。验证 tooltip 是否正位，看它的**包含块是否为视口**
（上溯第一个有 `transform/filter/backdrop-filter/contain` 的祖先应为 `null`）。

⚠️ 探 tooltip 时注意**排除宠物对话气泡**（`.kz2Bea_*`，也是 `fixed`），否则会误判；
且要先确认按钮 `getBoundingClientRect().top > 0`（滚出视口的负数坐标 hover 不到）。

## 10. ⭐ 右侧栏面板「颜色像叠了两次」：`panel` 与 `panelBody` 是两层
用户 2026-09-14 反馈「右侧面板的颜色像是叠加了两次」。实测：

```
P3OORG_panel       710x874  bg rgba(243,245,251,0.75)   ← 被 [class*="panel"] 命中
  └ P3OORG_panelBody 709x874  bg rgba(243,245,251,0.75)  ← 也被命中（嵌套！）
```
`[class*="panel"]` 是**子串**匹配，外层 `panel` 与内层 `panelBody` 尺寸几乎重合，
各上一层 0.75 → 等效 `1-(0.25)² = **0.94**`，比 token 原值实得多。
官方 CSS 只给外层上色，多出来那层是我们加的。

**修法：只作用于「最外层命中元素」** —— 用 `:not()` 里放一个后代选择器排除被嵌套的：
```css
[class*="rightbarCol"] [class*="panel"]:not([class*="panel"] [class*="panel"]) {
  background: var(--dsw-alias-bg-layer-1);
}
```
`:not(A A)` 的语义 = 「自身不是被另一个 A 包着的 A」→ 稳定只选最外层，**不依赖哈希名**，
比 `:not([class*="panelBody"])` 这类点名排除更抗改名。

**通用写法**：凡是给某个「可能嵌套同名元素」的类上底/上模糊，都先数一下
`document.querySelectorAll(sel)` 的**可见命中数**与它们的**嵌套关系**；
> 1 且相互嵌套时，用 `:not(sel sel)` 收敛到最外层。
实测判据：改完可见命中里**只有最外层有底色**，等效 alpha 回到 token 原值。


## 通用教训

给 `[class*="…"]` 加视觉属性前，先看这条选择器会不会同时命中**同一子树里的多层**；要「只留一层」就得连内层一起重置。跨构建哈希选择器优先加作用域或改用语义属性。

**首选顺序**：语义属性（`data-turn-process` / `data-slot` / `data-dsh-*`）> 哈希 + `[data-slot="main.conversation"]` 作用域 > 哈希 + `:not()` 排除。
本机多个坑（turnStatus、callRow、`l_V-RG_`）的共同点都是**当初图省事写了 `[class*="X_"]`**，
事后靠 `:not()` 补漏；有语义属性时应当直接换掉，`:not()` 只是没有语义属性时的退路。
