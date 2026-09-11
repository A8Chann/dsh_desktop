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

## 通用教训

给 `[class*="…"]` 加视觉属性前，先看这条选择器会不会同时命中**同一子树里的多层**；要「只留一层」就得连内层一起重置。跨构建哈希选择器优先加作用域或改用语义属性。
