# memos-cloud-dsh-plugin 本地兼容修复（0.1.0-patched）

> ## ⛔️ 2026-09-10 起本补丁已废弃（保留仅供历史回溯）
>
> 上游已发布 **`@memtensor/memos-cloud-dsh-plugin@0.1.1`**（npm 2026-09-07），
> 改用新 API `settingsCtx.settings.register("memos-cloud", Config, { base, validate })`，
> **不再导入** `installSettingsSection` / `settingsNamespace`；peer 依赖里也补上了
> `^0.1.2-rc.1` 分支。web profile 已把依赖改回 registry 包
> （`"@memtensor/memos-cloud-dsh-plugin": "^0.1.1"`），并在 dsh **0.1.5-rc.1** 上验证通过。
>
> **不要**再执行 `scripts\apply-memos-plugin-fix.ps1`：它会把依赖改回 `link:` 指针、
> 回退到打过补丁的 0.1.0，覆盖当前的官方包。`$DSH_HOME/profiles/web/vendor/`
> 下的这份拷贝也不再被任何依赖引用，可随时删除。
>
> 以下为当时的（已过时的）背景与做法记录。

上游 `@memtensor/memos-cloud-dsh-plugin@0.1.0`（npm 2026-08-16 发布，此后无更新；
GitHub `MemTensor/MemOS-Cloud-Dsh-Plugin` 与 `MemTensor/MemOS-Cloud-OpenClaw-Plugin`
最后提交同为 2026-08-16，且 OpenClaw 仓库已移除 `packages/dsh` 子包）使用了
`@deepseek-ai/dsh-settings` 的**已删除** API：

```js
import { installSettingsSection, settingsNamespace } from "@deepseek-ai/dsh-settings";
```

dsh 0.1.2-alpha.2 随附的 `@deepseek-ai/dsh-settings@0.1.2-alpha.2` 只导出
`SettingsConflictError / SettingsProvider / redactSecrets`，移除项：

| 旧 API | 新 API |
| --- | --- |
| 自由函数 `installSettingsSection(ctx, ns, schema, entry, hooks)` | `SettingsProvider#installSection(owner, ns, schema, entry, hooks)`（经 `ctx.inject(["settings"], cb)` 注入） |
| `settingsNamespace("memos-cloud")`（品牌字符串） | 命名空间校验移入 `register()`/`installSection()`，直接传 `"memos-cloud"` |

## 本目录内容

`assets/memos-cloud-dsh-plugin-fixed/` = npm 0.1.0 包体（`lib/index.js`、`lib/index.d.ts`、
`cordis.patch.yml`、`package.json`、LICENSE、README）＋ 以下两处修改：

1. `lib/index.js`
   - 删除 `import { installSettingsSection, settingsNamespace } from "@deepseek-ai/dsh-settings";`
   - `MEMOS_SETTINGS_NAMESPACE = settingsNamespace("memos-cloud")` → `"memos-cloud"`
   - `installSettingsSection(ctx, ...)` → `ctx.inject(["settings"], (sctx) => sctx.settings.installSection(ctx, ...))`
2. `lib/index.d.ts`
   - `import("@deepseek-ai/dsh-settings").SettingsNamespace` → `string`

其余 4 个 peer 导入（`launchEnvironmentOf` / `isAppendSurfaceEvent` / `credentialRef` /
`createUserMessage`）在 dsh 0.1.2-alpha.2 对应包中仍存在，无需改动。

## 部署方式

`dsh --profile web`（即 `dsh web`）的 profile 位于 `$DSH_HOME/profiles/web`
（`$DSH_HOME` 默认 `%USERPROFILE%\.dsh`）：

1. 把本目录整包复制到 `$DSH_HOME/profiles/web/vendor/memos-cloud-dsh-plugin/`；
2. 把 profile `package.json` 依赖改为
   `"@memtensor/memos-cloud-dsh-plugin": "link:./vendor/memos-cloud-dsh-plugin"`；
3. 在 profile 目录执行 `pnpm install`（node_modules 中该包变为指向 vendor 的符号链接，
   pnpm-lock.yaml 同步记录；后续 `dsh plugin add/remove` 重建 node_modules 后仍保留）。

直接执行 `pwsh scripts\apply-memos-plugin-fix.ps1` 可一步完成上述 1-3 步并校验
（该脚本已加废弃拦截，需 `-Force` 才会真正执行）。

上游发布修复版本后，把依赖改回 `"@memtensor/memos-cloud-dsh-plugin": "^0.1.1"` 并
`pnpm install`，即可回归官方包（建议删除本目录）——**2026-09-10 已完成该回归**。
