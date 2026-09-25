Add-Type -Namespace D -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc cb, IntPtr p);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder sb, int max);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
public delegate bool EnumProc(IntPtr h, IntPtr p);
'@

$main = [D.W]::FindWindow('Tauri Window', 'DSH Desktop')
"main=$main"
$rows = New-Object System.Collections.ArrayList
$cb = [D.W+EnumProc]{
  param($h, $p)
  $cn = New-Object System.Text.StringBuilder 160
  [void][D.W]::GetClassName($h, $cn, 160)
  $tt = New-Object System.Text.StringBuilder 200
  [void][D.W]::GetWindowTextW($h, $tt, 200)
  $rr = New-Object D.W+RECT
  [void][D.W]::GetWindowRect($h, [ref]$rr)
  $cp = 0
  [void][D.W]::GetWindowThreadProcessId($h, [ref]$cp)
  [void]$rows.Add([pscustomobject]@{
    hwnd = $h; pid = $cp; vis = [D.W]::IsWindowVisible($h); parent = [D.W]::GetParent($h)
    cls = $cn.ToString(); title = $tt.ToString()
    rect = "$($rr.L),$($rr.T) $($rr.R - $rr.L)x$($rr.B - $rr.T)"
  })
  return $true
}
[void][D.W]::EnumChildWindows($main, $cb, [IntPtr]::Zero)
"descendants = $($rows.Count)"
$rows | Sort-Object { [int64]$_.hwnd } | Format-Table -AutoSize | Out-String -Width 220
