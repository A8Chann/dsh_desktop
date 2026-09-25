param([string]$Key = '{F5}', [int]$Times = 1, [int]$ShotDelay = 2500)
Add-Type -Namespace K -Name W -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
public static extern IntPtr FindWindow(string cls, string title);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern IntPtr GetFocus();
[DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint f);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
'@
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

$hwnd = [K.W]::FindWindow('Tauri Window', 'DSH Desktop')
if ($hwnd -eq [IntPtr]::Zero) { "no window"; exit 1 }
[void][K.W]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 400
$r = New-Object K.W+RECT
[void][K.W]::GetWindowRect($hwnd, [ref]$r)
$w = $r.R - $r.L; $h = $r.B - $r.T
"fg=$([K.W]::GetForegroundWindow()) target=$hwnd"
$gf = [K.W]::GetFocus()
$gp = 0; [void][K.W]::GetWindowThreadProcessId($gf, [ref]$gp)
"focus hwnd=$gf pid=$gp"
"about to send $Key x$Times"
foreach ($i in 1..$Times) {
  [System.Windows.Forms.SendKeys]::SendWait($Key)
  Start-Sleep -Milliseconds 120
}
"sent"
Start-Sleep -Milliseconds $ShotDelay
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$g.Dispose()
$out = "D:\HTML\DSH_Desktop\.tmp-f6\key-" + ($Key -replace '[{}]', '') + "-x$Times.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"saved $out"
