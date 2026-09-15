---
name: composer-dock-styling
description: >
  输入栏下方三行（Go 5h / 统计行 / 本会话）与 composer 停靠区的样式规则：外壳 accessory 白条、宽度收缩、cm-qchip hover 对齐
whenToUse: >
  改输入栏下方三行或 dock 内元素样式时
---

# 输入栏下方三行样式（2026-09-10 定稿）

## ⚠️ 最重要的一条：**上游已修 ≠ 本机已生效**（2026-09-15 踩坑）
上游 `9407d83` 把**队列 dock 两层底**的根因修在**壳层适配器**
（`@linxin666/dsh-client-ui-skin-center` 的 `src/client/runtime/shell-rendering.ts`）。
那是**npm 包里的代码，不是皮肤文件** —— 本机装的是 `0.3.22`（早于该提交），
所以壳层那层底**照样在画**（实测 dock 仍是不透明 `rgb(243,245,251)`）。

我按「上游已修 → 皮肤侧兜底已冗余」把它删了，结果**当场回归**「两层底 + 两侧各宽 8px」。

**判断准则：**
| 上游修在哪 | 本机怎么生效 |
| --- | --- |
| 皮肤文件（`skins/blue-fantasy/*.css`） | 换本地文件即可，**立即**生效 |
| 壳层 / 适配器 / 插件源码（`src/**`、`packages/**`） | **必须升级 npm 包 + 重启**，只换皮肤文件无效 |

**所以**：皮肤侧兜底规则在「包升级前」不能删。删之前先确认修复落在哪一层、
并实测一次（本文那条 dock 规则现在就是这么保留的，注释里写明了什么时候能删）。

## ⭐ 变量归属：输入区配件 ≠ 气泡（2026-09-14 用户明确）
**这一片（`bOPqQW_root` / `[data-composer-stats]`、`cm-root`、`cm-qchip`）是「输入区配件」，
不是「气泡」**，所以它读的是另外一套：

| 维度 | 配件区（本节这几个） | 正文托底（可读性层） |
| --- | --- | --- |
| 模糊 | `--dsh-input-card-blur`（**输入卡模糊**） | `--dsh-skin-bubble-blur`（气泡模糊程度） |
| 底色 | **皮肤基色 + 借 `--dsw-specific-input-major` 的 alpha** | `--dsh-skin-bubble-alpha`（气泡不透明度） |

### 🔑 底色写法：只借 token 的 **alpha**，颜色仍用皮肤基色
用户 2026-09-14 明确：「**我不是说颜色要与 `--dsw-specific-input-major` 一样，我只是说 alpha 值一样**」。

```css
[class*="cm-root"], [class*="-NDN2W_root"], [class*="cm-qchip"], /* + dock 内两条 */ {
  background: rgb(from var(--dsw-specific-input-major) 242 245 250 / alpha);
}
body[data-ds-dark-theme] /* 同上选择器 */ {
  background: rgb(from var(--dsw-specific-input-major) 16 22 42 / alpha);
}
```
- **相对颜色语法** `rgb(from C <r> <g> <b> / alpha)`：字面通道给皮肤基色，
  `alpha` 关键字取源色的 alpha。**WebView2 上已实测可用**（computed 正常解析）。
- 换颜色只需改那三个数字；**改 alpha 逻辑不用动本皮肤** —— 源 token 一改就自动跟随。

### 🔑 `--dsw-specific-input-major` 的 alpha 由「背景遮挡」驱动
```
亮  rgba(255, 255, 255, calc(1 - <背景遮挡> * .4))
暗  rgba(26,  34,  56,  calc(1 - <背景遮挡> * .35))
```
所以「配件联动背景遮挡」= **借它的 alpha 即可**，不必自己乘系数或抄公式。
⚠️ 2026-09-14 两次踩坑记录：
1. 先写成 `color-mix(… calc(var(--dsw-skin-scrim) * 200%) …)` → 等于把遮挡算了两遍
   （默认值碰巧一致，一拖动就分叉：scrim=0.2 时我给 .368、输入卡是 .92）；
2. 再写成 `background: var(--dsw-specific-input-major)` → 那连**颜色**也变成输入卡的白了，
   而用户只要 alpha。

实测（改 `--dsw-skin-scrim`）：三行 = `rgb(242,245,250)` 固定 + alpha `0.8 / 0.9216 / 0.6` 跟随；
暗色 = `rgb(16,22,42)` + alpha `0.8235`。深色变体必须保留（颜色是我们自己的，不会自动切）。

⚠️ 别再顺手把这些行改回气泡变量，也别给它们自己写 scrim 的 calc。

