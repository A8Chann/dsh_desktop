# 向上游 dsh-web 提 Issue / PR 的实操心法（2026-09-10 实战沉淀）

本目录是这次「把本地改动回馈上游」的全套素材与踩坑记录。仓库：[`zhu1090093659/dsh-web`](https://github.com/zhu1090093659/dsh-web)（7325 star / 478 fork）。

## 一、最重要的红线：只接受四类内容贡献

`.github/PULL_REQUEST_TEMPLATE.md` 与 `CONTRIBUTING.md` 写得很硬：

> 本仓库**只接受四类内容贡献**：插件申请（社区插件索引登记）、**皮肤增加（新皮肤收录）**、宠物增加、预设增加。
> **其余改动（修复 / 增强 / 全新功能 / 文档 / 测试 / 维护）不接受直接 PR，请先提 Issue 讨论**；
> 只勾「壁纸 / 渲染器」类别的 PR 会被自动关闭。仓库所有者 / 机器人 / 有写权限的协作者不受此限。

判断口径（本次实测）：

| 想提的东西 | 能不能直接 PR | 正确做法 |
|---|---|---|
| 新增功能（如 skin-center 的气泡模糊滑杆） | ❌ 会被 `Close PRs outside the accepted content scope` 关掉 | **先提 Issue**（本次 [#1469](https://github.com/zhu1090093659/dsh-web/issues/1466)），等维护者点头再开 PR |
| 既有皮肤的样式打磨 / 视觉修复（如本次可读性层） | ✅ 属于「皮肤 / 皮肤中心（皮肤样式）」 | 直接 PR（先例 [#1420](https://github.com/zhu1090093659/dsh-web/pull/1420) 同为外部贡献者的既有皮肤打磨） |
| 新皮肤 / 新宠物 / 新预设 | ✅ 内容贡献，欢迎直接提交 | 直接 PR |
| 插件的 bug 修复、文档 | ❌ | 提 Issue |

机器人会在 PR 上跑 5 道门禁（本次全绿）：分类路由、按路径打标签、关闭纯文档 PR、**关闭超出内容范围的 PR**、**校验贡献证据**。最后一条意味着 PR 正文必须按模板填全（尤其「测试证据」「视觉修复要求」「AI 编码披露」）。

## 二、提 Issue 的正确姿势（API 侧踩坑）

- 仓库的 Issue 表单是 GitHub **form**（`.yml`），标题前缀固定 `[Bug]: ` / `[Issue]: `；正文用 Markdown 按表单字段逐节写（`### 提交前查重` / `### 涉及插件` / `### Issue 类型` / `### 摘要` / `### 预期结果` / `### 详情 / 复现步骤` / `### 环境信息`）。
- **PowerShell 5.1 的 `ConvertTo-Json` 会把长中文正文膨胀到 450 KB**（3 KB 正文 → HTTP 422 Unprocessable Entity，且响应体为空、不给理由）。**用 Node 生成 payload**：
  `node -e "JSON.stringify({title,body})"` 写到文件，再 `curl.exe --data-binary "@payload.json"`。
- 带 labels 创建也可能 422（`enhancement` 明明存在）——最小 payload 才 201。**先用最小 payload 验证通道**，再用 `PATCH /issues/<n>` 把正文改成正式内容（Issue 不能用 API 删除，只能改；所以别拿正式仓库试手，最小 payload 也会真的建出一条）。
- 图片：GitHub API 不能直接给 Issue 附图。**在 fork 里开一个只放图的资产分支**，然后用 `https://raw.githubusercontent.com/<user>/dsh-web/<branch>/<path>` 直链引用（本次 `assets/skin-center-bubble-blur` 分支）。

## 三、提 PR 的正确姿势

- fork → `git clone --depth 1 --branch dev`（仓库 **640 MB**，全量 clone 后 `pnpm install` 让 node_modules 再涨到 **~700 MB**；放 D 盘）。
- 分支名 `feat/...`，提交信息 Conventional Commits、**禁 emoji**。
- 正文按 `.github/PULL_REQUEST_TEMPLATE.md` 逐节填：摘要 / 涉及包（勾选） / PR 类别（**必须落在四类内**） / PR 类型 / 最新代码确认 / 测试证据与上游同步 / 视觉修复要求（视觉类必填，且**模型必须支持图像输入**） / **AI 编码披露（必填，模型与工具不得留空）** / 仓库规范检查 / 本地验证 / 用户可见变更证据。
- 视觉证据照 [#1420] 的惯例归档进 `docs/archive/<YYYY-MM-DD>-<topic>-*/`，图片直链引用该分支的 raw 地址（自包含、不依赖外部图床）。

## 四、皮肤改动必须重建「画廊产物」（否则 CI 红）

皮肤在仓库里有 **3 份**，改一份要同步另外两份：

| 路径 | 关系 |
|---|---|
| `packages/skins/skin-center/skins/<id>/` | **权威源**（改这里） |
| `market/dist/assets/skins/<id>/` | 逐字节副本（本次实测 hash 与权威源一致） |
| `market/dist/tryon-assets/skins/<id>/` | **经安全管线转换**（选择器被强制加 `html[data-dsh-skin]` 前缀，体积变大），不能手抄 |
| `market/dist/assets/skins/<id>.zip`、`market/dist/styles.js` | 同批产物 |

一条命令搞定：`pnpm market:build`（本次 `wrote 2508 files (31 skins, ...)`），校验用 `pnpm market:check`（`tryon/ verified against hash manifest (756 files)`、`dist up to date (1720 files)`）。
⚠️ `pnpm install` / 构建会顺手改到 `packages/dsh-web-all/lib/client.js.map` 这类**无关产物**，提交前 `git checkout --` 回退。
其它门禁：`pnpm skin-center:check`（目录册 + hooks 注册表）、`pnpm docs:check`（双语文档配对）。

## 五、改 skin-center（TypeScript）的完整触点与坑

给「背景」卡加一个字段，**要改 6 个源文件**（少一个就 typecheck 失败 / 面板崩）：

1. `src/core/background.ts` — 接口字段 + `SKIN_BACKGROUND_DEFAULTS` + `RANGES`（自动覆盖 normalize / sanitize / resolve / FIELDS）
2. `src/index.ts` — 宿主 zod schema
3. `src/client/background.ts` — 字段常量 + CSS 变量名 + `SkinBackgroundHandle` 的两个方法 + 控制器（字段 / 构造与 `init()` 的应用调用 / `snapshot()` / getter / setter / `dispose()` 清理 / `assign()` / `apply*()`）
4. **`src/client/index.ts` — injected handle 门面**（逐条列方法）。**最容易漏**：漏了 → `tsc` 报 `TS2739 … missing the following properties: bubbleBlur, setBubbleBlur`，运行时组件直接崩（`useSyncExternalStore(subscribe, undefined)` → `TypeError: n is not a function`）。
5. `src/client/SkinCenter.tsx` — `useSyncExternalStore` + `useLiveValue` 两个 hook + `SliderControl` 行
6. `src/client/locales.ts` — `SkinCenterKey` 键联合类型 + `en` / `zh` 词典（该文件目前只有这两份词典）

测试侧（同样要改，否则断言失败或 mock 崩）：`tests/background.spec.ts`（snapshot `toEqual` 会枚举字段 + 补一条新字段的行为用例）、`tests/background-migration.spec.ts`（“resolved schema defaults” 夹具）、`tests/background-scope.spec.ts`（`current` 夹具）、`tests/skin-center-custom-theme.spec.tsx`（**手写 handle mock**，漏方法会让组件崩）、`tests/routes-v2.spec.ts`（区间夹紧断言）。

**完整性检查的正确写法**：`Select-String -Pattern 'bubbleOpacity'` 后**不要**用 `\\index\.ts` 这类宽泛排除 —— 它会连 `src/client/index.ts` 一起排掉（本次就是这么漏的）。按**文件分组计数**再看清单。

构建/门禁（只需该包依赖时）：`corepack pnpm install --filter "@linxin666/dsh-client-ui-skin-center..."`（2m33s，且 `prepare` 会跑 `tsdown` 顺带验证编译），然后 `typecheck` 与 `test`（本次 **35 files / 618 tests passed**）。

## 六、提交状态（2026-09-14）

| 条目 | 状态 |
|---|---|
| [issue #1469](https://github.com/zhu1090093659/dsh-web/issues/1469) 气泡模糊滑杆提案 | **已关闭（completed）**，维护者 @Aa728848 明确邀请提 PR，指定基线 `dev`、按模板填、无 Emoji |
| [PR #1468](https://github.com/zhu1090093659/dsh-web/pull/1468) blue-fantasy 文字可读性层 | **已合并**（merge commit `bf1e40e`） |
| [PR #1476](https://github.com/zhu1090093659/dsh-web/pull/1476) 后续：联动滑杆 / 代码块毛玻璃 / 表格托底收窄 / 过程行去双层 / 窗口外框底色 | **已合并** |
| [PR #1516](https://github.com/zhu1090093659/dsh-web/pull/1516) 气泡模糊程度滑杆（`bubbleBlur`） | **已合并**（merge `ffedeae`，维护者先 APPROVED 再合并）；随 **0.3.22** 发布 |
| [PR #1562](https://github.com/zhu1090093659/dsh-web/pull/1562) 修 blue-fantasy 操作行的 hover 气泡跑飞（`backdrop-filter` 劫持 fixed 定位） | **已被关闭**，维护者：`提交issues，目前不接修复PR` |
| [PR #1563](https://github.com/zhu1090093659/dsh-web/pull/1563) 浮层配色与面板层次（气泡 / hover 卡 / 队列 dock / 答题卡 / 右栏与底部面板） | **已被关闭**，同上 |
| [PR #1564](https://github.com/zhu1090093659/dsh-web/pull/1564) 输入区配件与顶栏改读正确变量（输入卡模糊 / 背景遮挡） | **已被关闭**，同上 |
| issue [#1571](https://github.com/zhu1090093659/dsh-web/issues/1571) 操作行 hover 气泡跑飞 | **已修复**（`9407d83`，改法与本地一致） |
| issue [#1572](https://github.com/zhu1090093659/dsh-web/issues/1572) 队列 dock 两层底 + 每边宽 8px | **已修复**（`9407d83`，修在**壳层** `shell-rendering.ts`） |
| issue [#1573](https://github.com/zhu1090093659/dsh-web/issues/1573) 右侧面板叠两层 / 分割线不一致 / 底部面板竖线断层 | **修 2/3**：叠两层 ✅、分割线 ✅；**底部面板断层不修**（称属上游 docking kit 几何，建议去 DSH 主仓提） |
| issue [#1574](https://github.com/zhu1090093659/dsh-web/issues/1574) 气泡与 hover 卡用未做主题区分的固定色 | **已关闭（not_planned）**；且**我方归因被纠正**：`#ffffe1` 是**本皮肤 `skin.css:87/178` 自己钉的**，不是壳层（详见 `nav-and-sidebars` 的归因更正） |
| issue [#1569](https://github.com/zhu1090093659/dsh-web/issues/1569) 答题卡缺毛玻璃 | **已关闭（not_planned）**，按现状保留 |
| issue [#1570](https://github.com/zhu1090093659/dsh-web/issues/1570) 配件与顶栏的变量归属 | **已关闭（not_planned）**，理由「外框密度固定是记录在案的刻意决定」 |
| issue [#1579](https://github.com/zhu1090093659/dsh-web/issues/1579) 顶栏/右栏密度没跟随「背景遮挡」 | **open** —— 换角度重提（见下方说明） |

> 💡 **#1579 的论证角度值得复用**：被自己早先的 PR 注释挡回来时**不要正面推翻它**，而是
> （a）承认来历经查证确实出自自己那份 PR（附 commit `5c7c9c92` + merge `b890a79`）；
> （b）指出**代码已与那段注释自己声明的目标不符** —— 注释写「三栏共享同一基色才读作一个面」，
> 而本皮肤 `skin.css:95/186` 自己把左栏定义成跟随遮挡、`patches.css` 又把另两栏钉死，
> 实测三栏**只在遮挡 = 50% 时同密度**（0 时左栏 α1.00 vs 0.75，1 时 0.50 vs 0.75）；
> （c）说明新请求**不违反**原决定（#1476 只排除了「气泡滑杆」，没排除「壁纸遮挡」，改法读的是遮挡 token）；
> （d）建议顺手把注释写准确，免得「密度固定」继续被当成「永远不该动」的依据。

> 三个 PR 互不重叠，且 **CI 全绿、mergeable**，但**仍被维护者以「目前不接修复 PR」整体关闭**（2026-09-14）。
> 结论：本仓库当前只走 issue 流程，先提 issue 等邀请，不要再直接提修复 PR。内容已按 issue 全部重新提交（见上表）。

> ⚠️ **#1466 是被机器人关掉的废稿**（第一次用最小 payload 试通道，未按模板填写）。教训见第七节。
> ⚠️ **#1516 首轮被 `Validate PR contribution evidence` 驳回** —— 原因与修法见第八节。
> ⚠️ **#1565–#1568 是被 issue 模板校验器关掉的**（API 提交绕不过标签竞态）。原因与绕法见第九节。
> 💡 **#1562 首轮就过**，因为正文按第八节第 1 条把「结果摘要：」后面的括号去掉了。

## 六之二、上线收尾（2026-09-14 完成）

**#1516 的合并 → 发布 → 本地升级 → 撤补丁，这条链已经走完：**

| 步骤 | 结果 |
| --- | --- |
| 合并 | `ffedeae`，2026-09-13T07:38Z |
| 发布 | `@linxin666/dsh-client-ui-skin-center` / `dsh-web-all` **0.3.22**（同日 14:24–14:28Z） |
| 本地升级 | 20 个 `@linxin666/*` 包全部 → 0.3.22；`profile/package.json` 依赖 → `^0.3.22` |
| 撤补丁 | 升级前先 `--revert`，升级后 **5 个文件与官方 tarball 逐字节一致**（零残留） |
| 实测 | 设置 → 皮肤 → 背景卡出现第 6 个旋钮「气泡模糊程度」（0–20 / 默认 10），拖动 → `--dsh-skin-bubble-blur` 跟着变 |

**踩到的两个小坑（都已处理）：**

1. **pnpm 24h 冷静期**：0.3.22 发布仅 13 小时后升级，必须先在该 profile 的
   `pnpm-workspace.yaml` 的 `minimumReleaseAgeExclude` 里放行 —— 聚合包**加上它依赖的 19 个子包**共 20 条。
2. **热修脚本的「已打」判据失效**：`skin-center-bubble-blur.mjs` 原本用 `BUBBLE_BLUR_VAR` 判断，
   而上游 0.3.22 原版 bundle 里**自己就带这个标识符（4 处）** → 永远误报「已打」。
   已改用脚本独有的中文注释标记，并加**版本闸门**（≥ 0.3.22 直接提示废弃并退出）。

## 七、本目录内容

| 文件 | 说明 |
|---|---|
| `skin-center-bubble-blur/0001-…patch` | 可直接 `git apply` 的源码补丁（**11 文件 / +109 −3**，含 5 个测试文件） |
| `skin-center-bubble-blur/ISSUE-BODY.md` | 已提交的 Issue 正文（= [#1469]） |
| `skin-center-bubble-blur/PR-BODY.md` | 已提交的 PR 正文（= [#1516]） |
| `blue-fantasy-readability/readability-subset.css` | 首版 PR 提交的 166 行可读性子集 |
| `blue-fantasy-readability/PR-BODY.md` | 首版 PR 正文（= [#1468]，正文已更新为后续说明） |
| `blue-fantasy-readability/PR2-BODY.md` | 后续 PR 正文（= [#1476]） |

~~遗留：**#1516 合入并发布后**，记得撤掉本地那份**产物级**热修。~~
**✅ 已于 2026-09-14 完成** —— 见上面「六之二、上线收尾」。热修脚本现已带版本闸门，检测到 ≥ 0.3.22 会直接提示废弃。

## 八、PR 被自动检查驳回的两个坑（2026-09-13 实测）

### 1. 「本地验证」的标签后必须紧跟冒号
`pr-contribution-rules.yml` 的 `readValidationPart` 用
`执行的命令\s*[：:]([\s\S]*?)(?=\n\s*结果摘要\s*[：:])` 取值。
写成 **`结果摘要（全部通过）：`** 会让 `结果摘要\s*[：:]` 匹配失败 →
**两段都被判空** → 报「请填写 本地验证 中的执行的命令与结果摘要」。

- 只能写 `结果摘要：`，括号说明挪到正文里。
- 本地 `scripts/pr-review.mjs` 的实现**更宽松**（标签后允许括号），所以**本地自检过 ≠ CI 过**。
  要按 CI 的正则自检，可直接复用 `pr-contribution-rules.yml` 里的那几行 JS。

### 2. 推「基于最新 dev 的分支」需要 token 有 `workflow` 权限
上游在旧基线之后的 60 个提交里改了 `.github/workflows/ci.yml`，而 fork 的 `dev` 还停在旧版。
于是**任何包含新 ci.yml 的推送都会被拒**：

```
refusing to allow a Personal Access Token to create or update workflow
`.github/workflows/ci.yml` without `workflow` scope
```

- 连 **GitHub 自己的 fork 同步**（`POST /repos/{owner}/{repo}/merge-upstream`）也会被同一限制拒（HTTP 422），
  因为它在服务端做的是 fast-forward、同样要写 workflow 文件。
- 新建分支名也拦（不是"更新既有分支"才拦）。
- 解法：经典 PAT 勾 **`repo` + `workflow`**；或改用 **SSH 推送**（不受 PAT scope 限制）。
- 校验是否已生效：`curl -sI -H "Authorization: token $T" https://api.github.com/user | grep -i x-oauth-scopes`。

### 3. 顺带：`libs:check` 决定了分支必须基于新 dev
CI 第 66 行跑 `pnpm libs:check`。`skin-center` 与 `dsh-web-all` 都**入库 lib 产物**，
且 `dsh-web-all` 把 skin-center 的 `src/client` 内联进自己的 bundle。
所以改 skin-center 源码后必须 `pnpm build` + `node scripts/lib-artifact-check.mjs --write`
并把重建的 `lib/` 与 `scripts/lib-artifact-fingerprints.json` 一起提交；
**用旧基线重建的 lib 合并进 dev 后会立刻变 stale**，因此不能靠"旧基线 + lib"绕过推送限制。

- 仓库的 Issue 表单是 GitHub **form**（`.yml`），并有机器人校验「必填部分是否齐全」。
- **先用最小 payload 在真仓库试通道 → 机器人 14 秒后按「未使用模板」自动关闭**，且**API 无法重开**（`PATCH state=open` 返回空壳 422、`errors: []`）。
- 正确做法：**payload 一次写全**（按表单逐节），用 Node 生成 JSON 再 `curl --data-binary`（PS 5.1 的 `ConvertTo-Json` 会把 3 KB 中文膨胀到 450 KB → 422）。
- 手滑建了废条目时：它**删不掉**，在下面留言指向新条目即可。

## 九、issue 模板校验器的「标签竞态」（2026-09-14 实测）

**症状**：用 API 按 `bug_report.yml` 的九个 `###` 段全填好提交（`labels: ["bug"]`），
几秒后被 `issue-template-enforcer` 自动关闭，评论里**没有**「缺少或为空的必填部分」那行，
只有最后一句「Bug 报告必须附带 bug 标签」——即**九个段全认出来了，只差标签**。

**根因**（时间线取证）：

```
10:57:47  commented / assigned   by github-actions[bot]       ← 校验器开始
10:57:48  closed  reason=not_planned  by github-actions[bot]  ← 校验器关单
10:57:49  labeled  label=bug  by github-actions[bot]          ← auto-labeler 才补上
```

- **`labels` 参数对没有 push 权限的提交者会被静默忽略**（GitHub 文档：设置 issue 标签需要 push 权限）。
  我传了 `["bug"]`，创建时根本没挂上，时间线里也没有我这个作者的 `labeled` 事件。
- `bug` 是 `auto-label-issues` 从标题 `[Bug]: ` 推出来的，**晚 1 秒**。
- 校验器 `on: [opened, reopened]`，在 `opened` 那一刻查 `issue.labels`，必然查不到 → 必关。

**绕法**：改从 `standard_issue.yml` 那条路提交（`### Issue 类型` 写 `问题`），
标题仍以 `[Bug]: ` 开头让 auto-labeler 补标签，正文里照样附
`Bug 截图` / `冒烟测试` / `引用代码`（多写的段不影响校验）。
这样 `isBug = (类型==='bug 报告') || hasBugLabel` 在两种时序下都成立：
- 标签先到 → 走 bug 分支，九个段齐全 → 过；
- 标签后到 → 走非 bug 分支，只需六个基础段 → 也过。

**顺带记牢**：
- 重开被机器人关掉的 issue：`PATCH {"state":"open"}` → 422（`errors: []`）；
  GraphQL `reopenIssue` → `UNPROCESSABLE: Could not reopen the issue.`。**两条路都不通**，别浪费时间。
- 给自己提的 issue 补标签：`POST /issues/{n}/labels` → 403 `Must have admin rights`。同样不通。
- 所以被误关的 issue 只能重提一条（旧的删不掉，正文里说明「以本条为准」）。
- 结论：**本仓库当前只走 issue 流程**（维护者原话「目前不接修复PR」）——
  先提 issue 等邀请，别再直接提修复 PR。

