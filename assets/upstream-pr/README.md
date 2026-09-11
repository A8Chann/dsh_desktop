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

## 六、本目录内容

| 文件 | 说明 |
|---|---|
| `skin-center-bubble-blur/0001-…patch` | 可直接 `git apply` 的源码补丁（**11 文件 / +109 −3**，含 5 个测试文件） |
| `skin-center-bubble-blur/ISSUE-BODY.md` | 已提交的 Issue 正文（= [#1469]） |
| `skin-center-bubble-blur/PR-BODY.md` | 备好的 PR 正文（等维护者点头后可直接用） |
| `blue-fantasy-readability/readability-subset.css` | 提交给上游的 166 行可读性子集（固定 alpha + 语义锚点） |
| `blue-fantasy-readability/PR-BODY.md` | 已提交的 PR 正文（= [#1468]） |

遗留：上游合入并发布后，记得撤掉本地那份**产物级**热修 —— `node scripts\skin-center-bubble-blur.mjs --revert`，避免同一功能两处实现。
