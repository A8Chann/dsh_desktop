param([int]$Seconds = 420, [int]$IntervalMs = 400)
Add-Type -Namespace M -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc cb, IntPtr p);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
public delegate bool EnumProc(IntPtr h, IntPtr p);
'@

$logPath = 'D:\HTML\DSH_Desktop\.tmp-f6\monitor.log'
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$deadline = $sw.Elapsed.TotalSeconds + $Seconds

function Snapshot {
  $main = [M.W]::FindWindow('Tauri Window', 'DSH Desktop')
  if ($main -eq [IntPtr]::Zero) { return 'MAIN_GONE' }
  $parts = New-Object System.Collections.ArrayList
  $cb = [M.W+EnumProc]{
    param($h, $p)
    $cn = New-Object System.Text.StringBuilder 128
    [void][M.W]::GetClassName($h, $cn, 128)
    if ($cn.ToString() -eq 'WRY_WEBVIEW') {
      $rr = New-Object M.W+RECT
      [void][M.W]::GetWindowRect($h, [ref]$rr)
      [void]$parts.Add(("h{0}:{1}@{2},{3} {4}x{5}" -f $h, $(if ([M.W]::IsWindowVisible($h)) { 'V' } else { 'h' }), $rr.L, $rr.T, ($rr.R - $rr.L), ($rr.B - $rr.T)))
    }
    return $true
  }
  [void][M.W]::EnumChildWindows($main, $cb, [IntPtr]::Zero)
  $fg = [M.W]::GetForegroundWindow()
  return ("main=$main fg=$fg | " + ($parts -join ' | '))
}

$last = ''
$changes = 0
$first = $true
while ($sw.Elapsed.TotalSeconds -lt $deadline) {
  $snap = Snapshot
  if ($first -or $snap -ne $last) {
    $changes++
    $stamp = (Get-Date).ToString('HH:mm:ss.fff')
    "$stamp CHANGE#$changes $snap" | Out-File -FilePath $logPath -Append -Encoding UTF8
    $last = $snap
    $first = $false
    # capture a window screenshot on every change
    if ($changes -gt 1) {
      try {
        Add-Type -AssemblyName System.Drawing
        $main = [M.W]::FindWindow('Tauri Window', 'DSH Desktop')
        $rr = New-Object M.W+RECT
        [void][M.W]::GetWindowRect($main, [ref]$rr)
        $w = $rr.R - $rr.L; $h = $rr.B - $rr.T
        if ($w -gt 0 -and $h -gt 0) {
          $bmp = New-Object System.Drawing.Bitmap $w, $h
          $g = [System.Drawing.Graphics]::FromImage($bmp)
          $g.CopyFromScreen($rr.L, $rr.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
          $g.Dispose()
          $bmp.Save(("D:\HTML\DSH_Desktop\.tmp-f6\chg-{0:D3}.png" -f $changes), [System.Drawing.Imaging.ImageFormat]::Png)
          $bmp.Dispose()
        }
      } catch { "$stamp shot-fail $($_.Exception.Message)" | Out-File -FilePath $logPath -Append -Encoding UTF8 }
    }
  }
  Start-Sleep -Milliseconds $IntervalMs
}
"$((Get-Date).ToString('HH:mm:ss.fff')) MONITOR END changes=$changes" | Out-File -FilePath $logPath -Append -Encoding UTF8
"done, changes=$changes, log=$logPath"
