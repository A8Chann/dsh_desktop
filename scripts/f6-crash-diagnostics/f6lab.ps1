param([int]$Minutes = 8, [int]$IntervalSec = 6, [int]$WarmupSec = 45)
Add-Type -Namespace F -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc cb, IntPtr p);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
public delegate bool EnumProc(IntPtr h, IntPtr p);
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

$log = 'D:\HTML\DSH_Desktop\.tmp-f6\f6lab.log'
function W([string]$m) { $line = "$((Get-Date).ToString('HH:mm:ss')) $m"; $line | Out-File -FilePath $log -Append -Encoding UTF8; $line }

function WVCount {
  $procs = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -like '*DSH-Desktop-2.8.0-tauri*' }
  return @($procs).Count
}

function ContentKids {
  $main = [F.W]::FindWindow('Tauri Window', 'DSH Desktop')
  if ($main -eq [IntPtr]::Zero) { return 'no-window' }
  $n = 0
  $cb = [F.W+EnumProc]{
    param($h, $p)
    $cn = New-Object System.Text.StringBuilder 128
    [void][F.W]::GetClassName($h, $cn, 128)
    if ($cn.ToString() -eq 'WRY_WEBVIEW') { $script:n++ }
    return $true
  }
  [void][F.W]::EnumChildWindows($main, $cb, [IntPtr]::Zero)
  return $n
}

W "LAB START warmup=${WarmupSec}s interval=${IntervalSec}s minutes=$Minutes wv=$((WVCount))"
Start-Sleep -Seconds $WarmupSec

$main = [F.W]::FindWindow('Tauri Window', 'DSH Desktop')
W "main hwnd=$main wv=$((WVCount))"
[void][F.W]::SetForegroundWindow($main)
Start-Sleep -Milliseconds 500

$deadline = (Get-Date).AddMinutes($Minutes)
$shots = 0
$lastCount = WVCount
W "baseline wv=$lastCount"
while ((Get-Date) -lt $deadline) {
  [System.Windows.Forms.SendKeys]::SendWait('{F6}')
  Start-Sleep -Milliseconds 250
  $c = WVCount
  if ($c -ne $lastCount) {
    W "WV COUNT CHANGED $lastCount -> $c (after F6)"
    $lastCount = $c
  }
  Start-Sleep -Seconds $IntervalSec
  $hots = [F.W]::GetForegroundWindow()
  if ($hots -ne $main) { [void][F.W]::SetForegroundWindow($main); Start-Sleep -Milliseconds 250 }
}
W "LAB END wv=$lastCount"
"saved $log"
