---
name: commandcode-provider
description: >
  Command Code provider 插件缺思考强度选项的根因、升级修法与免 UI 的单元级验证方法。
whenToUse: >
  CommandCode provider 接入、模型选择器缺少思考强度、`KNOWN_EFFORTS`/`resolveModel`
---

# Command Code provider 与思考强度

> 记录时间：2026-09-11。

## 症状

用 `@mars-sea/dsh-commandcode-provider` 接 `deepseek/deepseek-v4.1-flash` 时，模型选择器**没有思考强度选项**；官方 Command Code 该模型有 `low` / `high` / `max` 三档。

## 根因

插件把「可选思考强度」写死在快照表 `KNOWN_EFFORTS`（镜像官方 CLI 的 `ZA` 模型表），`resolveModel` 只有命中该表才返回 `reasoning.efforts`。本机装的是 **0.10.2**，其快照停在更早的 CLI 版本，表里没有 `deepseek/deepseek-v4.1-flash`。

官方 command-code 在 **1.53.0**（2026-09-10）才加入该模型（Go 档、Vision、1M 上下文、efforts `low`/`high`/`max`），插件 **0.10.3** 同步。

## 修法

升级插件（`0.10.3` 起即可，本机升到 `0.10.5`）：

```
node "%APPDATA%\DSH Desktop\versions\v18d385b479a67f08\node_modules\@deepseek-ai\dsh\lib\bin.js" plugin --profile web add "@mars-sea/dsh-commandcode-provider@0.10.5"
```

⚠️ 该版本在 24h 冷静期内，需先在 `~/.dsh/profiles/web/pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 放行 `@mars-sea/dsh-commandcode-provider@0.10.3/0.10.4/0.10.5`。**改完必须重启后端**（宿主模块在进程内存里）。

## 免 UI 的单元级验证

不用起第二个实例、不碰用户运行中的会话：直接 import 插件并打桩调用 `resolveModel`。

```js
const { CommandCodeAdapter } = await import('file:///…/@mars-sea/dsh-commandcode-provider/lib/index.js')
const a = Object.create(CommandCodeAdapter.prototype)
a.catalog = [{ id: 'deepseek/deepseek-v4.1-flash', name: 'x', contextWindow: 1e6, maxTokens: 131072 }]
a.deps = { options: () => ({}) }
console.log((await a.resolveModel('commandcode', 'deepseek/deepseek-v4.1-flash')).reasoning)
// → { efforts: [{id:'low'…},{id:'high'…},{id:'max'…}] }
```

打桩 `a.catalog` 是关键：否则 `resolveModel` 会回落 `loadCatalog()` 去发网络请求。

## 工具结果图片携带消息的触发顺序 bug（0.10.5 本地补丁，2026-09）

**症状**：Command Code API 400：「An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id' (insufficient tool messages following tool_calls message)」。会话首轮历史重放即失败（llm/retry 后切 opencode-go 才继续）。

**根因**：一次 assistant 消息里发多个并行工具调用（如两个 read_image），多条 tool-result 都带图片时，`messagesToOpenAI`（lib/index.js）会把「图片携带 user 消息」插入到 tool 消息之间：
`assistant(A,B) → tool(A) → user(图片A) → tool(B) → user(图片B)`
Command Code 网关把 OpenAI 格式转回 Anthropic 协议时，要求 assistant 的 tool_calls 后**连续**被对应 tool 消息覆盖；中间的任何非 tool 消息（user）即 400。`messagesToCC`（转 Anthropic 原生格式）同样存在此问题。

**修法（本地补丁）**：`pendingImages` 数组 + `flushPendingImages()`，把图片携带 user 消息推迟：
- 函数头：`const pendingImages = []; const flushPendingImages = () => { for (const m of pendingImages) out.push(m); pendingImages.length = 0; };`
- 图片块：`out.push({role:"user",content:carried})` → `pendingImages.push({role:"user",content:carried})`
- flush 时机：下一条非 tool user 消息前、下一条 assistant 消息前、循环末尾。
修复后：`assistant(A,B) → tool(A) → tool(B) → user(imgA) → user(imgB)`。两个函数（messagesToCC ~1323 行、messagesToOpenAI ~1421 行）都要打。

**补丁痕迹**：`~/.dsh/profiles/web/node_modules/@mars-sea/dsh-commandcode-provider/lib/index.js*)`，备份 `index.js.orig-dsh-toolimg-patch`（172345 B → 补丁后 172817 B，node --check 通过）。**0.10.5 目前是最新版，升级后需重打**。

**上游 PR**：已提交 [Mars-Sea/dsh-commandcode-provider#33](https://github.com/Mars-Sea/dsh-commandcode-provider/pull/33)（分支 `A8Chann:fix/tool-image-carry-order`，commit 59e0619；含 `src/adapter.ts` + `tests/adapter.test.ts` 两个回归测试 + 重建的 `lib/`）。合并发版后本地补丁即可撤下。

**验证法**：
1. 离线单元级：`scripts/verify-toolimg-patch.mjs`（多帧 zstd 读会话 → 重建消息 → 模拟当前/修复转换 → Anthropic 连续性校验）。出错会话 `session-f1068d0a-107a-4538-9a16-7dcef3463d2f`：当前逻辑 `["msg#100 (user): 缺 call_01_ET_mAJWnSxgcjz5y8kSWlpW3588"]`，修复逻辑 `[]`。
2. **端到端（2026-09-11 已实测通过）**：正常会话里**同一条 assistant 消息并行两个 `read_image`**，
   两张图都正常返回、不再 400、轮次继续。这是原 bug 的确切触发场景，可直接当回归用例。

**生效前提（易漏）**：补丁写在 `lib/index.js`，属宿主模块 —— **必须重启后端**才进内存。
判据：比对「后端进程启动时间」与「补丁文件 mtime」，前者晚于后者才算生效：
```powershell
$s=(Invoke-WebRequest http://127.0.0.1:19431/status -UseBasicParsing).Content|ConvertFrom-Json
(Get-Process -Id $s.pid).StartTime          # 需晚于
(Get-Item "$env:USERPROFILE\.dsh\profiles\web\node_modules\@mars-sea\dsh-commandcode-provider\lib\index.js").LastWriteTime
```
补丁存在性快查：`Select-String -Path <该 index.js> -Pattern 'pendingImages'` 应有多处命中
（本次 16 处）；文件大小 172345 B（orig）→ 172817 B（补丁后）。

