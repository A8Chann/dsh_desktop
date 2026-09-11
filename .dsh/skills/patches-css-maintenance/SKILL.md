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

## PowerShell 5.1 注意事项
- 本机只有 Windows PowerShell 5.1（无 `pwsh`）：含中文的 `.ps1` 必须存成**带 UTF-8 BOM**，而 write/edit 工具写出的是**无 BOM**。
- 每次用工具改完 `scripts\skin-patches.ps1` 都要按字节补回 `EF BB BF`（用 `[System.IO.File]::WriteAllBytes`，**不要**用 PS 做文本往返）。
- 脚本内部刻意**不把 git 的 stdout 管道回 PowerShell**（`git merge-file` 就地合并），避开中文注释在 stdio 解码环节被搞坏。
