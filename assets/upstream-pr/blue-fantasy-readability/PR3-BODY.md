## 摘要（Summary）

blue-fantasy 的操作行（复制 / 点赞 / 点踩 / 重新生成）托底顺带修掉了一个**真实 bug**：

壳层把每个图标的 hover 提示气泡（`position: fixed` 的 span）渲染在**操作行内部**，而
`backdrop-filter` 会让元素成为 `position: fixed` 后代的**包含块** —— 于是这条行上的托底
把气泡的定位基准从「视口」改成了「这一行」，气泡飞到几百像素之外。

实测：按钮在 `546,496`，气泡落在 `1030,1028`（**视口高只有 874，气泡直接掉到屏幕外**），
相距 719px；修复后气泡回到 `527,532`，距按钮 41px、正常贴在按钮下方。

改法：托底改由 `::before` 绘制（`inset: 0` 与行盒完全重合），行自身不再沾 `backdrop-filter`。
`padding: 0 8px` 与 `width: fit-content` **保持本规则原有取值**，所以托底尺寸、行几何、
图标位置全部不变。

## 涉及包（Affected Packages）

- [ ] 任务看板 `packages/dsh-task-board`
- [ ] Git 图谱 `packages/dsh-git-graph`
- [ ] 右侧面板 `packages/dsh-aionui-panel`
- [ ] 远程 Web UI `packages/dsh-remote-web-ui`
- [ ] SSH 远程运维 `packages/dsh-ssh`
- [ ] 宠物 `packages/dsh-pet`
- [ ] 预设中心 `packages/dsh-preset-center`
- [x] 皮肤 / 皮肤中心 `packages/dsh-skins` / `packages/skins`
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
- [x] Bug 修复
- [x] 视觉修复（UI / 视觉类问题的修复）
- [ ] 增强 / 优化（现有功能的改进、性能 / 体验优化）
- [ ] 新皮肤收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 新宠物收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 新预设收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 维护 / 重构

> 这是对已合并的 [#1468](https://github.com/zhu1090093659/dsh-web/pull/1468) / [#1476](https://github.com/zhu1090093659/dsh-web/pull/1476) 带入的 blue-fantasy 皮肤样式的修复。

## 最新代码确认（Latest Codebase Confirmation）

- [x] 我已基于最新 `dev` 分支开发，或在提交前已 rebase / 合并最新 `dev`。

同步命令：

```bash
git fetch upstream dev && git checkout -B fix/blue-fantasy-actions-tooltip upstream/dev   # dev @ 1cbd28c
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

使用的 AI 模型：DeepSeek（`deepseek-v4.1-flash`，多模态，支持图像输入 —— 本轮的定位数据与前后对比图都由无头浏览器实测产出）

使用的编码 Agent 工具：DeepSeek Harness

## 仓库规范检查（Repo Rules）

- [x] 未修改 DSH 官方源码，仅基于官方 NPM SDK（`@deepseek-ai/*`）开发。
- [x] 未新增指向 DSH 源码 checkout 的 tsconfig `extends` / `paths` / `references`。
- [x] 新增包目录以 `dsh-` 前缀命名（本 PR 未新增包）。
- [x] 所有新增 / 修改文件不含任何 emoji 字符。
- [x] 改动包 README 时同步维护中英双语三件套（本 PR 未改 README；`pnpm docs:check` 通过）。

## 本地验证（Local Validation）

执行的命令：

```bash
pnpm market:build && pnpm market:check
pnpm skin-center:check && pnpm docs:check && pnpm libs:check && pnpm aggregate:check
```

结果摘要：

全部通过（exit 0）。

- `market:build` → `wrote 3068 files (35 skins, 7 pets, 60 plugins, 32 presets)`；`market:check` → `tryon/ verified against hash manifest (756 files)`、`dist up to date (2276 files)`
- `skin-center:check` → `check OK (35 repo catalog skins)`
- `docs:check` → `all documentation gates passed`
- `libs:check` → `OK (4 committed lib/ packages match their sources)`
- `aggregate:check` → `check OK: packages\dsh-web-all`

无头浏览器实测（同一会话、同一滚动位置，唯一差别是这一条规则的版本）：

| 项 | 修复前（上游 dev） | 修复后 |
| --- | --- | --- |
| 操作行自身 `backdrop-filter` | `blur(10px) saturate(1.3)` | **`none`** |
| `::before` 托底 | 无 | `inset: 0` + `blur(10px) saturate(1.3)` + 同一底色 |
| 按钮位置 | `546,496` | `546,496`（不变） |
| 提示气泡位置 | `1030,1028` | **`527,532`** |
| 气泡与按钮距离 | **719px** | **41px** |
| 行几何 / 图标位置 | — | 与修复前逐像素一致（`padding: 0 8px`、`width: fit-content` 未动） |

## 用户可见变更证据（Local Feature Evidence）

**修复前 / Before** —— 按钮在左上、气泡被推到视口外（截图里根本看不到它，
它落在 `1030,1028`，而视口只有 874 高）：

![before](https://raw.githubusercontent.com/A8Chann/dsh-web/fix/blue-fantasy-actions-tooltip/docs/archive/2026-09-14-blue-fantasy-tooltip-anchor/before.png)

**修复后 / After** —— 气泡回到按钮正下方（`527,532`，距按钮 41px）：

![after](https://raw.githubusercontent.com/A8Chann/dsh-web/fix/blue-fantasy-actions-tooltip/docs/archive/2026-09-14-blue-fantasy-tooltip-anchor/after.png)

## 备注（Notes）

- 触发条件：blue-fantasy 皮肤 + 鼠标悬停在回合尾部的复制 / 点赞 / 点踩 / 重新生成按钮上。
  其它皮肤不受影响（这条规则来自 blue-fantasy 的可读性层）。
- 根因是 CSS 规范行为：`backdrop-filter`（以及 `transform` / `filter` / `contain: paint`）
  会让元素成为 `position: fixed` 后代的包含块。写皮肤时若某个元素内部会渲染浮层，
  这类属性应挪到 `::before` 上而不是元素自身。
- 本 PR **只动这一处**：没有顺手改 `padding` / `width`，也没有改其它元素。
