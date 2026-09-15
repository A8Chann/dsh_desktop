# 蓝色幻想（blue-fantasy）皮肤 · patches.css 存档

`patches.css` 是**皮肤补丁层**（skin-center 会在加载时自动给每条选择器加上
`html[data-dsh-skin="blue-fantasy"] ` 前缀），也是我们这一路 UI 修补全部落地的文件。

## 为什么要在仓库里留一份

皮肤是从 **dsh-market.com** 安装的（`provenance.installed.0.2.0.json` 里记着安装时间与每个文件的
sha256）。也就是说 `~/.dsh/skins/blue-fantasy/patches.css` **不是我们独有的文件**：

- 从市场**更新/重装**这个皮肤 → `patches.css` 会被上游版整文件覆盖，我们所有修补一次性丢失；
- 上游改这个文件时**不会** bump `skin.json` 的 `version`（至今仍是 `0.2.0`），市场因此**不会**提示“有更新”，
  只能靠 sha256 比对发现（本目录 `upstream/notes.json` 记的就是基准）。

所以仓库里存三样东西：**我们的成品**、**上游基准**、**上游坐标**。

## 目录结构

| 文件 | 是什么 |
|---|---|
| `patches.css` | 我们的成品（= 上游基准 + 本地追加），`~/.dsh/skins/blue-fantasy/patches.css` 的镜像 |
| `skin.css` | **token 层**，同样是我们改过的成品（`~/.dsh/skins/blue-fantasy/skin.css` 的镜像）。⚠️ 它也是**上游文件**，市场更新会整文件覆盖 → 必须一并归档才能恢复 |
| `upstream/patches.base.css` | **上游基准**：最近一次已合入的上游修订原文（用于三方合并） |
| `upstream/notes.json` | 上游坐标：仓库/分支/路径、基准 commit 与 sha256、市场清单地址、已合入记录 |
| `provenance.installed.0.2.0.json` | 市场安装时留下的原始 provenance（含各文件安装时 sha256） |

> ⚠️ `scripts/skin-patches.ps1` 目前**只存档 `patches.css`**，改过 `skin.css` 后要手动同步/恢复这一份。
> 2026-09-15 起本皮肤在 `skin.css` 里有真实改动（tooltip 的两个 token，见下），所以这一条开始有关系了。

## 日常操作

> 本机只装了 **Windows PowerShell 5.1**（没有 `pwsh`），且 `scripts\skin-patches.ps1` 里带中文——
> 所以那个脚本存的是**带 UTF-8 BOM** 的版本（5.1 读无 BOM 的 UTF-8 会把中文读乱）。
> 用 **edit/write 工具改过脚本后，记得把 BOM 补回去**（字节级前置 `EF BB BF` 即可，不要用 PS 做文本往返）。

```powershell
# 看一眼：本地成品与仓库镜像是否一致
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\skin-patches.ps1 status

# 改了皮肤、要留底（~/.dsh → 仓库）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\skin-patches.ps1 save

# 皮肤被市场覆盖了、要还原（仓库 → ~/.dsh，会先备份现场）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\skin-patches.ps1 restore

# 查上游有没有新改动（比对 dev 分支 + 市场清单）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\skin-patches.ps1 check

# 把上游新改动三方合并进成品（ours=成品 base=上游基准 theirs=上游最新）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\skin-patches.ps1 merge          # 只看结果
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\skin-patches.ps1 merge -Apply   # 落盘（先备份，并推进基准）
```

`merge` 用 `git merge-file` **就地**合并（不把 git 的 stdout 管道回 PowerShell，避免中文注释在解码那一步被搞坏）；
结果落在 `%TEMP%\patches.merged.*.css`，有冲突就打印冲突行号并保持本地成品不动。

> 每次改完 `patches.css`，内容页要**重新加载**（⋯ → 重新加载）才生效——页面只在加载时拉一次皮肤 CSS。

## 上游基准沿革

- 安装时：`0f4d4890…`（= 上游 commit `eba552b2`，即 #1358 之前）
- 2026-09-10 合入：`3354134f…`（= 上游 commit `66d83fa`，`perf(skin-center): remove root backdrop-filter in blue-fantasy skin (#1358)`）
  - `.aionui-root` 从 `backdrop-filter: blur(12px)` 那条规则里摘掉
  - `[data-aionui-explorer-col], [data-aionui-preview-col]` 增加 `contain: paint`
- 合并后实测：本地上成品**完整包含**上游最新版全部非空行（缺失 0），其余 485 行是本地追加。

## 我们在这个文件里改了什么（摘要）

配色基准一律取左侧边栏 `[data-pane="sidebar"]` 的 computed `backgroundColor`（当前 `rgba(242,245,250,.75)`）。
具体条目见仓库根 `AGENTS.md` 的「皮肤补丁 patches.css」一节，含每个选择器、实测数值与踩坑记录。
