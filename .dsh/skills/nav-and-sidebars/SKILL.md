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

### hover 提示气泡（`[role="tooltip"]`）同属「外框」，也固定浓度
壳层规则：`._bubble_1nw3t_1 { background: var(--dsw-alias-tooltip-bg); color: var(--dsw-static-neutral-bluish-00) }`。
- ⚠️ `--dsw-alias-tooltip-bg` = **`#ffffe1` 固定淡黄，亮暗主题都不变**（壳层不做主题区分）→ 暗色下整屏唯一的暖黄块，很突兀。
- 皮肤基线区（L30 附近）只把文字色换成了 `--dsw-alias-tooltip-fg`；底色覆盖加在**本地追加区**，靠层叠顺序取胜（同特异性、后出现者赢）。
- 2026-09-14 定稿：亮 `rgba(242,245,250,.92)` / 暗 `rgba(16,22,42,.92)`，文字 `#1d2539` / `#dbe2f2`，加 0.5px 发丝边 + `blur(10px)`。
  浓度取 **.92 固定**、比导航栏 `.75` 更实（气泡只有一行字且直接压在内文上，需要更高对比），同样**不跟滑杆**。
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
- 同类「壳层写死深色」的还有 `--dsw-alias-tooltip-bg: #ffffe1`（淡黄，亮暗同值）。找这类问题的通用手法：
  悬停后遍历 `document.styleSheets`，打印命中元素且带 `background` 的规则并标出来源。

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
