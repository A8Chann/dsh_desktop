---
name: session-migration
description: >
  dsh 0.1.5-rc.1 会话迁移闸门 bug（SOURCE_KINDS 白名单缺 kind）的根因、本地补丁、验证与 zstd 多帧日志读法。
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
