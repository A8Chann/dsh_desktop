---
name: session-migration
description: >
  dsh 会话迁移/准入闸门类 bug 合集：0.1.5-rc.1 SOURCE_KINDS 白名单缺 kind；0.1.7 v4 producer-owned source kind（kind:"plugin" 被拒 → 本轮运行失败）。含根因、本地补丁、验证脚本与 zstd 多帧日志读法。
whenToUse: >
  读取不了之前的对话」、会话迁移 `source v0`、`SOURCE_KINDS` 白名单、读 `session.jsonl.zstd`
---

# 「读取不了之前的对话」＝ 会话迁移闸门 bug

> dsh 0.1.5-rc.1，2026-09-10，已本地补丁。

## 现象

点侧边栏任意旧会话 → 右侧报：

```
历史加载失败：failed to observe session "…": cannot safely transform unclassified message source;
source v0 artifact remains unchanged (raw log: …\session.jsonl.zstd)
```

## 根因

`@deepseek-ai/dsh-session-format-v2-to-v3` 的 `SOURCE_KINDS` 白名单（15 项）不含**核心自己写出的** kind：

- `provider` / `fallback`（`@deepseek-ai/dsh-session-title`，会话标题 LLM 请求/兜底）
- `instruction-hint`（agent preset 的 tool-bootstrap 注入，如 liangshen 预设的 anchored-tool-bootstrap）

v0 会话一旦含这些来源，整段迁移就被拒绝（是 upstream 的 kind 分类漂移，**不是会话损坏**）。

## 本地补丁

会被 dsh 升级覆盖，升级后需重打。文件：

```
%APPDATA%\DSH Desktop\versions\<id>\node_modules\@deepseek-ai\dsh-session-format-v2-to-v3\lib\index.js
```

在 `SOURCE_KINDS` 末尾补 `"provider", "fallback", "instruction-hint"`（已带注释块）。**改完必须重启后端**才生效（迁移代码在宿主进程内存里）。

## 验证办法

另起一个实例（`node <bin.js> --profile minimal --no-open --port 3099`，别动线上那个）→ 无头 Edge 连上去点旧会话，看会话正文是否出现（出现即修复）。

## 读会话日志

`~/.dsh/sessions/<workspace>/<session-id>/session.jsonl.zstd` 是**多帧 zstd 追加**。Node 的 `zstdDecompressSync` 与流式解压都只吃第一帧；要按 `28 B5 2F FD` magic 切帧后逐帧解压才能拿到全部记录。

## B.「本轮运行失败 format v4 message requires a producer-owned source kind」（v3→v4）

> dsh 0.1.7-rc.1 / 0.1.7-alpha.1，2026-09-24，本机 web profile 已本地补丁。
> 上游讨论：deepseek-ai/deepseek-harness#7455。

### 现象

升级 0.1.7 后启动/开会话/发消息都正常，但**每一轮都失败**，GUI 报：

```
本轮运行失败 format v4 message requires a producer-owned source kind
```

### 根因

0.1.7 新增 `@deepseek-ai/dsh-session-format-v3-to-v4`，其 `source()` 准入（lib/index.js:126）要求
`message.source.kind` 为非空字符串**且 `!== "plugin"`**（producer-owned source kind），
V3 时代的插件包装 `{kind:"plugin", plugin:"<name>"}` 被明确废弃。
**实时新消息不走迁移**，插件只要还在实发 `kind:"plugin"`，消息一进会话就被拒 → 整轮失败。

本机元凶（web profile，`grep -rn 'kind: *["'"']plugin["'"']' ~/.dsh/profiles/web/node_modules`）：

| 插件 | 写入点 | 旧形态 |
|---|---|---|
| `@memtensor/memos-cloud-dsh-plugin` 0.1.1 | lib/index.js `createRecallMessage` | `{kind:"plugin", plugin:"memos-cloud", form:"recall"}` |
| `dsh-rule-manager` | lib/index.js `createUserMessage` | `{kind:"plugin", plugin:"dsh-rule-manager"}` |

两个包都无读侧 kind 依赖（只匹配 `source.kind === "user"`），可安全改写入侧。

### 本地补丁（会被插件升级覆盖，需重打；备份 `*.orig-v4kind`）

按迁移合成形态改 kind（`producerKind()` 对未知插件返回 `plugin:<原名>`，见 v3-to-v4 lib/index.js:92，
迁移产物保留其它字段、只丢 `plugin` 字段——补丁形态与迁移产物**完全一致**，历史/新消息 kind 统一）：

- memos-cloud：`kind: "plugin"` → `kind: "plugin:memos-cloud"`（删 `plugin` 字段，保留 `form`）
- rule-manager：`source: { kind: 'plugin', plugin: 'dsh-rule-manager' }` → `source: { kind: 'plugin:dsh-rule-manager' }`

改完 `node --check` + **重启后端**（或切回该 profile）生效。用户当时的规避手段是切到 minimal profile
（minimal 无第三方插件依赖，不报错，但失去 memos 记忆召回与 rule-manager）。

### 验证（脚本已落库，勿重复造）

1. **准入 oracle**（补丁形态是否过门）：`node scripts/verify-v4-kind-patch.mjs [session.v4.jsonl.zstd ...]`
   —— 用 core 自己导出的 `assertV4RowAdmission` 断言：旧形态必须抛
   `producer-owned source kind`、补丁形态必须通过；并检查两个插件文件无残留写入点；
   可选传入 v4 会话文件逐行跑同一断言。
2. **旧会话读取链**（读侧是否无恙）：`node scripts/verify-legacy-session-read.mjs <session.v3.jsonl.zstd ...>`
   —— 复刻真实读取链：`createSessionFormatCatalogWithChildren([]).createRestore(header, {recovery:'recoverable',validation:'current'})`
   → 逐行 `decodeRow` → `finish()`。本机 3 个旧 v3 会话（1.1 万+事件、116 条 plugin 行）全部
   v3→v4 干净迁移、残留 0，旧 memos-cloud 行迁移后正是 `plugin:memos-cloud`（与补丁形态一致）。
3. **真实宿主加载**：另起 `node <bin.js> --profile web --no-open --port 3099`，干净启动即插件树无误。

### 读侧结论（纠正 #7455 中 Ansonfishing 的「表现 B」担忧）

`SessionLogScanner.consumeEventLine` 确实是**先 `assertV4RowAdmission` 后 `restore.decodeRow`**
（worker.cjs:11944/11954），但该 scanner 只用于**当前世代 v4 文件**（`decodeCurrentGeneration`）；
旧世代文件走 `requireStoredLog → loadStoredMigration → prepareStoredMigration` →
`MigratingJsonlRows.consume`（直接进迁移 restore，**无准入门**）→ 迁移后才做 v4 校验。
所以对原始行跑 `assertV4RowAdmission` 被拒 ≠ 旧会话打不开；本机实证旧会话迁移无恙。
若将来真的出现「历史加载失败」，第一嫌疑才是这道门的排序，补法：准入按文件世代判定或移到迁移后。

### 排障命令

```powershell
# 找所有实时发 kind:"plugin" 的插件（JSON 里 key 带引号，grep 模式要写成 'kind": *"plugin'）
grep -rn 'kind: *["'']plugin["'']' ~/.dsh/profiles/*/node_modules --include=*.mjs --include=*.js
# 会话日志里统计旧形态（zstd 需多帧解码，用 scripts/session-log.mjs raw --grep）
node scripts/session-log.mjs raw <file> --grep 'kind": *"plugin'
```
