# 上游 PR 素材：给 skin-center 加「气泡模糊程度」滑块

这是**源码级补丁**（不是我在本地 `node_modules` 里打的那份产物级热修），可直接 `git apply` 到
[`zhu1090093659/dsh-web`](https://github.com/zhu1090093659/dsh-web) 的 `dev` 分支。

- 基线：上游 `dev` 分支，2026-09-10 快照（`packages/skins/skin-center/src/`，共 48 个源文件）
- 改动量：**5 个文件、70 行纯新增、0 行删除**
- 补丁：`0001-feat-skin-center-add-bubble-blur-control.patch`

## 为什么要这个功能

皮肤中心「背景」卡现有 5 个旋钮（背景遮挡 / 空对话背景模糊 / 有对话背景模糊 / 输入卡片模糊 /
气泡不透明度），**没有气泡模糊**。而皮肤侧已经用起了气泡相关的 CSS 变量约定
（`--dsh-skin-bubble-alpha`、whale-mom 的 `--dsw-specific-bubble`），于是：

- 想要「气泡更透」和「气泡更糊」是**两个独立诉求**：贴纸/插画壁纸下把不透明度拉到 0 时，
  不该连磨砂一起消失；反之想要磨砂但保持底色时也没法只调一个。
- 上游自己就有同类先例：[#783「皮肤中心新增『输入卡模糊』滑杆」] 这种提案被接受并实现，
  本次只是把它对称地补到气泡这一层。

## 契约（与既有旋钮保持一致）

| 项 | 值 |
|---|---|
| 设置字段 | `bubbleBlur`（命名空间 `skin-background`，与 `bubbleOpacity` 同级） |
| 范围 / 步进 | `0–20` px，step `1` |
| 默认值 | **10** px（= 既有「输入卡片模糊」的默认档，采用该变量的皮肤观感不变） |
| 写到 body 的变量 | `--dsh-skin-bubble-blur`（px 字符串，如 `10px`），与 `--dsh-skin-bubble-alpha` 并列 |
| 可选性 | **可选变量**：皮肤不读它 → 零变化；宿主侧是纯新增字段 → 老配置照常加载，无迁移 |
| 主开关行为 | 皮肤中心关闭时移除该变量（与 `--dsh-skin-bubble-alpha` 一致） |

皮肤侧用法（blue-fantasy 本地已在用）：

```css
backdrop-filter: blur(var(--dsh-skin-bubble-blur, 10px)) saturate(1.3);
```

## 5 个文件各改了什么

| 文件 | 改动 |
|---|---|
| `src/core/background.ts` | `SkinBackgroundConfig` 加可选字段 `bubbleBlur`；`SKIN_BACKGROUND_DEFAULTS` 加 `bubbleBlur: 10`；`RANGES` 加 `[0, 20]`（自动覆盖 normalize / sanitize / resolve / FIELDS） |
| `src/index.ts` | zod schema 加 `bubbleBlur: z.number().min(0).max(20).step(1).default(...)` |
| `src/client/background.ts` | `BUBBLE_BLUR_FIELD` / `BUBBLE_BLUR_VAR` / `DEFAULT_BUBBLE_BLUR` 常量；`SkinBackgroundHandle` 加 `bubbleBlur()` / `setBubbleBlur()`；`BackgroundController` 加字段、构造/init/setEnabled 的 apply 调用、`snapshot()`、getter/setter、`dispose()` 清理、`assign()`、`applyBubbleBlur()` |
| `src/client/SkinCenter.tsx` | `useSyncExternalStore` + `useLiveValue` 两个 hook；在「气泡不透明度」行之后加第 6 个 `SliderControl`（`id="skin-center-bubble-blur"`，0–20，aria `Npx`） |
| `src/client/locales.ts` | `SkinCenterKey` 联合类型加两个键；`en` / `zh` 词典各加 `bubbleBlur` + `bubbleBlurHint`（该文件目前只有这两份词典） |

> 触点完整性：在**全部 48 个源文件**里检索 `bubbleOpacity` / `BUBBLE_ALPHA` / `inputCardBlur`，
> 除上述 5 个文件外**没有任何其它引用** → 该字段不需要在别处（迁移、legacy bridge、boot 等）额外接线。

## 怎么用

```bash
# 1) fork 后克隆上游
git clone https://github.com/<你>/dsh-web.git && cd dsh-web
git checkout dev
git checkout -b feat/skin-center-bubble-blur

# 2) 应用补丁（本文件同目录）
git apply /path/to/0001-feat-skin-center-add-bubble-blur-control.patch
#    冲突时用 --3way；应用后再 review 一遍 diff

# 3) 过门禁（仓库自带）
pnpm install          # 首次
make check            # typecheck → lint → build → test → check:consumer-types

# 4) 提交 + 推送 + 开 PR
git add -A
git commit -m "feat(skin-center): add bubble blur control"
git push -u origin feat/skin-center-bubble-blur
```

建议先开一个 **Proposal issue**（照 #783 的格式：动机 → 契约 → 默认值理由 → 可用性/零破坏性 → 截图），
维护者点头后再提 PR。

### 提交信息 / PR 描述要点

- 标题：`feat(skin-center): add bubble blur control`（本地化风格：`feat(skin-center): 新增「气泡模糊程度」滑杆`）
- 关键论点：
  1. **可选变量 + 纯新增字段**，老配置无迁移、皮肤不读则零变化；
  2. 默认 **10px** 与既有「输入卡片模糊」默认档一致，采用它的皮肤观感不变；
  3. 与 `--dsh-skin-bubble-alpha` **相互独立**（贴纸壁纸下"更透"与"更糊"是两个诉求）；
  4. 对称补齐 #783 已建立的产品面（第 6 个背景旋钮，位置紧邻「气泡不透明度」）；
  5. 验证：`make check`；i18n 已补 `en` / `zh`。

## 与本地热修的关系（重要）

本地 `~/.dsh/profiles/web/node_modules/...` 的那份是**产物级**补丁
（`scripts/skin-center-bubble-blur.mjs` 改的是打包后的 `lib/client.js`，因为要在自己的环境里立刻生效）。
**上游 PR 合入并发版后，应当撤掉本地那份**，避免同一功能两处实现、后续升级互相打架：

```powershell
node scripts\skin-center-bubble-blur.mjs --revert
```

## 本补丁的验证记录

- ✅ `git apply --check` 在全新上游快照上通过；实际应用后改动文件数 = 5
- ✅ 三个 `.ts` 文件通过 `node --check`（Node 的类型剥离语法检查）
- ⚠️ **未在本机跑上游的 `make check` / 单测**（没有 clone 整个 monorepo 装依赖）—— 提交前请在上游仓库里跑一次
