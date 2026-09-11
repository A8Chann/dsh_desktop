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
