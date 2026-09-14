---
name: nav-and-sidebars
description: >
  改顶部导航栏 / 左右侧边栏底色与面板渲染层级（display:contents 宿主坑）。
---

# 顶部导航栏 / 右侧侧边栏 / 左侧栏底色规则

## 左侧栏底色（基准色）

`[data-pane="sidebar"]` 的 computed `backgroundColor` = **`rgba(242,245,250,.75)`**（实测值）。顶部导航栏与右侧栏面板统一用这个色 + `.75`：

```css
[data-slot="main.conversation"] header,
[data-slot="rightbar.session"] > *,
[class*="rightbarCol"] [class*="panel"] {
  background: rgba(242,245,250,.75);
}
```

顶部另有 1px 分隔线。深色主题：`rgba(16,22,42,.75)`。

### ⚠️ 外框用**固定** 0.75，不要跟随 `--dsh-skin-bubble-alpha`
2026-09-11 用户明确：「气泡不透明度不应该联动顶部导航和右侧边栏」。外框是框架不是气泡 —— 拖动气泡滑杆时导航栏/右侧栏必须**纹丝不动**。
- 曾经写成 `rgb(242 245 250 / calc(var(--dsh-skin-bubble-alpha, .5) * 1.5))`（默认 50% 时正好 .75），被用户否掉，已改回固定 `rgba(242,245,250,.75)`。
- 验证方法：把 `--dsh-skin-bubble-alpha` 依次设成 `0.9 / 0.1 / 0`，导航栏与右侧栏应**始终** `.75`，而正文托底跟着变。

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
