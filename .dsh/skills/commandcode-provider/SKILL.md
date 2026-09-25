---
name: commandcode-provider
description: >
  Command Code provider 插件（@mars-sea/dsh-commandcode-provider）版本、思考强度缺失根因、升级修法，以及免 UI / 免第二个实例的验证方法。
whenToUse: >
  CommandCode provider 接入、模型选择器缺少思考强度、`KNOWN_EFFORTS`/`resolveModel`、升级该插件、tool 结果图片触发顺序 400。
---

# Command Code provider（@mars-sea/dsh-commandcode-provider）

> 基线：**0.11.13**，2026-09-24 为适配 dsh 0.1.7-rc.1 升级实测。宿主：受管版本 `v18d385b479a67f08`（0.1.7-rc.1），profile `web`。
> 历史基线：0.11.7（2026-09-20，全局 dsh 0.1.5-rc.2）；升级命令用的受管版本 `%APPDATA%\DSH Desktop\versions\v18d385b479a67f08`。

## 当前状态（2026-09-24）

- 已装 **0.11.13**（npm `latest`，2026-09-24 07:26 UTC 发布，与 dsh 0.1.7-rc.1 同日）。
- **0.11.8 的兼容范围 `^0.1.6-alpha.2 || ^0.1.6-alpha.1 || ^0.1.5-rc.2 || …` 不含 0.1.7-rc.1**（prerelease 元组不同），0.1.7 上必须升 0.11.13（`engines.dsh ^0.1.7-rc.1`、`dshReleases.0.1.7-rc.1=compatible`）。
- **两条本地补丁线均已退役**：思考强度随 **0.10.3** 发布；tool 结果图片触发顺序修复随 **0.10.6**（2026-09-12）发布 —— PR #33 于 09-11 合并，但 0.10.5 发布于合并之前，所以当时必须留着本地补丁。
- 判据（升级后必查）：`lib/index.js` 里 `pendingImages` **命中 8 次**（16 行），且 `Attached image(s) from tool result` 存在。
- `dsh.client.inject` 0.11.8 与 0.11.13 一致（含 `@deepseek-ai/dsh-api-remotes`，由 web-app bundle 图装载，非平台种子词）——客户端依赖面无新增，0.1.7 种子表与 0.1.5 相同。

## 症状与根因（思考强度，历史）

用 `deepseek/deepseek-v4.1-flash` 时模型选择器**没有思考强度选项**（官方有 `low`/`high`/`max`）。
根因：插件把可选思考强度写死在快照表 `KNOWN_EFFORTS`（镜像官方 CLI 模型表），`resolveModel` 只有命中该表才返回 `reasoning.efforts`；早于 0.10.3 的快照里没有该模型。官方 command-code **1.53.0**（2026-09-10）才加入它（Go 档、Vision、1M 上下文），插件 **0.10.3** 同步。

## 升级命令

```
node "%APPDATA%\DSH Desktop\versions\<id>\node_modules\@deepseek-ai\dsh\lib\bin.js" plugin --profile web add "@mars-sea/dsh-commandcode-provider@<精确版本>"
```

