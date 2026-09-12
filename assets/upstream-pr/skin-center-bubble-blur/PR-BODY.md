## 摘要（Summary）

皮肤中心的「背景」卡现在有 5 个旋钮（背景遮挡 / 空对话背景模糊 / 有对话背景模糊 / 输入卡片模糊 / 气泡不透明度），**没有气泡模糊**。

「气泡更透」与「气泡更糊」是两个独立诉求：插画 / 贴纸壁纸下把不透明度调到 0 时，不该连磨砂一起消失；只想调磨砂、保留底色时也没有入口。

本 PR 对称补齐 [#783](https://github.com/zhu1090093659/dsh-web/issues/783)（输入卡模糊滑杆）建立的产品面，在「气泡不透明度」旁边新增第 6 个旋钮。按 [#1469](https://github.com/zhu1090093659/dsh-web/issues/1469) 的讨论结论提交。

The Background card ships five knobs today and no bubble blur. Bubble transparency and bubble blur are independent wishes: with an illustrated wallpaper, pushing opacity to 0 must not take the frosting with it. This adds the sixth knob next to “Bubble opacity”, mirroring the product surface #783 established for the input card.

## 涉及包（Affected Packages）

- [ ] 任务看板 `packages/dsh-task-board`
- [ ] Git 图谱 `packages/dsh-git-graph`
- [ ] 右侧面板 `packages/dsh-aionui-panel`
- [ ] 远程 Web UI `packages/dsh-remote-web-ui`
- [ ] SSH 远程运维 `packages/dsh-ssh`
- [ ] 宠物 `packages/dsh-pet`
- [ ] 预设中心 `packages/dsh-preset-center`
- [x] 皮肤 / 皮肤中心 `packages/dsh-skins` / `packages/skins`
- [x] 聚合包 / 设置 `packages/dsh-web-all` / `packages/dsh-web-settings`
- [ ] 其他（请说明）

> 聚合包勾选原因：skin-center 的 `src/client` 被内联进 `dsh-web-all` 的客户端 bundle，因此本次一并重建并提交了 `packages/dsh-web-all/lib/`。

## PR 类别（PR Category）

- [ ] 壁纸 / 渲染器（Wallpaper Engine / WebGL / 背景场景）
- [x] 皮肤 / 皮肤中心（新皮肤收录、皮肤样式）
- [ ] 插件功能（任务看板 / Git 图谱 / 右侧面板 / 远程 Web UI / SSH / 宠物 / 预设中心 / 设置 / 聚合包）
- [ ] 社区插件索引
- [ ] 维护 / 其他

## PR 类型（PR Type）

- [x] 面向用户的功能或行为变更
- [ ] Bug 修复
- [ ] 视觉修复（UI / 视觉类问题的修复）
- [x] 增强 / 优化（现有功能的改进、性能 / 体验优化）
- [ ] 新皮肤收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 新宠物收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 新预设收录（内容贡献，欢迎直接提交，无需先提 issue）
- [ ] 维护 / 重构

## 最新代码确认（Latest Codebase Confirmation）

- [x] 我已基于最新 `dev` 分支开发，或在提交前已 rebase / 合并最新 `dev`。

同步命令：

```bash
git fetch upstream dev
git rebase --onto upstream/dev b33fbe2 feat/skin-center-bubble-blur   # dev @ d56328c
```

## 测试证据与上游同步（Test Evidence & Upstream Sync）

- [x] 我提供了自己本地测试的证据（执行的命令 / 测试结果 / 运行截图）。
- [x] 我已同步上游最新 `dev` 分支（`git fetch origin && git rebase origin/dev`），并附上同步后重新测试通过的证据（视觉 / 用户可见变更附截图）。

## AI 编码披露（AI Coding Disclosure）

- [x] 完全 AI 编码：全部编程改动由 AI 产出，并由贡献者接受 / 审查。
- [ ] 部分 AI 辅助：AI 帮助编写或修改了部分编程改动。
- [ ] 未使用 AI 编码辅助。

使用的 AI 模型：DeepSeek（`deepseek-v4.1-flash`，多模态，支持图像输入 —— 本次视觉证据由无头浏览器截图 + computed style 实测产出并逐项核对）

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
pnpm --filter @linxin666/dsh-client-ui-skin-center test
pnpm --filter @linxin666/dsh-client-ui-skin-center typecheck
pnpm --filter @linxin666/dsh-web-all test
pnpm --filter @linxin666/dsh-client-ui-skin-center --filter @linxin666/dsh-web-all build
node scripts/lib-artifact-check.mjs --write
pnpm libs:check && pnpm docs:check && pnpm i18n:check && pnpm aggregate:check
pnpm market:check && pnpm skin-center:check && pnpm skin-hooks:check
pnpm sync-shared:check && pnpm runtime-deps:check && pnpm test:scripts
```

结果摘要：

以下全部通过。

- `skin-center` test → **39 files / 639 tests passed**；typecheck → clean（exit 0）
- `dsh-web-all` test → **6 files / 38 passed | 8 skipped**
- `libs:check` → **OK (4 committed lib/ packages match their sources)**（已提交重建的 `lib/` + 指纹清单）
- `docs:check` / `i18n:check` / `aggregate:check` / `market:check` / `skin-center:check` / `skin-hooks:check` / `sync-shared:check` / `runtime-deps:check` → 全部 exit 0
- `test:scripts` → **pass 280 / fail 0**

界面实测（无头 Edge，设置 → 皮肤 → 背景卡）：

| 项 | 实测 |
| --- | --- |
| 旋钮 `#skin-center-bubble-blur` | `input[type=range]`，min 0 / max 20 / step 1，默认 **10**，aria「气泡模糊程度」 |
| 第 6 个旋钮位置 | 紧邻「气泡不透明度」，背景卡共 6 个 range |
| 默认态 | body `--dsh-skin-bubble-blur: 10px` |
| 拖到 18 | body 变量 → `18px`，真实元素实测 `blur(18px) saturate(1.3)` |
| **与透明度解耦** | 不透明度 = 0 时 `--dsh-skin-bubble-alpha: 0`，而 `--dsh-skin-bubble-blur` 仍为 `18px` —— 透明度归零不会带走磨砂 |

## 用户可见变更证据（Local Feature Evidence）

背景卡全貌（含新增的第 6 个旋钮「气泡模糊程度」）：

![default](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/skin-center-bubble-blur/docs/archive/2026-09-13-skin-center-bubble-blur/default-10px.png)

拖到 18px：

![blur18](https://raw.githubusercontent.com/A8Chann/dsh-web/feat/skin-center-bubble-blur/docs/archive/2026-09-13-skin-center-bubble-blur/blur-18px.png)

## 契约补充（Contract）

| | |
| --- | --- |
| 设置字段 | `bubbleBlur`（命名空间 `skin-background`，与 `bubbleOpacity` 同级） |
| 范围 / 步进 | `0–20` px，step `1` |
| 默认 | **10** px（采用该变量的皮肤观感不变） |
| body 变量 | `--dsh-skin-bubble-blur`（如 `10px`），与 `--dsh-skin-bubble-alpha` 并列 |
| 可选性 | 皮肤不读该变量 = 零变化；宿主侧为纯新增可选字段，旧配置无需迁移 |
| 主开关 | 皮肤中心关闭时移除该变量（与既有 alpha 一致） |

皮肤侧用法（即使没有本 PR，靠 fallback 也成立）：

```css
backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
```

## 改动清单（Changes）

- `src/core/background.ts` — `SkinBackgroundConfig.bubbleBlur?`、默认值、`RANGES`（自动覆盖 normalize / sanitize / resolve / FIELDS）
- `src/index.ts` — zod schema
- `src/client/background.ts` — `BUBBLE_BLUR_FIELD` / `BUBBLE_BLUR_VAR` / `DEFAULT_BUBBLE_BLUR`；`SkinBackgroundHandle` 的两个方法；控制器字段、构造与 `init()` 应用、`snapshot()`、getter、setter、`dispose()` 清理、`assign()`、`applyBubbleBlur()`
- `src/client/index.ts` — injected handle 门面（漏这里 typecheck 会以 TS2739 失败，组件运行时也会崩）
- `src/client/SkinCenter.tsx` — 两个 hook + 第 6 个 `SliderControl`（`id="skin-center-bubble-blur"`）
- `src/client/locales.ts` — 键联合类型 + `en` / `zh` 词条
- 测试 — `background.spec.ts`（新增 “applies, persists, and cleans up message bubble blur”，并补默认值与 snapshot 断言）、`background-migration.spec.ts`、`background-scope.spec.ts`、`skin-center-custom-theme.spec.tsx`（handle mock + 滑块交互用例）、`routes-v2.spec.ts`（区间夹紧覆盖）
- 重建产物 — `packages/skins/skin-center/lib/`、`packages/dsh-web-all/lib/`、`scripts/lib-artifact-fingerprints.json`

## 备注（Notes）

- 该变量按**可选契约**设计：皮肤照上面那行写即可接住（whale-mom 已在用同模式的 `--dsh-skin-bubble-alpha`）。蓝幻想皮肤侧消费它的可读性补丁已另行提交。
- 已同步上游最新 `dev`（`d56328c`）后再跑的全部测试与门禁。
