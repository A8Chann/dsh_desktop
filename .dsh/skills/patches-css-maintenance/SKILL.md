---
name: patches-css-maintenance
description: >
  patches.css 的存档、上游更新比对与合并、市场覆盖机制。
---

# patches.css 存档与上游更新

## 文件性质
- `patches.css` 是**市场安装件**：`~/.dsh/skins/blue-fantasy/` 来自 dsh-market，安装时留下的 `dsh-market.provenance.json` 记着**每个文件的 sha256**。
- 从市场**更新/重装皮肤会把 `patches.css` 整文件覆盖**。
- 上游改它时**不 bump `skin.json` 的 version**（至今 `0.2.0`），市场因此**不会**提示「有更新」——只能靠 sha256 比对发现。

## 仓库存档结构
- `assets/skins/blue-fantasy/`
  - 成品 `patches.css`
  - 上游基准 `upstream/patches.base.css`
  - 上游坐标与沿革 `upstream/notes.json`
  - 安装时 provenance 快照
- 时间戳快照：`~/.dsh/_skin-backups/blue-fantasy/`
- 配套脚本 `scripts/skin-patches.ps1`，命令：`status` / `save` / `restore` / `check` / `merge [-Apply]`

## 上游坐标
- 上游 = `github.com/zhu1090093659/dsh-web` 的 **`dev` 分支** `packages/skins/skin-center/skins/blue-fantasy/`
- **市场文件与该分支逐字节相同**，等价可取 `https://dsh-market.com/assets/skins/blue-fantasy/patches.css`
- 清单：`https://dsh-market.com/manifest/skins.json`（作者 `powerdog996（DreamSkin 社区）· dsh-web 适配`）

## 已合入的上游更新（2026-09-10）
- commit `66d83fa`「perf(skin-center): remove root backdrop-filter in blue-fantasy skin (#1358)」
- 内容：`.aionui-root` 从 `backdrop-filter: blur(12px)` 规则里摘掉、给 `[data-aionui-explorer-col], [data-aionui-preview-col]` 加 `contain: paint`（**就这 2 行**）。
- 合并后实测：成品完整包含上游全部 207 条非空行（缺失 0），其余 ~485 行是本地追加。

## 判定「本地 = 上游 + 本地追加」的快速方法（不改文件）
把上游文件逐行 trim 后与本地逐行比对，数「上游有、本地没有」的行——为 0 即完全包含。

⚠️ **但行级比对会误报**：上游的注释是**英文**、且会把相关规则**合并成一条**（如把 markdown 深色
的 `p/h/blockquote/ul/pre` 合成一条），本地是中文注释 + 拆分写法。此时行级比对会报「缺失几十行」，
其实功能完全等价。**要判真假得用规则级比对**：解析出 `{selector, body}` 后按选择器归一化对比
（`scripts/` 里没有常驻脚本，需要时现写一个；关键三步：剥注释→按 `{}` 配对切规则→
`selector.replace(/\s*([,:>])\s*/g,'$1')` 归一）。真正要看的只有两种：**① 上游有、本地完全没有该选择器**（=真缺失）
**② 选择器相同但规则体不同**（通常就是"本地领先、已提 PR 等合入"的那批）。

## 双向同步：PR 与本地皮肤（2026-09-11 实战流程）
本地皮肤是上游的超集，所以改动经常要**两边走**。推荐顺序（踩过的坑都在里面）：

1. **先改本地**验证观感（F5 即可，无需 PR）；
2. 挑出**适合上游**的子集提 PR —— 注意剔除三类：绕第三方插件（cost-meter 的 `cm-*` 类名）、
   与外壳渲染层 `!important` 打架的、审美取向过强（如整页毛玻璃化）；
3. **上游有改动后，把 PR 的形态回流本地**（不要让两份写法漂移）：
   - 先做**规则级**三向对比（本地 / 上游 dev / PR 分支），只挑真正不同的规则；
   - **加作用域**（`[data-slot="main.conversation"]`）与**语义锚点**这两点最值得回流——
     它们是防误伤的，本地同样受益；
   - 回流后**实测**（无头 Edge）：确认会话内仍命中、**会话外命中数为 0**。
4. 真实案例（2026-09-11 两轮）：
   - 第一轮 4 项：亮色基色改 `rgb(242 245 250)`、代码块毛玻璃、表格托底收窄、外框固定 0.75；
   - 第二轮 3 项：`callRow`/`producedRow` 加作用域、用户气泡跟随 α（`calc(alpha * 120%)`，
     默认 .5 时正好是原 60%）。
   - 两轮都出现**「本地早就有这条规则」**的情况（本地是超集）——所以回流时**只改选择器/公式的差异部分**，
     别整段覆盖。

## 已合入的上游更新（2026-09-11）
- PR [#1468](https://github.com/zhu1090093659/dsh-web/pull/1468)：可读性层首版（markdown/表格/disclosure/状态行/展开正文），**已合并**。
- PR [#1476](https://github.com/zhu1090093659/dsh-web/pull/1476)：联动滑杆 + 代码块 + 表格托底 + 外框 + 工具行/气泡/产出行，**open**（CI 全绿）。
- issue [#1469](https://github.com/zhu1090093659/dsh-web/issues/1469)：气泡模糊滑杆提案，等回音。

## PowerShell 5.1 注意事项
- 本机只有 Windows PowerShell 5.1（无 `pwsh`）：含中文的 `.ps1` 必须存成**带 UTF-8 BOM**，而 write/edit 工具写出的是**无 BOM**。
- 每次用工具改完 `scripts\skin-patches.ps1` 都要按字节补回 `EF BB BF`（用 `[System.IO.File]::WriteAllBytes`，**不要**用 PS 做文本往返）。
- 脚本内部刻意**不把 git 的 stdout 管道回 PowerShell**（`git merge-file` 就地合并），避开中文注释在 stdio 解码环节被搞坏。