- Windows 下**不写 `^`**（cmd 转义）。
- 24h 冷静期：先在 `~/.dsh/profiles/web/pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 加 `'@mars-sea/dsh-commandcode-provider@<版本>'`。
  **新发现**：pnpm 11.24 在 `plugin add` 时若命中冷静期，会自己往该文件追加一行并打印
  `Added 1 entry to minimumReleaseAgeExclude in pnpm-workspace.yaml` —— 但显式先加仍然更稳（也留下日期注释）。
- **改完必须重启后端**（宿主模块在进程内存里）。重启入口只有壳侧动作：
  `GET /action?name=restart&k=<控制令牌>`；**令牌只注入自家页面**，脚本拿不到 → 实测裸调 `?name=restart` 会 **403**
  （`controls.rs` 只放行只读接口）。所以：让用户点标题栏蓝色药丸「· 点击重启更新插件」（`/plugin-hint` 为 `{"pending":true}` 即提示态已置位）。
- 生效判据：`(Get-Process -Id $s.pid).StartTime` 晚于 `lib/index.js` 的 `LastWriteTime`。

## 验证：两个层次（都不需要碰用户运行中的实例）

### 1) 单元级：打桩 `resolveModel`（秒级）

在 **profile 根目录**（`~/.dsh/profiles/web/`）建临时 `.mjs`，按相对路径 import（这样 `@deepseek-ai/*` 走 profile 的 node_modules 解析）：

```js
const { CommandCodeAdapter } = await import('./node_modules/@mars-sea/dsh-commandcode-provider/lib/index.js')
const a = Object.create(CommandCodeAdapter.prototype)
a.catalog = [{ id: 'deepseek/deepseek-v4.1-flash', name: 'x', contextWindow: 1e6, maxTokens: 131072 }]
a.deps = { options: () => ({}) }
console.log((await a.resolveModel('commandcode', 'deepseek/deepseek-v4.1-flash')).reasoning)
// 0.11.7 实测 → efforts [low, high, max]
```

打桩 `a.catalog` 是关键：否则 `resolveModel` 会回落 `loadCatalog()` 发网络请求。验证完删掉脚本。
导出的符号：`COMMAND_CODE_CLI_VERSION` / `CommandCodeAdapter` / `resolveAdapterOptions`（0.11.7 无 number 版本号导出）。

### 2) 端到端：全新 `DSH_HOME` 起第二个实例（几分钟，推荐给大版本升级）

不碰用户的 `~/.dsh`，也不占用 3080/19431：

```powershell
$env:DSH_HOME = "D:\HTML\DSH_Desktop\.tmp\ccverify-home"   # 独立 home
node <bin.js> --profile ccverify --from-default-profile web --dump-config   # 由 web 模板建 profile
node <bin.js> plugin --profile ccverify add "@mars-sea/dsh-commandcode-provider@0.11.7"
# 用 run_in_background 起：node <bin.js> --profile ccverify --no-open --port 3099，工作目录指向临时空目录
```

判据（本次实测全通过）：
1. 进程 25s 后仍存活，端口 3099 在听，打印 `dsh web: http://127.0.0.1:3099/?token=…`；
2. `curl -sL -c cookies "http://127.0.0.1:3099/?token=<token>"` 得 28 KB 首页；
3. 首页里组合 URL 含 `@mars-sea/dsh-commandcode-provider/client.js`，且**带上整串模块一起**请求（单独请求该模块会 404）
   → `200`，约 11.4 MB（`curl -b cookies`，URL 里的 `&amp;` 要还原成 `&`）—— 说明客户端 bundle 组装成功；
4. 宿主启动期无异常。⚠️ 该实例的 stdout 不进日志文件（没有 `logs/`），stderr 要看 job 输出。

收尾：`job_kill` → `Stop-Process` 残留 node → 删 `DSH_HOME` 与工作目录 → 确认 3099 已释放、主实例 `/status` 仍 `running`。
进程判定用 `Get-CimInstance Win32_Process | Where CommandLine -like '*ccverify*'`（不要只看 `Start-Process`，
本机 `Start-Process` 会因 `NO_PROXY`/`no_proxy` 键冲突直接抛异常，改用 `run_in_background` 的 pwsh job）。

## 兼容性与注意点（0.11.x）

- 0.11.7 `engines.dsh` / `peerDependencies.dsh` 声明：`^0.1.6-alpha.2 || ^0.1.6-alpha.1 || ^0.1.5-rc.2 || ^0.1.5-rc.1 || …` → 现行 0.1.5-rc.2 **在支持范围内**。
- **0.11.6 起 `react` / `@deepseek-ai/dsh-client-ui-slots` / `@deepseek-ai/dsh-client-ui-primitives` 改为 optional peer**
  （PR #52，为「老版 Desktop 校验 peer 闭包」而改）；pnpm 本就不装 optional peer，webview 从 `staticModules` 种子表解析，`pnpm peers check` 报的 missing peer 是**噪音**。
- 0.11.x 起对宿主版本做**能力探测 + 优雅降级**：0.11.0 侧边栏额度卡需要 dsh ≥ 0.1.5-rc.1 的中心面板能力；0.11.5 起新增 `data-composer-stats`
  缺失时回落到 `[data-slot="conversation.composer.dock"]`。所以插件比宿主「新」是设计内行为。
- 0.11.x 与 cost-meter 的交叉：`dsh-cost-meter` 的 CommandCode 面板是它**自己**读 `COMMANDCODE_API_KEY`
  （`lib/index.js` 里 `codingPlans.commandcode`），不依赖 provider 插件内部结构；本机 cost-meter 1.7.19 / npm 已 1.7.30，
  升级 cost-meter 属独立事项（见 `cost-meter-commandcode` 技能）。

## 工具结果图片触发顺序 bug（历史，0.10.5 本地补丁）

**症状**：400「An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id' (insufficient tool messages following tool_calls message)」。会话首轮历史重放即失败。

**根因**：一条 assistant 消息里并行多个工具调用（如两个 `read_image`），多条 tool-result 都带图时，`messagesToOpenAI` / `messagesToCC`
会把「图片携带 user 消息」插进 tool 消息之间：`assistant(A,B) → tool(A) → user(图A) → tool(B) → user(图B)`；网关转回 Anthropic 协议要求 tool 消息**连续**覆盖 tool_calls，中间插 user 即 400。

**上游修法**（PR [#33](https://github.com/Mars-Sea/dsh-commandcode-provider/pull/33)，0.10.6 发布）：`pendingImages` + `flushPendingImages()` 推迟图片消息，
并在每张图前标注来源 `Attached image(s) from tool result (call-a):`（并行两个 read_image 时信封文本相同，靠它区分归属）。
修复后形状：`assistant(A,B) → tool(A) → tool(B) → user(imgA) → user(imgB)`。

**回归验证**：正常会话里**同一条 assistant 消息并行两个 `read_image`**，两张图都返回、不 400、轮次继续（2026-09-11 在本机实测通过，
当时的补丁版即按此用例验证）。离线工具：`scripts/verify-toolimg-patch.mjs`（出错会话 `session-f1068d0a-107a-4538-9a16-7dcef3463d2f`）。
