param([string]$Shot = '', [switch]$SendF6, [int]$Wait = 0)
Add-Type -Namespace F6 -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc cb, IntPtr p);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
public delegate bool EnumProc(IntPtr h, IntPtr p);
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

$hwnd = [F6.W]::FindWindow('Tauri Window', 'DSH Desktop')
if ($hwnd -eq [IntPtr]::Zero) { $hwnd = [F6.W]::FindWindow($null, 'DSH Desktop') }
"main hwnd = $hwnd"
if ($hwnd -eq [IntPtr]::Zero) { exit 1 }
$pid0 = 0
[void][F6.W]::GetWindowThreadProcessId($hwnd, [ref]$pid0)
"main pid  = $pid0"
"visible   = $([F6.W]::IsWindowVisible($hwnd))  iconic = $([F6.W]::IsIconic($hwnd))"
"foreground= $([F6.W]::GetForegroundWindow())"
$r = New-Object F6.W+RECT
[void][F6.W]::GetWindowRect($hwnd, [ref]$r)
"rect      = $($r.L),$($r.T) $($r.R - $r.L)x$($r.B - $r.T)"
$script:rect = $r

$list = New-Object System.Collections.ArrayList
$cb = [F6.W+EnumProc]{
  param($h, $p)
  $cn = New-Object System.Text.StringBuilder 256
  [void][F6.W]::GetClassName($h, $cn, 256)
  $tt = New-Object System.Text.StringBuilder 256
  [void][F6.W]::GetWindowTextW($h, $tt, 256)
  $rr = New-Object F6.W+RECT
  [void][F6.W]::GetWindowRect($h, [ref]$rr)
  $cp = 0
  [void][F6.W]::GetWindowThreadProcessId($h, [ref]$cp)
  [void]$list.Add(("  hwnd={0} pid={1} vis={2} cls={3} title='{4}' rect={5},{6} {7}x{8}" -f `
    $h, $cp, [F6.W]::IsWindowVisible($h), $cn.ToString(), $tt.ToString(), $rr.L, $rr.T, ($rr.R - $rr.L), ($rr.B - $rr.T)))
  return $true
}
[void][F6.W]::EnumChildWindows($hwnd, $cb, [IntPtr]::Zero)
"---- children: $($list.Count) ----"
$list | ForEach-Object { $_ }

function Shot([string]$path) {
  $rr = $script:rect
  $w = $rr.R - $rr.L; $h = $rr.B - $rr.T
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($rr.L, $rr.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $g.Dispose()
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "saved $path"
}

if ($Shot -ne '') { Shot $Shot }

if ($SendF6) {
  [void][F6.W]::SetForegroundWindow($hwnd)
  Start-Sleep -Milliseconds 400
  [System.Windows.Forms.SendKeys]::SendWait('{F6}')
  "sent F6"
  if ($Wait -gt 0) { Start-Sleep -Milliseconds $Wait }
  if ($Shot -ne '') { Shot ($Shot -replace '\.png$', '-after.png') }
}
