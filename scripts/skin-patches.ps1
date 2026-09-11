# 蓝色幻想皮肤 patches.css 的存档 / 同步 / 上游更新检查
#
#   status   本地 ~/.dsh 与仓库镜像是否一致（默认动作）
#   save     本地 → 仓库（改完皮肤要留底）
#   restore  仓库 → 本地（皮肤被市场覆盖后还原，会先备份现场）
#   check    上游有没有新改动（dsh-web dev 分支 + 市场清单）
#   merge    把上游新改动三方合并进成品（ours=成品, base=上游基准, theirs=上游最新）
#
# 设计要点：所有文件搬运走 Copy-Item（逐字节），需要读文本时显式 -Encoding UTF8，
# 三方合并交给 `git merge-file` 就地进行——**不把 git 的 stdout 管道回 PowerShell**，
# 避免中文注释在 stdio 解码那一步被搞坏（本机 console 编码不是 UTF-8）。
[CmdletBinding()]
param(
  [ValidateSet('status', 'save', 'restore', 'check', 'merge')]
  [string]$Action = 'status',
  [string]$Skin = 'blue-fantasy',
  [switch]$Apply
)

$ErrorActionPreference = 'Stop'

$RepoRoot  = Split-Path -Parent $PSScriptRoot
$LiveDir   = Join-Path $env:USERPROFILE ".dsh\skins\$Skin"
$LiveFile  = Join-Path $LiveDir 'patches.css'
$RepoDir   = Join-Path $RepoRoot "assets\skins\$Skin"
$RepoFile  = Join-Path $RepoDir 'patches.css'
$BaseFile  = Join-Path $RepoDir 'upstream\patches.base.css'
$NotesFile = Join-Path $RepoDir 'upstream\notes.json'
$BackupDir = Join-Path $env:USERPROFILE ".dsh\_skin-backups\$Skin"

