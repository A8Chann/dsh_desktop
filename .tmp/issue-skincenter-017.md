### 提交前查重

- [X] 我已搜索过 open/closed 的 Issue，确认本 Issue 没有重复。

搜索 `ContextMeter` / `上下文已用` 均 0 命中；`accessory` / `配件` 命中的 #1570、#1572、#1566、#1563、#1564 都是 blue-fantasy 皮肤自身的配件/浮层问题（已修）。本 Issue **不是皮肤问题**：它出在 skin-center 的**外壳渲染层**，**所有皮肤**都受影响。

### 涉及插件

`@linxin666/dsh-client-ui-skin-center`（外壳渲染层的 composer accessory 规则）。被漏掉的元素来自外壳 `@deepseek-ai/dsh-client-ui-conversation` 的 `ContextMeter`。

### Issue 类型

Bug 报告

### 摘要

0.1.7 新增了「上下文已用 N%」环形指示（`ContextMeter`：圆环 + 百分比，点击弹上下文明细面板）。外壳把它放在

```
div.uV2eYG_dock
  ├ div[data-slot="conversation.composer.dock"] > [data-composer-stats]   ← 统计行
  └ span.JObwrW_root > button.JObwrW_trigger   ← ContextMeter
```

即它**与 `conversation.composer.dock` 槽位宿主平级**。而皮肤外壳渲染层的 accessory 规则只作用于

```css
[data-phase="active"] [data-slot="conversation.input.dock"] > *,
[data-phase="active"] [data-slot="conversation.composer.dock"] > *
```

—— 槽位的**直接子元素**。于是新配件拿不到配件底色/模糊，而**同一行的统计行有**，在同一行里出现「一个配件有底、另一个裸露」的不一致（开任意皮肤都会看到；不开皮肤时两行本来都没底，所以只有皮肤态才暴露）。

### 预期结果

该配件与同行统计行同底、同模糊（或明确它属"不需要配件底"的一类，并让统计行也保持一致，避免同一行两种外观）。

### 详情 / 复现步骤

1. dsh `0.1.7-rc.1` + 任意皮肤（本例 blue-fantasy / skin-center 0.3.25）；
2. 看输入卡下方那一行：左侧统计行（`N 轮 · tok/s`、`缓存命中 %`）有半透明底 + 模糊，右侧「◯ 17%」没有；
3. 逐元素读 computed 即可确认（见下表）。

实测（Edge 153 无头 + CDP）：

| 元素 | 现状 | 本地补丁后 |
| --- | --- | --- |
| 统计行 `[data-composer-stats]` | `bg=color(srgb .949 .961 .980 / .8)`、`bf=blur(10px) saturate(1.3)` | 同（未改） |
| `button.JObwrW_trigger`（ContextMeter） | `background: 0 0`（组件自带）、`backdrop-filter: none` | 与统计行**逐字节相同**：`bg=color(srgb .949 .961 .980 / .8)`、`bf=blur(10px) saturate(1.3)`；暗色 `color(srgb .114 .145 .224 / .824)` |

### 环境信息

- DSH 版本: 0.1.7-rc.1
- 浏览器: Edge 153 无头 + CDP，视口 1280x900
- 插件: `@linxin666/dsh-client-ui-skin-center` 0.3.25（经 `@linxin666/dsh-web-all` 0.4.1 装载）；被漏元素来自外壳 `@deepseek-ai/dsh-client-ui-conversation`
- 操作系统: Windows 11；`dsh web` 本机 `127.0.0.1:3080`
- 与具体皮肤无关：accessory 规则是 `[data-dsh-skin]` / `[data-dsh-custom-theme]` 通用的那两条

### Bug 截图

输入卡下方那一行（同一 GUI、同一裁剪区域，2 倍放大）：

