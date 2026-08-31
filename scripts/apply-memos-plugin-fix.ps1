# apply-memos-plugin-fix.ps1
# ---------------------------------------------------------------------------
# 把仓库内的 memos-cloud-dsh-plugin 兼容修复（assets/memos-cloud-dsh-plugin-fixed）
# 部署到 $DSH_HOME/profiles/web，并对齐 package.json 的 link: 依赖，然后重新 pnpm install。
#
# 背景：@memtensor/memos-cloud-dsh-plugin@0.1.0 使用 @deepseek-ai/dsh-settings 旧 API
# （installSettingsSection / settingsNamespace），而 dsh 0.1.2-alpha.2 随附的
# dsh-settings 已改为 SettingsProvider#installSection(owner, ns, schema, entry, hooks)。
# 上游（npm 0.1.0 / GitHub MemTensor/MemOS-Cloud-Dsh-Plugin、MemOS-Cloud-OpenClaw-Plugin）
# 截至 2026-08-16 无修复版本，故在仓库内固化一份打过补丁的 0.1.0 源码。
#
# 何时需要重跑：dsh plugin add/remove 或任何会重建 profile node_modules 的操作之后
# （pnpm 会重新链接依赖；虽然 link: 依赖在 package.json 中持久存在，但 vendor 目录内容
# 若被删除/覆盖，重跑本脚本即可恢复）。
#
# 用法：pwsh scripts\apply-memos-plugin-fix.ps1
# ---------------------------------------------------------------------------
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$assetSrc = Join-Path $repoRoot "assets\memos-cloud-dsh-plugin-fixed"

$dshHome = if ($env:DSH_HOME) { $env:DSH_HOME } else { Join-Path $env:USERPROFILE ".dsh" }
$profileDir = Join-Path $dshHome "profiles\web"
$vendorDir = Join-Path $profileDir "vendor\memos-cloud-dsh-plugin"

if (-not (Test-Path $assetSrc)) { throw "fixed plugin source not found: $assetSrc" }
if (-not (Test-Path $profileDir)) { throw "web profile not found: $profileDir (run 'dsh web' once to initialize)" }

Write-Host "[1/4] copy fixed plugin -> $vendorDir"
New-Item -ItemType Directory -Force -Path (Split-Path $vendorDir) | Out-Null
Copy-Item "$assetSrc\*" $vendorDir -Recurse -Force

# 用 node 改 profile package.json（PowerShell 的文本往返会破坏中文/编码，见 AGENTS.md）
Write-Host "[2/4] ensure package.json dependency: @memtensor/memos-cloud-dsh-plugin = link:./vendor/memos-cloud-dsh-plugin"
& node -e @"
const fs = require('fs');
const p = process.argv[1];
const pkg = JSON.parse(fs.readFileSync(p, 'utf8'));
pkg.dependencies ??= {};
pkg.dependencies['@memtensor/memos-cloud-dsh-plugin'] = 'link:./vendor/memos-cloud-dsh-plugin';
fs.writeFileSync(p, JSON.stringify(pkg, null, 2) + '\n');
"@ (Join-Path $profileDir "package.json")
if ($LASTEXITCODE -ne 0) { throw "failed to update profile package.json" }

Write-Host "[3/4] pnpm install in $profileDir"
Push-Location $profileDir
try { pnpm install; if ($LASTEXITCODE -ne 0) { throw "pnpm install failed" } }
finally { Pop-Location }

Write-Host "[4/4] verify: dump-config should contain memos-cloud entry"
Push-Location $profileDir
try {
  $env:DSH_HOME = $dshHome
  $npmRoot = (& npm root -g).Trim()
  $dshBin = Join-Path $npmRoot "@deepseek-ai\dsh\lib\bin.js"
  if (-not (Test-Path $dshBin)) { throw "dsh bin.js not found: $dshBin" }
  & node $dshBin web --dump-config *> "$env:TEMP\dsh-web-dump-verify.txt"
  if ($LASTEXITCODE -ne 0) { throw "dump-config failed (exit $LASTEXITCODE); see $env:TEMP\dsh-web-dump-verify.txt" }
  Select-String -Path "$env:TEMP\dsh-web-dump-verify.txt" -Pattern "memos-cloud" | Select-Object -First 2 | ForEach-Object { Write-Host ("    " + $_.Line) }
  Write-Host "OK: web profile configuration composes (memos-cloud entry present)."
}
finally { Pop-Location }

Write-Host "Done. Restart the dsh backend (desktop: 'restart', or close/reopen) to boot the web profile."