## 底的全部来源
`Go 5h`(cm-qstrip) / 统计行(`[data-composer-stats]`) / 本会话(`cm-root`) 的底**全部来自皮肤「外壳渲染层」的 accessory 规则**：
```
… [data-slot="conversation.input.dock"] > * { background: var(--dsh-composer-accessory-bg, var(--dsw-specific-tip,…)) !important; backdrop-filter: … !important }
```
实测解析成 **不透明的 `rgb(243,245,251)` + `blur(10px)`**（补丁自己写的那份 `.5` 白底早被这条 `!important` 压掉了，改它是没用的）。

## 对话队列（排队消息）dock —— 同样要清掉外壳那层底（2026-09-14 新增）
用户反馈「对话队列的样式背景似乎有两层，两侧明显有一点超出的元素」。

队列组件在官方 `@deepseek-ai/dsh-client-ui-conversation`：`QueueDock`，类名 `_7yHdaG_*`，
根节点带 **`data-queue-dock`**，内部是 `._7yHdaG_panel`（自带 `background: var(--dsw-specific-tip)`、
`border-radius:12px 12px 0 0`、`padding:2px 0`、`::after` 0.5px 描边）。
根 `._7yHdaG_dock` 自带 `padding: 0 var(--dsh-composer-dock-inset)`（=8px），**panel 在它内侧**。

**两层 + 两侧超出的成因**：上面那条 `… > *` 的 accessory 规则把队列 dock 也当成 accessory 行刷了底，
于是「外壳一层（画在 dock 上，含 8px padding）+ 官方 panel 一层」；
dock 比 panel **两边各宽 8px** → 外壳那层就两侧各突出来 8px。

**做法**（照壳层自己的先例 —— 它已经用 `[data-goal-bar="true"]` 重复三遍提特异性来豁免同类元素）：
```css
[data-phase="active"] [data-slot="conversation.input.dock"] > [data-queue-dock],
[data-phase="active"] [data-slot="conversation.composer.dock"] > [data-queue-dock] {
  background: transparent !important;
  -webkit-backdrop-filter: none !important;
  backdrop-filter: none !important;
  box-shadow: none !important;
  border-radius: 0 !important;
}
```
**只清视觉属性，不动 padding / width / margin** —— 那会像上一轮那样把行宽也改了。
实测：dock `bg=transparent / bf=none / radius=0 / shadow=none`，panel 保留官方底
（亮 `rgb(243,245,251)`、暗 `rgb(26,34,56)`），可见层数 2 → 1。

> 验证技巧：队列只在「会话忙时发消息」才出现，无头页复现不了 → **手工合成同构 DOM** 即可验证层叠：
> 往 `[data-slot="conversation.input.dock"]` 插一个
> `<div data-queue-dock class="_7yHdaG_dock"><div class="_7yHdaG_panel">…</div></div>`，
> 官方 CSS 是按类名/属性匹配的，合成元素一样吃得到，量 `dock` 与 `panel` 的 `background` 与左右边界差即可。

## 选择回复的卡片（答题卡 / 计划复核卡）：消费 `--dsh-input-card-blur`
官方 `@deepseek-ai/dsh-client-ui-user-questions` 渲染两张卡，结构都是
`div.frame[data-*-key] > section.card`：
- 答题卡 `[data-question-key] > section`
- 计划复核卡 `[data-plan-review-key] > section`

两者 CSS 只有 `background: var(--dsw-specific-input-major)`、**没有 backdrop-filter**，
于是同一屏里输入卡是毛玻璃、这两张卡是死平面（用户 2026-09-14 反馈）。

```css
[data-question-key] > section,
[data-plan-review-key] > section {
  -webkit-backdrop-filter: blur(var(--dsh-input-card-blur, 10px)) saturate(1.3);
  backdrop-filter: blur(var(--dsh-input-card-blur, 10px)) saturate(1.3);
}
```
- 底色 token 本身半透明（亮 `rgba(255,255,255,.8)` / 暗 `rgba(26,34,56,.825)`），**补模糊就见效**，不用改底色。
- 模糊值直接读 `--dsh-input-card-blur`（皮肤中心的「输入卡模糊」就是写这个变量，
  外壳渲染层把同一个值给 `[data-composer-card]`）→ 拖滑杆时三处一起变。
- 用**语义属性**定位，不用哈希类名；**不加** `[data-slot="main.conversation"]` 作用域
  —— 这两张卡在输入区附近，未必在该 slot 内。