![修复前：统计行有底，右侧「◯ 17%」裸露](https://raw.githubusercontent.com/A8Chann/dsh-web/evidence/017-skin-gaps/docs/evidence/017/before-ring.png)

![修复后：「◯ 17%」与统计行同一套底](https://raw.githubusercontent.com/A8Chann/dsh-web/evidence/017-skin-gaps/docs/evidence/017/after-ring.png)

### 冒烟测试

- 环境: dsh 0.1.7-rc.1 + blue-fantasy + Edge 153 无头（CDP），Windows 11，视口 1280x900。
- 做法: 临时注入与本地补丁同款的探针规则，切一次「有/无」并读 `getComputedStyle`（主题切换后等 600ms，避开 0.13s 过渡）。
- 观察结果: 修复前 `bg=rgba(0,0,0,0)` / `bf=none`；修复后亮 `color(srgb .949 .961 .980 / .8)` + `blur(10px) saturate(1.3)`，与同行统计行**逐字节相同**；暗色 `color(srgb .114 .145 .224 / .824)`。
- 命中数: 定位选择器命中 **1** 个元素（只有这颗环，面板内按钮不含 `svg circle`），没有误伤。
- 变量联动: 拖「背景遮挡」时底色 alpha 跟随（说明读的是 `--dsw-specific-input-major` 的 alpha，与同行配件同源）；模糊读 `--dsh-input-card-blur`。
- hover / 展开态: 保留这层底 + 叠插件标准的 8% 靛蓝（`Input.dispatchMouseEvent` 实测：底色不变，`background-image` 变 `linear-gradient(rgba(74,95,168,.08)…)`，文字色 `rgb(95,110,147) → rgb(62,75,109)`）——否则覆盖面规则特异性更高会把组件自己的 hover 反馈盖掉。
- 回归: `Page.reload({ignoreCache:true})` 后 console error 与未捕获异常均为 0。

### 引用代码

- skin-center 外壳渲染层（注入的 `<style data-dsh-shell-rendering>`，无 href，按 `ownerNode` 属性识别）里的配件规则：
  ```css
  html[data-dsh-skin] [data-phase="active"] [data-slot="conversation.input.dock"] > *,
  html[data-dsh-skin] [data-phase="active"] [data-slot="conversation.composer.dock"] > * {
    box-shadow: var(--dsh-composer-accessory-shadow, …);
    background: var(--dsh-composer-accessory-bg, var(--dsw-specific-tip, …)) !important;
    backdrop-filter: blur(var(--dsh-composer-accessory-blur, …)) !important;
    …
  }
  ```
- 外壳 `@deepseek-ai/dsh-client-ui-conversation` 的 `skeleton/ContextMeter.module.css`（类名前缀随构建变；可用结构与语义定位）：
  ```css
  .JObwrW_trigger{…;background:0 0;border:none;border-radius:24px;display:inline-flex;padding:1px 8px}
  .JObwrW_trigger:hover,.JObwrW_trigger[aria-expanded=true]{background:var(--dsw-alias-interactive-bg-hover);color:var(--dsw-alias-label-secondary)}
  .JObwrW_track{fill:none;stroke:var(--dsw-alias-border-l3);stroke-width:2px}
  .JObwrW_fill{fill:none;stroke:var(--dsw-alias-label-tertiary);stroke-width:2px;stroke-linecap:round}
  ```
  `aria-label` 是本地化文案（zh「上下文已用 {percent}」/ en「{percent} of context used」），**不宜**作为选择器。
- 本地补丁（语义槽位 + 结构定位，`button:has(svg circle)` = 「带圆环的按钮」，实测命中 1）：
  ```css
  [data-slot="conversation.composer.bar"] button:has(svg circle) {
    background: rgb(from var(--dsw-specific-input-major) 242 245 250 / alpha);
    -webkit-backdrop-filter: blur(var(--dsh-input-card-blur, 10px)) saturate(1.3);
    backdrop-filter: blur(var(--dsh-input-card-blur, 10px)) saturate(1.3);
  }
  body[data-ds-dark-theme] [data-slot="conversation.composer.bar"] button:has(svg circle) {
    background: rgb(from var(--dsw-specific-input-major) 29 37 57 / alpha);
  }
  /* hover / [aria-expanded=true]：保留底 + 叠 8% 靛蓝（深色 rgba(160,185,235,.08)） */
  ```

### 补充信息

建议的修法方向（供参考，按仓库规则这里只提 Issue、未附 PR）：

- 把配件规则的覆盖面从「槽位的直接子元素」扩展到**该停靠行的全部配件**，例如同时匹配停靠行容器（`uV2eYG_dock`）的直系子元素；或
- 用结构锚点补一条：`… [data-slot="conversation.composer.bar"] button:has(svg circle)`（与统计行同款底/模糊）；
- 若认为它**应当保持裸环**，那么同行的统计行也应同步去掉配件底，避免一行两制。

`[data-goal-bar]` 与 `[data-queue-dock]` 两个先例说明这类"新配件逃出 `> *`"的情况已有豁免/补齐的处理套路，可照此办理。