function Say($msg) { Write-Host "[skin-patches] $msg" }
function Sha($path) { (Get-FileHash $path -Algorithm SHA256).Hash.ToLower() }
function Lines($path) { (Get-Content $path -Encoding UTF8).Count }
function NonEmpty($path) { Get-Content $path -Encoding UTF8 | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' } }

function BackupLive {
  if (-not (Test-Path $LiveFile)) { return }
  New-Item -ItemType Directory -Force -Path $BackupDir | Out-Null
  $dest = Join-Path $BackupDir ("patches.{0}.css" -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
  Copy-Item $LiveFile $dest -Force
  Say "现场已备份 → $dest"
}

function Get-Upstream($rawBase, $outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
  $targets = @{
    'patches.css' = (Join-Path $outDir 'patches.upstream.css')
    'skin.json'   = (Join-Path $outDir 'skin.upstream.json')
  }
  foreach ($name in $targets.Keys) {
    $url = "$rawBase/$name"
    try { Invoke-WebRequest -UseBasicParsing $url -OutFile $targets[$name]; Say "拉取 $name ← $url" }
    catch { throw "拉取 $name 失败：$($_.Exception.Message)" }
  }
  return $targets
}

switch ($Action) {

  'status' {
    foreach ($p in @($LiveFile, $RepoFile)) {
      if (-not (Test-Path $p)) { Say "缺少文件：$p"; exit 1 }
    }
    $lh = Sha $LiveFile; $rh = Sha $RepoFile
    $lw = (Get-Item $LiveFile).LastWriteTime; $rw = (Get-Item $RepoFile).LastWriteTime
    Say ("本地  {0}  {1} 行  {2}" -f $lh, (Lines $LiveFile), $lw)
    Say ("仓库  {0}  {1} 行  {2}" -f $rh, (Lines $RepoFile), $rw)
    if ($lh -eq $rh) { Say '一致 ✅（仓库镜像 = 生效文件）' }
    else {
      Say '不一致 ⚠️'
      $newer = if ($lw -gt $rw) { '本地较新 → 跑 save' } else { '仓库较新 → 跑 restore' }
      Say "  $newer"
    }
    if (Test-Path $BaseFile) { Say ("上游基准  {0}  {1} 行" -f (Sha $BaseFile), (Lines $BaseFile)) }
  }

  'save' {
    if (-not (Test-Path $LiveFile)) { Say "找不到生效文件：$LiveFile"; exit 1 }
    Copy-Item $LiveFile $RepoFile -Force
    BackupLive
    Say ("已保存 → {0}" -f $RepoFile)
    Say ("sha256 {0}  {1} 行" -f (Sha $RepoFile), (Lines $RepoFile))
    Say '提示：改动只在仓库工作区，需要时用 git 提交。'
  }

  'restore' {
    if (-not (Test-Path $RepoFile)) { Say "仓库里没有成品：$RepoFile"; exit 1 }
    BackupLive
    Copy-Item $RepoFile $LiveFile -Force
    Say ("已还原 → {0}" -f $LiveFile)
    Say ("sha256 {0}  {1} 行" -f (Sha $LiveFile), (Lines $LiveFile))
    Say '提示：内容页需要「⋯ → 重新加载」才会生效。'
  }

  'check' {
    if (-not (Test-Path $NotesFile)) { Say "缺少上游坐标：$NotesFile"; exit 1 }
    $notes = Get-Content $NotesFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $rawBase = $notes.upstream.rawBase
    $tmp = Join-Path $env:TEMP ("skinup-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $files = Get-Upstream $rawBase $tmp

    $upVer = (Get-Content $files['skin.json'] -Raw -Encoding UTF8 | ConvertFrom-Json).version
    Say ("上游 skin.json 版本：{0}（安装时 {1}）" -f $upVer, $notes.installedVersion)

    $upHash = Sha $files['patches.css']
    $baseHash = Sha $BaseFile
    Say ("上游 patches.css {0}  {1} 行" -f $upHash, (Lines $files['patches.css']))
    Say ("本地基准         {0}  {1} 行" -f $baseHash, (Lines $BaseFile))

    if ($upHash -eq $baseHash) {
      Say '上游无新改动 ✅'
    }
    else {
      Say '上游有改动 ⚠️（注意：上游改这个文件不会 bump version，市场也不会提示）'
      $upLines = NonEmpty $files['patches.css']; $baseLines = NonEmpty $BaseFile
      $added = $upLines | Where-Object { $baseLines -notcontains $_ }
      $removed = $baseLines | Where-Object { $upLines -notcontains $_ }
      Say ("  上游新增 {0} 行 / 删除 {1} 行" -f $added.Count, $removed.Count)
      $added | Select-Object -First 15 | ForEach-Object { Say "    + $_" }
      $removed | Select-Object -First 15 | ForEach-Object { Say "    - $_" }
      Say '  下一步：pwsh scripts\skin-patches.ps1 merge（只看）/ merge -Apply（落盘）'
    }

    $liveLines = NonEmpty $LiveFile
    $pending = (NonEmpty $files['patches.css']) | Where-Object { $liveLines -notcontains $_ }
    Say ("成品里缺失的上游行：{0}" -f $pending.Count)
    $pending | Select-Object -First 15 | ForEach-Object { Say "    ! $_" }
    Remove-Item $tmp -Recurse -Force
  }

  'merge' {
    if (-not (Test-Path $NotesFile)) { Say "缺少上游坐标：$NotesFile"; exit 1 }
    if (-not (Test-Path $BaseFile)) { Say "缺少上游基准：$BaseFile"; exit 1 }
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Say '需要 git（三方合并用 git merge-file）'; exit 1 }
    $notes = Get-Content $NotesFile -Raw -Encoding UTF8 | ConvertFrom-Json
    $tmp = Join-Path $env:TEMP ("skinmerge-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $files = Get-Upstream $notes.upstream.rawBase $tmp

    $ours = Join-Path $tmp 'ours.css'
    Copy-Item $LiveFile $ours -Force
    & git merge-file -L '本地成品' -L '上游基准' -L '上游最新' $ours $BaseFile $files['patches.css']
    $code = $LASTEXITCODE
    Say ("git merge-file 退出码：{0}（0=干净，>0=冲突行数）" -f $code)

    $mergedOut = Join-Path $env:TEMP ("patches.merged.{0}.css" -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
    Copy-Item $ours $mergedOut -Force
    Say "合并结果 → $mergedOut（临时文件，仅用于查看/存档）"

    if ($code -gt 0) {
      Say '有冲突 ⚠️ 保留标记（<<<<<<< / ======= / >>>>>>>）供人工处理——本地成品未改动。'
      Select-String -Path $ours -Pattern '^(<<<<<<<|=======|>>>>>>>)' -Encoding UTF8 |
        Select-Object -First 10 | ForEach-Object { Say ("    {0}: {1}" -f $_.LineNumber, $_.Line) }
      Remove-Item $tmp -Recurse -Force
      exit 1
    }

    $changed = (Sha $ours) -ne (Sha $LiveFile)
    if (-not $changed) { Say '合并后与当前成品相同，无需落盘。'; Remove-Item $tmp -Recurse -Force; exit 0 }
    Say ("合并后 {0} 行（当前成品 {1} 行）" -f (Lines $ours), (Lines $LiveFile))

    if (-not $Apply) { Say '这是预演；加 -Apply 才会写回本地并把上游基准推进到最新。'; Remove-Item $tmp -Recurse -Force; exit 0 }

    BackupLive
    Copy-Item $ours $LiveFile -Force
    Copy-Item $files['patches.css'] $BaseFile -Force
    Copy-Item $LiveFile $RepoFile -Force
    Say '已落盘：本地成品 + 仓库镜像已更新，上游基准已推进 ✅'
    Say ("  成品 {0}" -f (Sha $LiveFile))
    Say ("  基准 {0}" -f (Sha $BaseFile))
    Say '提示：① 内容页重新加载；② 把 upstream/notes.json 的 base 段与 merged 记录更新为最新 commit。'
    Remove-Item $tmp -Recurse -Force
  }
}