- ⚠️ 加 `backdrop-filter` 前已核对：该插件 `Tooltip` 出现 **0 次**、无 `position: fixed`
  → 不存在操作行那种「劫持 fixed 浮层定位」的风险。**任何要加 bf 的元素都先做这一步核对。**

## 「Go 5h」(cm-qstrip) —— 清掉外壳那层底
- 查证 cost-meter 自己的 CSS 里 `.cm-qstrip` **只有布局、没有任何 background**（`display:flex;flex-wrap:wrap;justify-content:center;width:100%;max-width:var(--dsh-chat-content-width,720px);padding:0 calc(var(--dsh-composer-side-clearance,0px) + 16px)`）。
- **里面每个 `cm-qchip` 小胶囊才各自有底**（`var(--dsw-alias-bg-layer-2)`）。用户说的「本来就有背景」指的就是这些小胶囊。
- 做法：用高特异性 `!important` 把外壳底清成 `transparent`（连 `backdrop-filter`/`box-shadow` 一起清），并**撤掉皮肤之前给的 fit-content/内边距**，交还插件原生布局（否则那一条会被压成 135px 的小条，与官方不一致）。

## 统计行 + 本会话行 —— 毛玻璃
用户先要求「优化成透明的」，做成**全透明后又反馈「需要背景模糊」**，故最终是**半透明底 + blur**：
```css
background: rgba(242,245,250,.55) !important;
backdrop-filter: blur(10px) saturate(1.3) !important;
box-shadow: none !important
```
底色调皮肤基色 `rgb(242,245,250)`，按「配色基准」规则取自左侧栏。深色主题 `rgba(16,22,42,.55) !important`。

## 覆盖外壳层的写法
覆盖外壳层必须用**同款选择器 + 皮肤前缀 + `!important`**：前缀加进去后特异性 (0,4,1) 压过外壳层的 (0,3,1)；这是 patches.css 里**唯一必须用 `!important`** 的地方。选择器要同时覆盖 `conversation.input.dock` 与 `conversation.composer.dock` 两种 dock 变体。

实测（F5 后）：cm-qstrip 条 `bg=透明/bf=none`（737 宽，原生布局，只余小胶囊自身底色）；统计行 `bg=rgba(242,245,250,.55) bf=blur(10px) saturate(1.3)`；本会话行同。

## cm-qchip（「Go 5h」里的小胶囊）hover 对齐
**问题**：插件自己的 hover 会让它"变透明"——
- `.cm-qchip{background:var(--dsw-alias-bg-layer-2)}`（常态 77.6% 浅灰实底）
- `.cm-qchip:hover{background:var(--dsw-alias-interactive-bg-hover)}`（hover 只有 8% 靛蓝）
- 实测 `0.776 → 0.08`，视觉上就是鼠标一移上去整颗胶囊淡掉。

**中途两版（都被用户推翻，别再来）**：
- ① 只把 hover 换成 `rgba(74,95,168,.18)`（仍"变淡"）
- ② 加 `blur(10px)`（**方向对了**——半透明 hover 不模糊就是"清晰透出壁纸"，像穿帮；见放大图 `z-chip-hover`）

**✅ 定稿（用户：「照着下面的那个轮数的样式改改」）—— 直接对齐同区域的「轮数」行，不发明新颜色。**
配件区 CDP 逐个 hover 测得的基准：

| 元素 | 常态 bg | hover bg | 备注 |
|---|---|---|---|
| 轮数行 `bOPqQW_root` = `[data-composer-stats]` | `rgba(242,245,250,.55)` + blur(10px) saturate(1.3) | **不变** | radius 12px |
| 轮数药丸 `bOPqQW_pill`（行内两颗可点胶囊） | 透明 | `rgba(74,95,168,.08)` | 插件标准 hover |
| 本会话行 `cm-root` | `rgba(242,245,250,.55)` + blur | 不变 | 皮肤补丁给的 |
| 余额/今日 `cm-foot` | 透明 | `rgba(74,95,168,.08)` | 插件标准 hover |
| `cm-qchip` | 插件默认 `--dsw-alias-bg-layer-2`（**0.776 偏灰实底**） | 跳到 8% 靛蓝**丢掉底色** | 所以是全场唯一的"灰盒子"且 hover 会变淡 |

定稿写法：常态 = 轮数行同款 `.55` + blur；hover = **保留这层底**，再叠一层插件标准的 8% 靛蓝（用 `background-image: linear-gradient(...)` 叠加，而不是把 `background` 换掉）：
```css
[class*="cm-qchip"] { background: rgba(242,245,250,.55); backdrop-filter: blur(10px) saturate(1.3) }
[class*="cm-qchip"]:hover { background-color: rgba(242,245,250,.55);
  background-image: linear-gradient(rgba(74,95,168,.08), rgba(74,95,168,.08)) }
```
深色主题同构：底色 `rgba(16,22,42,.55)`、叠加层 `rgba(160,185,235,.08)`（**必须补深色常态那条**，否则深色下会被浅色 `.55` 盖住）。

