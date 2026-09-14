## 摘要（Summary）

把 blue-fantasy 里**浮层与面板**的几处问题一并修掉，都是壳层留给皮肤、但皮肤没接上的部分：

1. **提示气泡配色** —— 壳层的 `--dsw-alias-tooltip-bg` 解析成同一个**淡黄 `#ffffe1`（亮暗主题都不变）**，在这套皮肤上是全屏唯一的暖色块，也不响应暗色主题。改用皮肤自己的面板色 + 一道发丝边。
2. **侧边栏 hover 卡片配色** —— 壳层故意做了深卡：把 `--dsw-hovercard-bg` 以 `#2C2C2E` 写在卡片元素自身上，配白/灰文字。在浅色皮肤上就是一块近黑。卡片和它的三行文字一起改（只改底色会让白字压在浅底上）。注意这张卡的 role 是 `button` 不是 `tooltip`，所以上面那条规则从没覆盖到它。
3. **对话队列 dock** —— 皮肤外壳的 accessory 规则会给 composer dock 的**每个直接子元素**上底，队列 dock 就是其中之一；而队列自己的 panel 已经有底，于是成了两层；又因为 dock 保留了 inset 内边距、panel 在其内侧，那层底每边还比 panel 宽 8px。
4. **答题卡 / 计划复核卡** —— 这两张卡只设了底色，所以挨着毛玻璃输入卡时是死平面。底色本身半透明，补上模糊即可，读的是输入卡同一个滑杆。
5. **面板层次** —— 右侧栏改用 `--dsw-alias-bg-layer-1`（官方 `bg-base` 太透，`rgba(255,255,255,0.225)`，画会透出来）；分割线改用左栏那条 `--dsw-alias-border-l3`（官方给右栏的是 `-l4`，深一档，两侧不一致）；并且**只保留一层** —— panel 内部嵌了一个尺寸几乎相同、类名同样含 `panel` 的 `panelBody`，壳层那条合并规则同时命中两层。底部面板展开时右边缘会压住这根竖线导致断层，补一条同色右边框。

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
git fetch upstream dev && git checkout -B fix/blue-fantasy-panels-and-overlays upstream/dev   # dev @ 1cbd28c
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

使用的 AI 模型：DeepSeek（`deepseek-v4.1-flash`，多模态，支持图像输入 —— 本轮的定位数据与截图都由无头浏览器实测产出）

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

- `market:build` → `wrote 3068 files (35 skins, 7 pets, 60 plugins, 32 presets)`；`market:check` → `tryon/ verified against hash manifest (756 files)`、`dist up to date (2276 files)`
- `skin-center:check` → `check OK (35 repo catalog skins)`
- `docs:check` → `all documentation gates passed`
- `libs:check` → `OK (4 committed lib/ packages match their sources)`

无头浏览器实测（把本 PR 的 patches.css 换进运行中的皮肤后取的 computed 值）：

| 项 | 修复前 | 修复后 |
| --- | --- | --- |
| 提示气泡底色 | `#ffffe1`（亮暗同值） | `rgba(242,245,250,0.92)` / 暗 `rgba(16,22,42,0.92)` |
| 提示气泡文字 | 深色固定 | `rgb(29,37,57)` / 暗 `rgb(219,226,242)`，加 0.5px 发丝边 |
| hover 卡片底色 | `rgb(44,44,46)`（近黑） | `rgba(242,245,250,0.96)`，标题 `rgb(29,37,57)`、时间/状态 `rgb(90,106,140)` |
| 右侧栏底色 | `rgba(255,255,255,0.225)` | `rgba(243,245,251,0.75)`（= `bg-layer-1`） |
| 右侧栏分割线 | `rgba(44,58,115,0.42)`（`-l4`） | `rgba(44,58,115,0.32)`（= 左栏 `-l3`） |
| 右侧栏内层 `panelBody` | 也有一层 0.75 底（叠加后约 0.94） | 透明（**只剩一层**） |
| 底部面板 | 无右边框 → 交界处竖线断层 | `border-right: 1px`，宽度 587px 不变 |

## 用户可见变更证据（Local Feature Evidence）

**侧边栏 hover 卡片** —— 从近黑改为皮肤冷色卡（标题/时间/状态三行同步）：

![hovercard](https://raw.githubusercontent.com/A8Chann/dsh-web/fix/blue-fantasy-panels-and-overlays/docs/archive/2026-09-14-blue-fantasy-overlays-and-panels/hovercard.png)

提示气泡、右侧栏 / 底部面板的数值见上表（同一无头会话里逐项读取 computed 值）。

## 备注（Notes）

- 这几处都属于「壳层留了 token / 留了结构，皮肤没接」的情况，改动集中在
  `packages/skins/skin-center/skins/blue-fantasy/patches.css` 一个文件。
- 第 5 项里 `[class*="panel"]` 同时命中 `panel` 与 `panelBody` 是**子串匹配**的经典误伤，
  壳层自己的合并规则也踩了同一个坑；这里在皮肤侧补一条内层重置，不动壳层。
- 另有一个 PR 处理操作行 hover 气泡跑飞（`backdrop-filter` 劫持 `position: fixed` 的包含块），
  以及一个 PR 处理输入区配件的变量归属，三者互不重叠。
