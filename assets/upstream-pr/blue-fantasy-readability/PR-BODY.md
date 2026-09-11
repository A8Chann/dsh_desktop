## 摘要（Summary）

blue-fantasy（蓝色幻想）的鲸鱼插画直接垫在对话之下，正文、工具输出与轮尾控件在画面上对比度不足。本 PR 给该皮肤的 L3 补丁层加一层**文字背后的毛玻璃托底**：只在文字所在块补底（块级元素收缩到内容宽度），其余区域保持插画可见；深色主题用深底。纯 CSS，无 hooks、无 DOM 改动，选择器只用稳定锚点（`data-slot` / `data-disclosure-row` / `data-turn-tail` / `data-chat-flow-kind` / `data-context-injection-body` / `data-system-prompt-body`）。

覆盖范围：markdown 段落/标题/引用/列表/代码块/表格、可展开行（think / bash / 上下文注入 / 系统提示词）、轮尾操作行（复制 / 反馈 / 重新生成）、「深度求索中…」状态行（用 `::before` 垫底以保住 shimmer）、展开正文（命令输出 / 推理 / 注入上下文）。

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
git fetch origin && git rebase origin/dev   # 分支基于 dev @ b33fbe2
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

使用的 AI 模型：DeepSeek（`deepseek-flash`，支持图像输入 —— 本次通过截图逐轮比对验证对比度与深浅色表现）

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
# 皮肤 CSS 过安全管线（force-scoping / @import / 越界路径）与内置皮肤校验
pnpm --filter @linxin666/dsh-client-ui-skin-center exec vitest run \
  tests/builtin-skins.spec.ts tests/css-safety.spec.ts

# 画廊产物重建 + 一致性校验
pnpm market:build
pnpm market:check

# 皮肤目录册与 hooks 注册表
pnpm skin-center:check

# 文档门禁
pnpm docs:check
```

结果摘要：

- `builtin-skins.spec.ts` + `css-safety.spec.ts` → **Test Files 2 passed / Tests 89 passed**
- `market:build` → `wrote 2508 files (31 skins, 7 pets, 58 plugins, 32 presets)`；`market:check` → `tryon/ verified against hash manifest (756 files)`、`dist up to date (1720 files)`
- `skin-center:check` → `skin-hooks-registry: check OK`、`check OK (31 repo catalog skins; package ships blue-fantasy only)`
- `docs:check` → `all documentation gates passed`

## 用户可见变更证据（Local Feature Evidence）

证据（浅色 / 深色，同一滚动位置）：

![light](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/blue-fantasy-alpha-nav/docs/archive/2026-09-11-blue-fantasy-alpha-nav/light.png)

![dark](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/blue-fantasy-alpha-nav/docs/archive/2026-09-11-blue-fantasy-alpha-nav/dark.png)

被本层覆盖的元素计数（实测，同一会话）：markdown 块 **36**、可展开行 **95**、状态行 **1**；暗色主题变体见 CSS（`body[data-ds-dark-theme]`）。

## 后续（同一分支的补充提交）

1. **α 联动滑块**：所有「文字背后的托底」透明度改为读 `var(--dsh-skin-bubble-alpha, .5)`，系数取「默认 50% 时正好复现上一版的固定值」（`.5→×1`、`.55→×1.1`、`.4→×.8`、`.45→×.9`）。于是「气泡不透明度」滑块统管这一层。亮色基色由纯白改为左栏实测的 `rgb(242 245 250)`，与周围面板同色系。
   - 实测：滑块 `0.8 → .8`、`0.2 → .2`、`0 → 全透明`。
2. **代码块补毛玻璃**：围栏代码在壳层里渲染成 `<div class="md-code-block">` 而**不是 `<pre>`**，所以原有的 markdown `> pre` 规则从未命中它 —— 它一直只有壳层自带底色、没有 backdrop 模糊。补上（外壳自带的填充色不动）。
3. **表格托底贴合内容宽度**：壳层的表格外层容器默认撑满整栏，而表格本身贴内容宽度，于是短表格旁边会空出约 **300px** 的空玻璃（实测托底 829px / 表格 528px）。把托底收成贴合表格（仍以整栏为上限）；宽表照旧由容器自身的 `overflow-x: auto` 横向滚动。
4. **回合过程行**（`N 次工具调用 · M 条消息`）：改挂语义属性 `data-turn-process`，不再用会连带命中其 label 与 chevron 的共享类名前缀（三层 `.5` 白叠成肉眼可见的「双层背景」）。顺带把上游那条分隔线的 `padding: 0 0 8px` + 底边框收成与相邻工具行一致的版式，分隔线仅在展开态保留。
5. **窗口外框**：顶部导航栏与右侧栏面板取左栏基色、**固定 0.75**（不跟随「气泡不透明度」——外框不是气泡）；导航栏内部各项继续保持无背景。

## 备注（Notes）

- 模糊读 `var(--dsh-skin-bubble-blur, 10px)`，透明度读 `var(--dsh-skin-bubble-alpha, .5)`：两个变量都由皮肤中心「背景」卡提供。前者皮肤中心**目前还没有**（见 [#1469](https://github.com/zhu1090093659/dsh-web/issues/1469) 的提案），此处靠兜底值生效；一旦那条落地，本层两个维度都可被用户调节，皮肤无需再改。
- 未包含的相邻改动（如需可另开）：工具调用行的多层背景去重（涉及 `callRow` / `o3BgMG_*` 等模块哈希，脆弱性较高）。