实测（F5 后）：常态 `bg=rgba(242,245,250,.55) bf=blur(10px) saturate(1.3)` **与轮数行/本会话行逐字节相同**；hover = `.55` + `LG(74,95,168,.08)`，净效果 == 轮数药丸 hover（`.55` 底 + 8% 靛蓝）。
放大对照图：`.shot/qchip-align-rest.png` / `qchip-align-hover.png` / `qchip-align-ref-stats.png`。

⚠️ 圆角仍是插件原来的 **6px**（用户只提颜色与 hover；轮数行是 12px 胶囊圆角）——要"更一致"可改 12px。
⚠️ hover 这颗胶囊 500ms 后插件会弹 tooltip（`Go: 0% · 重置:… 点击立即刷新`），截图里出现的黄条是它，不是样式问题。

## `[class*="yAWgPa_chip"]` = `ReferenceChip`（输入框里 @引用 的小胶囊）
- ① 补丁「补漏」批次曾把它误当成"会话 chip"改过（白底 `.5` + `blur` + `8px` 圆角）→ 已删。
- ② 2026-09-10 又短暂加过一版常态/hover 底色 → **同样已撤回**（当时把"hover 变透明"误判成它，实际是上面的 `cm-qchip`）。
- 它现在**完全沿用插件原样式**：`.yAWgPa_chip{background:var(--dsw-alias-interactive-bg-hover); border-radius:6px; color:var(--dsw-alias-state-business-primary); height:22px; padding:0 6px; display:inline-flex}`——常态就是 8% 靛蓝（`#4a5fa814`），**且插件没有任何 `:hover`**（实测 `:hover` 命中但底色不变）。
- ⚠️ 教训：**"某个元素 hover 变透明"这种反馈，一定要先在页面上把候选元素逐个 hover 量一遍**，别凭"最像"就动手——本次就是答错对象、白改一轮。若要让它更显眼，**别用左侧栏那个中性基色**（压在白色输入卡片上≈白、比 8% 靛蓝更看不见），要用它自己的蓝色系加强。

---

# Composer 输入区 / 停靠区（dock）样式规则

## 外壳渲染层的 accessory 规则（第二类白条）

skin-center 另有一张 `<style data-dsh-shell-rendering>`（**无 href，样式表索引拿不到名字，只能靠 `ownerNode.attributes` 识别**），其中一条把 composer 停靠区的**每一个直接子元素**都当 accessory 上背景：

```css
html[data-dsh-skin] [data-phase="active"] [data-slot="conversation.input.dock"] > *
html[data-dsh-skin] [data-phase="active"] [data-slot="conversation.composer.dock"] > *
```
→ `background / border-radius / backdrop-filter` **全带 `!important`**。

子元素自身 CSS 若写 `width:100%`（如 `dsh-client-ui-chat/StatsPills.module.css` 的统计行 `.xxx_root[data-composer-stats]`），会被拉成**通栏白条**（实测 **737px**）；`cm-qstrip` / `cm-root` 自带 `width:fit-content` 才是贴合小条。

### 修法（写在 `patches.css`，只动宽度、不碰 `!important` 背景）

给该子元素加：

```css
width: fit-content;
max-width: 100%;
margin-left: auto;
margin-right: auto;
padding-left: 10px;
padding-right: 10px;
```

实测 737px → **390px**，保留原有背景 / 模糊 / 圆角，深浅色主题自动适配。

### ⚠️ 内边距对齐

必须跟同区域兄弟配件对齐：`cm-qstrip`（Go 5h）/ `cm-root`（本会话 ¥…）在 cm-* 补丁里都是 **10px**。写 16px 会让这条比下面那行每边多凸出 6px。

**量法**：对 `[data-slot="conversation.input.dock"|"conversation.composer.dock"] > *` 逐个取 `getBoundingClientRect().width` 与 `paddingLeft/Right` 横向对比。

### 排查套路

列出 dock 的所有直接子元素（class / `data-*` / 宽度 / 是否 `fullWidth`）逐个对比，谁通栏就收谁；`[data-dsh-composer-*]` 这类语义属性比哈希类名稳定，优先用它。

## 注意

composer 模块的哈希前缀（如 `uV2eYG_`）不要当正文行处理，参见 `hash-selector-pitfalls`。

