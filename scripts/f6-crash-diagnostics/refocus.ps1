Add-Type -Namespace R -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc cb, IntPtr p);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool InvalidateRect(IntPtr h, IntPtr r, bool erase);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
public delegate bool EnumProc(IntPtr h, IntPtr p);
'@
Add-Type -AssemblyName System.Drawing

function Count-Kids {
  $main = [R.W]::FindWindow('Tauri Window', 'DSH Desktop')
  $list = New-Object System.Collections.ArrayList
  $cb = [R.W+EnumProc]{
    param($h, $p)
    $cn = New-Object System.Text.StringBuilder 160
    [void][R.W]::GetClassName($h, $cn, 160)
    $cp = 0
    [void][R.W]::GetWindowThreadProcessId($h, [ref]$cp)
    [void]$list.Add("$($cn.ToString())|$cp|vis=$([R.W]::IsWindowVisible($h))")
    return $true
  }
  [void][R.W]::EnumChildWindows($main, $cb, [IntPtr]::Zero)
  return ,$list
}

"--- BEFORE ---"
$before = Count-Kids
"descendants=$($before.Count)"
$before | ForEach-Object { "  $_" }

$main = [R.W]::FindWindow('Tauri Window', 'DSH Desktop')
"foreground before = $([R.W]::GetForegroundWindow())"
[void][R.W]::ShowWindow($main, 9)   # SW_RESTORE
[void][R.W]::SetForegroundWindow($main)
Start-Sleep -Milliseconds 1200
"foreground after  = $([R.W]::GetForegroundWindow())"

$r = New-Object R.W+RECT
[void][R.W]::GetWindowRect($main, [ref]$r)
$w = $r.R - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$g.Dispose()
$bmp.Save('D:\HTML\DSH_Desktop\.tmp-f6\after-focus.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"saved after-focus.png"

"--- AFTER ---"
$after = Count-Kids
"descendants=$($after.Count)"
$after | ForEach-Object { "  $_" }
