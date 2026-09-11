## 摘要（Summary）

承接已合并的 [#1468](https://github.com/zhu1090093659/dsh-web/pull/1468)（blue-fantasy 插画背景下的文字可读性层），把同一层补到剩余几类**仍直接压在插画上**的行：

1. **透明度联动「气泡不透明度」滑杆** —— 所有文字托底的 α 改读 `var(--dsh-skin-bubble-alpha, .5)`，系数取「默认 50% 时正好复现上一版的固定值」（`.5→×1`、`.55→×1.1`、`.4→×.8`、`.45→×.9`）；亮色基色由纯白改为左栏实测的 `rgb(242 245 250)`，与周围面板同色系。
2. **代码块补毛玻璃** —— 围栏代码在壳层里渲染成 `<div class="md-code-block">` 而**不是 `<pre>`**，所以上一版的 markdown `> pre` 规则从未命中它，它一直只有壳层自带底色、没有 backdrop 模糊。
3. **表格托底贴合内容宽度** —— 壳层表格外层容器默认撑满整栏，而表格本身贴内容宽度，短表格旁边会空出约 **300px** 的空玻璃（实测托底 829px / 表格 528px → 收窄后 548 / 528）。
4. **回合过程行去双层** —— `N 次工具调用 · M 条消息` 原挂在会连带命中 label/chevron 的共享类名前缀上，三层半透明白叠成肉眼可见的双层背景。
5. **窗口外框底色** —— 顶部导航栏与右侧栏面板取左栏基色、固定 0.75（不跟随气泡滑杆；外框不是气泡）。
6. **工具调用行：一行只留一层托底** —— 工具行是「wrapper → 行本体 → 可展开标题行」三层近乎同尺寸的嵌套，三层都上底会叠出「两层背景」。改为只给最外层 wrapper 上底，并把内两层显式重置（带 `[data-slot="conversation.session"]` 与深色主题前缀以压过既有规则的特异性）。
7. **用户气泡** —— 保留皮肤原有的强调色填充，加半透明并给透过来的插画加模糊；浓度同样跟随滑杆（取 120%，默认 50% 时正好是 60%）。
8. **「本次产出」行** —— 它在 turn tail 里、之前没有任何托底。宽度**不能**用裸 `fit-content`：父级是 `display:contents` 的插槽宿主（宽度算得 0），否则文件 chip 会被挤成 3 行；改为行内级 shrink-to-fit 盒（`display:inline-flex` + `align-self:flex-start`）。

> 本地实验里另有两条**没有**采纳：① 给 composer seat 加 `backdrop-filter`（skin-center 自己的 scene neutralizer 已用 `backdrop-filter: none !important` 清掉它、把模糊交给 `--dsh-input-card-blur` 的输入卡，规则实际不生效）；② 一整批同时给工具行内外层都上底的「补漏」规则（正是上面第 6 条要撤销的形状）。


## 涉及包（Affected Packages）

- [x] 皮肤 / 皮肤中心 `packages/dsh-skins` / `packages/skins`
- [ ] 任务看板 `packages/dsh-task-board`
- [ ] Git 图谱 `packages/dsh-git-graph`
- [ ] 右侧面板 `packages/dsh-aionui-panel`
- [ ] 远程 Web UI `packages/dsh-remote-web-ui`
- [ ] SSH 远程运维 `packages/dsh-ssh`
- [ ] 宠物 `packages/dsh-pet`
- [ ] 预设中心 `packages/dsh-preset-center`
- [ ] 聚合包 / 设置 `packages/dsh-web-all` / `packages/dsh-web-settings`
- [ ] 其他（请说明）

## PR 类别（PR Category）

- [ ] 壁纸 / 渲染器（Wallpaper Engine / WebGL / 背景场景）
- [x] 皮肤 / 皮肤中心（新皮肤收录、皮肤样式）
- [ ] 插件功能（任务看板 / Git 图谱 / 右侧面板 / 远程 Web UI / SSH / 宠物 / 预设中心 / 设置 / 聚合包）
- [ ] 社区插件索引
- [ ] 维护 / 其他

## PR 类型（PR Type）

- [ ] 面向用户的功能或行为变更
- [ ] Bug 修复
- [x] 视觉修复（UI / 视觉类问题的修复）
- [x] 增强 / 优化（现有功能的改进、性能 / 体验优化）
- [ ] 新皮肤收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 新宠物收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 新预设收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 维护 / 重构

## 最新代码确认（Latest Codebase Confirmation）

- [x] 我已基于最新 `dev` 分支开发，或在提交前已 rebase / 合并最新 `dev`。

同步命令：

```bash
git fetch upstream dev && git checkout -B feat/blue-fantasy-alpha-nav upstream/dev   # dev @ 33ce09c（含已合并的 #1468）
```

## 测试证据与上游同步（Test Evidence & Upstream Sync）

- [x] 我提供了自己本地测试的证据（执行的命令 / 测试结果 / 运行截图）。
- [x] 我已同步上游最新 `dev` 分支（`git fetch origin && git rebase origin/dev`），并附上同步后重新测试通过的证据（视觉 / 用户可见变更附截图）。

## 视觉修复要求（Visual Fix Requirements）

- [x] 我提供了修复完成后的截图（完成态或修复前后对比）。
- [x] 修复使用的 AI 模型支持图像输入（多模态模型）；未使用 AI 编码时此项视为满足。

## AI 编码披露（AI Coding Disclosure）

- [x] 完全 AI 编码：全部编程改动由 AI 产出，并由贡献者接受 / 审查。
- [ ] 部分 AI 辅助：AI 帮助编写或修改了部分编程改动。
- [ ] 未使用 AI 编码辅助。

使用的 AI 模型：DeepSeek（`deepseek-flash`，支持图像输入 —— 本轮通过无头浏览器逐项测量 computed style 与截图比对确认）

使用的编码 Agent 工具：DeepSeek Harness（DSH Desktop 内置本地 agent）

## 仓库规范检查（Repo Rules）

- [x] 未修改 DSH 官方源码，仅基于官方 NPM SDK（`@deepseek-ai/*`）开发。
- [x] 未新增指向 DSH 源码 checkout 的 tsconfig `extends` / `paths` / `references`。
- [x] 新增包目录以 `dsh-` 前缀命名（本 PR 未新增包）。
- [x] 所有新增 / 修改文件不含任何 emoji 字符。
- [x] 改动包 README 时同步维护中英双语三件套（本 PR 未改 README；`pnpm docs:check` 已通过）。

## 本地验证（Local Validation）

执行的命令：

```bash
pnpm --filter @linxin666/dsh-client-ui-skin-center exec vitest run \
  tests/builtin-skins.spec.ts tests/css-safety.spec.ts
pnpm market:build && pnpm market:check
pnpm skin-center:check
pnpm docs:check
```

结果摘要：

- `builtin-skins.spec.ts` + `css-safety.spec.ts` → **Test Files 2 passed / Tests 90 passed**
- `market:build` → `wrote 2532 files (32 skins, 7 pets, 58 plugins, 32 presets)`；`market:check` → `tryon/ verified against hash manifest (756 files)`、`dist up to date (1743 files)`
- `skin-center:check` → `skin-hooks-registry: check OK`、`check OK (32 repo catalog skins; package ships blue-fantasy only)`
- `docs:check` → `all documentation gates passed`

数值测量（无头 Edge，同一会话）：

| 项 | 改前 | 改后 |
| --- | --- | --- |
| 代码块 `backdrop-filter` | `none` | `blur(10px) saturate(1.3)` |
| 表格托底宽度 | 829px（整栏） | **548px**（表格 528px + 内边距） |
| 回合过程行带底色的后代 | 2（label / chevron 各吃一层） | **0**（只剩按钮自身一层） |
| 工具行内层带底色的元素 | 2（行本体 + 标题行各一层） | **0**（只剩最外层 wrapper 一层） |
| 「本次产出」行 | 无托底 | 511×32 有托底 + 模糊 |
| 用户气泡 α | 固定 | 跟随（默认 `.6`、α=1 → 不透明、α=0 → 透明） |
| 顶部导航栏 / 右侧栏 α 随滑杆 | 跟随（0.5→0.75 等） | **固定 0.75** |
| 正文托底 α 随滑杆 | 固定 | 跟随（`0.8→.8`、`0.2→.2`、`0→全透明`） |

## 用户可见变更证据（Local Feature Evidence）

证据（浅色 / 深色，同一滚动位置；主题由 `body[data-ds-dark-theme]` 切换）：

![light](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/blue-fantasy-alpha-nav/docs/archive/2026-09-11-blue-fantasy-alpha-nav/light.png)

![dark](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/blue-fantasy-alpha-nav/docs/archive/2026-09-11-blue-fantasy-alpha-nav/dark.png)

图中可见：表格托底贴合内容宽度（右侧不再空出一条玻璃）、代码块文字下方有毛玻璃、顶部导航栏有底色且不随气泡滑杆变化。

## 备注（Notes）

- 两个变量都由皮肤中心「背景」卡提供：`--dsh-skin-bubble-blur`（**皮肤中心目前还没有**，见 [#1469](https://github.com/zhu1090093659/dsh-web/issues/1469) 的提案）与 `--dsh-skin-bubble-alpha`（已有）。本层对两者都用兜底值，因此**不依赖** #1469 先合并。
- 本轮把窗口外框刻意排除在气泡滑杆之外：外框是框架不是气泡，拖动滑杆不应把导航栏一起淡掉。
