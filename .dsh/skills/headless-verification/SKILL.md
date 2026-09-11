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

### 清理

用完只杀命令行含该 `--user-data-dir` 的进程，**别误杀用户自己的 Edge**。

## 皮肤 CSS 热更新

- 皮肤 CSS 响应头是 `cache-control: no-store`，但页面**只在加载时拉一次** → 改完必须让内容页重载（标题栏聚焦后按 F5，或重启后端）。
- 控制服务 `/action?name=reload` 需要**进程内令牌 `k`**（只以 `window.__DSHD_K` 注入页面），外部脚本拿不到，**别指望用 curl reload**。

## 流程约定（用户 2026-09-10 明确）

**每次改完 `patches.css`，验证截图前必须先 F5 刷新内容页**（无头验证就是脚本里的 `Page.reload {ignoreCache:true}`），否则截出来还是旧样式。

## 样式表定位技巧

- 带 href 的样式表：可用 `document.styleSheets[i].href` 查名。
- **无 href 的内联样式表**（如 `<style data-dsh-shell-rendering>`）：样式表索引拿不到名字，**只能靠 `ownerNode.attributes` 识别**。
