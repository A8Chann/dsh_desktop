Add-Type -Namespace F7 -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern IntPtr GetFocus();
[DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

$hwnd = [F7.W]::FindWindow('Tauri Window', 'DSH Desktop')
if ($hwnd -eq [IntPtr]::Zero) { "no window"; exit 1 }
"target hwnd=$hwnd fg=$([F7.W]::GetForegroundWindow())"
[void][F7.W]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 500
"fg after activate=$([F7.W]::GetForegroundWindow())"

$r = New-Object F7.W+RECT
[void][F7.W]::GetWindowRect($hwnd, [ref]$r)
$w = $r.R - $r.L; $h = $r.B - $r.T
"rect=$($r.L),$($r.T) ${w}x${h}"

function Snap([string]$tag) {
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $g.Dispose()
  $f = "D:\HTML\DSH_Desktop\.tmp-f6\f6-$tag.png"
  $bmp.Save($f, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "  [$tag] saved -> $f"
}

Snap '00-before'
foreach ($i in 1..1) {
  [System.Windows.Forms.SendKeys]::SendWait('{F6}')
  "sent F6 #$i"
}
foreach ($ms in 300, 1000, 3000, 8000, 15000) {
  Start-Sleep -Milliseconds $ms
  Snap ("after-" + $ms + "ms-cum")
}
"done"
