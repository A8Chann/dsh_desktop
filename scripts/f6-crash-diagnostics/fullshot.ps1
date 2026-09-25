Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$b = [System.Windows.Forms.SystemInformation]::VirtualScreen
"virtual screen: $($b.X),$($b.Y) $($b.Width)x$($b.Height)"
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.X, $b.Y, 0, 0, (New-Object System.Drawing.Size $b.Width, $b.Height))
$g.Dispose()
$bmp.Save('D:\HTML\DSH_Desktop\.tmp-f6\fullscreen.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
'saved fullscreen'
