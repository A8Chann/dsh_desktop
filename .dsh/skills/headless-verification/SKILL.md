---
name: headless-verification
description: >
  用无头 Edge + CDP 验证样式命中/截图，及皮肤 CSS 热更新与 reload 令牌限制。
---

# 无头浏览器 / CDP 验证与皮肤热更新

## 无头 Edge + CDP（比肉眼截图快且精确）

起一个无头 Edge 指向同一个 dsh 端口：

```
msedge --headless=new --remote-debugging-port=9222 \
  --user-data-dir=%TEMP%\edgecdp \
  "http://127.0.0.1:3080/?token=<token>"
```

再用 Node 的**全局 `WebSocket`** 走 CDP：

- `Runtime.evaluate`：遍历 `document.styleSheets` + `el.matches(rule.selectorText)` 找出真正命中的规则；
- `Page.captureScreenshot`：出图对照；
- `Page.reload {ignoreCache:true}`：改完再验一遍。

### 现成工具：`scripts/cdp/`（本次沉淀，已实测可用）

- `eval.mjs <expr文件>` —— 在页面里求值并打印结果
- `shot.mjs <out.png> [x y w h]` —— 截图（可选裁剪，scale=2）
- `reload.mjs [选择器] [超时ms]` —— 重载 + 收集 console error / 未捕获异常（默认等 `.cm-footer-stack`）

```sh
cd D:\HTML\DSH_Desktop
node scripts/cdp/reload.mjs                    # 改完补丁先跑这个
node scripts/cdp/eval.mjs %TEMP%\expr.js       # 再取数值
node scripts/cdp/shot.mjs %TEMP%\out.png 0 550 290 200
```

### ⚠️ 两个必踩的坑

- **表达式一律写进 `.js` 文件再传路径**。直接 `node eval.mjs "<内联表达式>"` 会被 PowerShell 的
  引号/转义处理搞坏（模板字符串直接 `SyntaxError: Invalid or unexpected token`）。
- `shot` 的裁剪用 **4 个独立数字**，别传 JSON（`'{"x":0,…}'` → `Expected property name or '}' in JSON at position 1`）。

### ✅ 截图验证可用（2026-09-11 补丁后），但数值仍是主证据

**历史坑（已修）**：`read_image` 读回的图片是 tool result 里的 image part，
在 Command Code provider 下会触发 `insufficient tool messages following tool_calls` 400，
整轮断掉、改到一半的补丁丢失（2026-09-10 实际发生）。

**2026-09-11 已修**：`@mars-sea/dsh-commandcode-provider` 的 `messagesToOpenAI` /
`messagesToCC` 对「图片携带 user 消息插到 tool 消息之间」做了 `pendingImages` 推迟
（详见 `commandcode-provider` 技能）。**已实测**：同一 assistant 消息里并行两个
`read_image` 正常返回、不再 400。补丁后**必须重启后端**才生效。

**但仍然建议**：样式类改动以 `getComputedStyle` 数值断言为主证据，截图作观感辅证 ——
数值可断言、可回归比对、不依赖视觉判断。够用的断言：
```js
// 目标行自身
{ bg, backdropFilter, borderRadius, padding, height, borderBottom }
// 多层背景：沿祖先链数非透明背景的层数
chain.filter(c => c.backgroundColor !== 'rgba(0, 0, 0, 0)').length
// 行内后代是否还有谁带底/模糊（应为 0）
[...el.querySelectorAll('*')].filter(d => bg !== 'transparent' || bf !== 'none').length
// 回归对照：同时取 callRow / disclosure-row / cm-root / sidebar 的 bg 与修复前比对
```

### ⚠️ 探针会污染页面，验完必须还原

`Runtime.evaluate` 里改过的页面状态**不会自己恢复**，会污染后续读数：

- 临时注入的 `<style id="__variant_probe">` —— 验完 `document.getElementById(...)?.remove()`；
- 临时 `document.body.setAttribute('data-ds-dark-theme', …)` —— **必须 `removeAttribute` 还原**，
  否则页面停在深色态，之后所有浅色读数（如 `sidebar` 应为 `rgba(242,245,250,.75)`）全部失真。
  实测踩过一次：验深色后忘了摘，下一轮探测把「行底色 `rgba(255,255,255,.5)`」读成深色值，
  一度误判规则没生效。

改完属性/样式**先 `void document.body.offsetHeight` 强制重排再读**，否则拿到旧的 computed 值。
收尾用一句「基线自检」确认还原：`sidebar` 底色 + `body.hasAttribute('data-ds-dark-theme')`。

### ⚠️ 造数据：`compactTranscript` 才出「N 次工具调用」行

这条过程行不是每个回合都有 —— 必须**同一回合里连续两次工具调用**（≥2 才折叠），
且渲染依赖 `compactTranscript`（= 设置「对话显示」为**紧凑**，默认值）。
只调一次工具、或单步回合，`[data-turn-process]` 数量恒为 0，容易误判成"选择器写错了"。

**造法**（页面里驱动，不用人手点）：`新会话` → 往 composer 的
`[data-composer-card] [contenteditable="true"]` 里 `Input.insertText` 一句话要求
「依次调用两次 Pwsh」→ `Input.dispatchKeyEvent` 发 Enter → 轮询
`document.querySelectorAll('[data-turn-process]').length` 直到 >0。

### 清理

用完只杀命令行含该 `--user-data-dir` 的进程，**别误杀用户自己的 Edge**：

```powershell
Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" |
  Where-Object { $_.CommandLine -like '*edgecdp*' } |
  ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
```

⚠️ 无头 Edge 会**随上次会话结束而退出**，下一轮要先探一次 9222
（`Invoke-WebRequest http://127.0.0.1:9222/json/version`，连接被拒就重新拉起），
否则 `reload.mjs` / `eval.mjs` 直接 `ECONNREFUSED`。
后端若重启过，**token 会变**，要重新从 `http://127.0.0.1:19431/status` 的 `url` 字段取。


## 皮肤 CSS 热更新

- 皮肤 CSS 响应头是 `cache-control: no-store`，但页面**只在加载时拉一次** → 改完必须让内容页重载（标题栏聚焦后按 F5，或重启后端）。
- 控制服务 `/action?name=reload` 需要**进程内令牌 `k`**（只以 `window.__DSHD_K` 注入页面），外部脚本拿不到，**别指望用 curl reload**。

## 流程约定（用户 2026-09-10 明确）

**每次改完 `patches.css`，验证截图前必须先 F5 刷新内容页**（无头验证就是脚本里的 `Page.reload {ignoreCache:true}`），否则截出来还是旧样式。

## 样式表定位技巧

- 带 href 的样式表：可用 `document.styleSheets[i].href` 查名。
- **无 href 的内联样式表**（如 `<style data-dsh-shell-rendering>`）：样式表索引拿不到名字，**只能靠 `ownerNode.attributes` 识别**。
