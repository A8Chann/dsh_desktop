## 摘要（Summary）

把输入区配件与顶栏**接到正确的变量**上：它们之前读的是「气泡」那套滑杆（或根本没读）。

**输入区配件**（统计行，以及 `dsh-cost-meter` 的「本会话」「Go 5h」小胶囊）属于**配件**而不是文字托底，现在读：

- 模糊 → `--dsh-input-card-blur`，与输入卡同一个滑杆
- 底色 → `--dsw-specific-input-major` 的 **alpha**，叠在本皮肤自己的基色上

用相对颜色语法**只借 alpha**：整个 token 引用会连颜色一起拿走，而这里要的基色是左侧栏那个色。又因为该 token 的 alpha 本身就是壁纸遮挡算出来的，这些行**无需第二个公式**就与输入卡同步。文字托底继续读 `--dsh-skin-bubble-*`，两套变量到此互相独立。

另外壳层的 accessory 规则会把 composer dock 的每个直接子元素刷满，统计行因此被拉成通栏；现在收成贴内容宽度。「Go 5h」那条 strip 是外层壳、底色由里面的胶囊承担，所以 strip 自身那层是多余的，清掉。

**顶栏**改用左侧栏自己的填充 token，并跟着壁纸遮挡走，取代原先写死的 0.75 —— 它是 chrome，画被压暗时它应当更实（遮挡 0 → 0.95，遮挡 1 → 0.45）。该 token 自带主题，所以单独那条深色规则撤销。**右侧栏的填充保持原样不动。**

`producedRow` 补 `box-sizing: border-box`，让内边距留在 fit-content 宽度内。

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
git fetch upstream dev && git checkout -B feat/blue-fantasy-accessories-and-chrome upstream/dev   # dev @ 1cbd28c
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

使用的 AI 模型：DeepSeek（`deepseek-v4.1-flash`，多模态，支持图像输入 —— 本轮的数值与截图都由无头浏览器实测产出）

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
pnpm skin-center:check && pnpm docs:check && pnpm libs:check
```

结果摘要：

全部通过（exit 0）。

- `market:build` → `wrote 3068 files (35 skins, 7 pets, 60 plugins, 32 presets)`；`market:check` → `dist up to date (2276 files)`
- `skin-center:check` → `check OK (35 repo catalog skins)`
- `docs:check` → `all documentation gates passed`
- `libs:check` → `OK (4 committed lib/ packages match their sources)`

无头浏览器实测（换入本 PR 的 patches.css，**一次只动一个变量**）：

| 变量 | 顶栏 alpha | 配件行 | 文字托底 |
| --- | --- | --- | --- |
| 遮挡 0 / 0.5 / 1 | 0.95 / 0.70 / 0.45 | 底色固定 `rgb(242,245,250)`，alpha 1 / 0.8 / 0.6 | 0.5 **不变** |
| 暗色主题 遮挡 0.5 | 0.70，`rgb(29,37,57)` | `rgb(29,37,57)`，alpha 0.8235 | `rgba(16,22,42,0.4)` **不变** |
| 输入卡模糊 | — | `blur(10px)` 跟随 | 仍跟气泡模糊滑杆 |

即：**拖「背景遮挡」只动顶栏与配件行，拖「气泡模糊程度 / 气泡不透明度」只动文字托底**。

## 用户可见变更证据（Local Feature Evidence）

![chrome-and-accessories](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/blue-fantasy-accessories-and-chrome/docs/archive/2026-09-14-blue-fantasy-accessories-and-chrome/chrome-and-accessories.png)

## 备注（Notes）

- `cm-*` 类名来自社区插件 **`dsh-cost-meter`**（统计行与「Go 5h」胶囊）。皮肤为它做适配，
  如果上游不希望皮肤引用第三方插件类名，这部分可以单独摘掉，其余改动独立可用。
- 本 PR 只改 `packages/skins/skin-center/skins/blue-fantasy/patches.css` 一个文件。
- 另有两个 PR：一个修操作行 hover 气泡跑飞（`backdrop-filter` 劫持 `position: fixed` 包含块），
  一个修浮层配色与面板层次（气泡 / hover 卡 / 队列 dock / 答题卡 / 右栏与底部面板）。
  三者互不重叠。
