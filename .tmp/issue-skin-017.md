### 提交前查重

- [X] 我已搜索过 open/closed 的 Issue，确认本 Issue 没有重复。

相关但不同的既有 Issue：#1570（输入区配件与顶栏的变量归属，已修）、#1564 / #1563（配件改读正确变量、浮层配色与面板层次，已修）、#1678（折叠 rail 下座位错位，已修）。本 Issue 是 **0.1.7 新增/改动元素在 blue-fantasy 下的两处适配缺口**，与上面几单的根因不同。

### 涉及插件

blue-fantasy 皮肤（由 skin-center 承载）；涉及的**新元素来自外壳**：`@deepseek-ai/dsh-client-ui-chat`（过程行）、`@deepseek-ai/dsh-client-ui-conversation`（步骤行/配件）。不涉及任何第三方插件。

### Issue 类型

Bug 报告

### 摘要

0.1.7 把「过程」拆细之后，blue-fantasy 的补丁层没跟上，出现两处：

1. **新增的步骤过程行 `[data-step-process]` 没有底色/模糊**：同一族的 `[data-turn-process]` 行、工具调用行都走了蓝幻想的「可读性层」（半透明底 + 模糊），唯独新步骤行是裸文字，直接压在插画上。
2. **基础层的全局 `button:disabled` 覆盖了外壳有意留下的光标**：外壳（`dsh-client-ui-chat`）给 `[data-turn-process]` 写了 `.l_V-RG_root:disabled{cursor:default}` —— 它在 0.1.7 已退化成**只显示用时**的非交互信息条；而皮肤基础层 `patches.base.css` L93 的 `button:disabled, .aionui-btn:disabled, .aionui-menu-item-disabled{opacity:.55;cursor:not-allowed}` 加上皮肤前缀后特异性更高，把它盖掉了，于是这条信息条变成「🚫 光标 + 变灰」，看起来像被禁用的控件。

### 预期结果

1. `[data-step-process]` 标题行与同族行一致：半透明底 + 模糊 + 8px 圆角，**并保持左对齐**（图标仍与上下成员行对齐，不因加内边距而右移 8px）；
2. 「用时 N 分 M 秒」信息条不表现为禁用控件：`cursor: default`、不做额外衰减。

### 详情 / 复现步骤

1. dsh `0.1.7-rc.1` + blue-fantasy 皮肤，打开任一带工具调用的会话；
2. 看过程组里的步骤行（文案如「已完成分析」「已访问网页，已调用工具，执行了命令」）与回合末尾的「用时 …」胶囊。

实测（Edge 153 无头 + CDP，逐元素读 computed）：

| 元素 | 现状 | 本地补丁后 |
| --- | --- | --- |
| `[data-step-process] [data-process-activity]`（标题行 button） | `bg=rgba(0,0,0,0)`、`backdrop-filter: none`、`padding: 0` | `bg=rgba(242,245,250,.5)`、`blur(10px) saturate(1.3)`、`padding: 1px 8px`、`radius: 8px`；胶囊左缘 `x=417`、**图标仍在 `x=425`**（与成员行对齐） |
| `[data-turn-process][disabled]`（用时胶囊） | `cursor: not-allowed`、`opacity: .55`（皮肤全局规则盖掉外壳的 `cursor: default`） | `cursor: default`、`opacity: 1` |

### 环境信息

- DSH 版本: 0.1.7-rc.1
- 浏览器: Edge 153 无头 + CDP，视口 1280x900
- 皮肤 / 插件: blue-fantasy（skin-center 0.3.25）+ `@linxin666/dsh-web-all` 0.4.1
- 操作系统: Windows 11；`dsh web` 本机 `127.0.0.1:3080`
- 两处都与第三方插件无关（元素由外壳渲染），未装本皮肤时不会出现第 2 条

### Bug 截图

步骤行（同一 GUI、同一裁剪区域，2 倍放大；上：修复前，下：修复后）：

![步骤过程行 · 修复前（裸文字压插画）](https://raw.githubusercontent.com/A8Chann/dsh-web/evidence/017-skin-gaps/docs/evidence/017/before-step.png)

![步骤过程行 · 修复后（玻璃胶囊，图标仍与成员行对齐）](https://raw.githubusercontent.com/A8Chann/dsh-web/evidence/017-skin-gaps/docs/evidence/017/after-step.png)

「用时」信息条（上：修复前，整体被 `.55` 压灰；下：修复后。🚫 光标是 `cursor` 计算值，静态截图拍不到，见上表）：

![用时信息条 · 修复前](https://raw.githubusercontent.com/A8Chann/dsh-web/evidence/017-skin-gaps/docs/evidence/017/before-duration.png)

![用时信息条 · 修复后](https://raw.githubusercontent.com/A8Chann/dsh-web/evidence/017-skin-gaps/docs/evidence/017/after-duration.png)

### 冒烟测试

- 环境: dsh 0.1.7-rc.1 + blue-fantasy + Edge 153 无头（CDP `Runtime.evaluate`），Windows 11，视口 1280x900。
- 做法: 用探针样式临时还原/恢复两处规则，各切一次读 `getComputedStyle`（切主题后等 600ms 再读，避开 0.13s 过渡）。
- 观察结果: 见上表；修复后 `[data-step-process]` 标题行 `bg`/`bf` 到位、图标仍在 `x=425`；`[data-turn-process][disabled]` 为 `cursor: default` / `opacity: 1`。
- 变量联动: 把 `--dsh-skin-bubble-alpha` 设 `0 / .9`、`--dsw-skin-scrim` 设 `0 / .5 / 1`，步骤行底色随之变化（证明它走的是皮肤的气泡透明度/背景遮挡变量族，与其他行一致）。
- 回归: `Page.reload({ignoreCache:true})` 后收集到的 console error 与未捕获异常均为 0；`[data-turn-process]` 的展开态分隔线（`[data-open]`）未受影响。

### 引用代码

- 外壳 `@deepseek-ai/dsh-client-ui-conversation`：新增步骤行 `StepProcess`，稳定锚点是语义属性而不是哈希类名：
  `div.O_Ebla_root[data-step-process]` > `button[data-process-activity]`（内含 `[data-step-process-icon]` / `[data-step-process-chevron]`），展开体 `[data-step-process-body]` / `[data-step-process-content]`（成员行带 `data-turn-process-member`）。实测一个会话 56 行。
- 外壳 `@deepseek-ai/dsh-client-ui-chat`：`.l_V-RG_root:disabled { cursor: default; }`（0.1.7 起 `[data-turn-process]` 恒为 `disabled`，只显示用时，点击不可展开）。
- 皮肤 `patches.base.css` L93：`button:disabled, .aionui-btn:disabled, .aionui-menu-item-disabled { opacity: .55; cursor: not-allowed; }`。
- 本地补丁（可直接采纳或改写）：
  ```css
  [data-slot="main.conversation"] [data-step-process] [data-process-activity] {
    background: rgb(242 245 250 / calc(var(--dsh-skin-bubble-alpha, .5) * 1));
    -webkit-backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
    backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
    border-radius: 8px;
    padding: 1px 8px;
    width: fit-content;
    max-width: 100%;
    margin-left: -8px;       /* 抵消 padding，让图标与成员行对齐 */
  }
  body[data-ds-dark-theme] [data-slot="main.conversation"] [data-step-process] [data-process-activity] {
    background: rgb(16 22 42 / calc(var(--dsh-skin-bubble-alpha, .5) * .8));
  }
  [data-slot="main.conversation"] [data-turn-process][disabled] { cursor: default; opacity: 1; }
  ```
- ⚠️ 两个不要：底色**不要**加在 shimmer 文字那个 span（`_root_1bw19_1` 用 `background-clip: text`，加底会把文字变没）；也**不要**加在 `[data-step-process]` root（它包含展开正文 → 会叠成两层背景）。

### 补充信息

两处本地都是临时绕过。第 2 条的**更根本的修法**建议是收窄基础层那条全局 `button:disabled`（例如把会话区/信息条排除在外），而不是逐条补 `[disabled]` 覆盖 —— 否则以后每个自定义 disabled 外观的外壳组件都会被这条全局规则盖掉。

按仓库规则这里只提 Issue、未附 PR；需要补丁可随时说。
