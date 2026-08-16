# Render the Segoe Fluent Icons Power glyph into a classic (BMP) .ico.
# PNG-compressed ICO frames work in the tray but AppWindow/taskbar often ignore them.
Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = "Stop"
$outIco = if ($args[0]) { $args[0] } else { Join-Path $PSScriptRoot "..\assets\app.ico" }
$outIco = [System.IO.Path]::GetFullPath($outIco)

$pfc = New-Object System.Drawing.Text.PrivateFontCollection
$pfc.AddFontFile("C:\Windows\Fonts\SegoeIcons.ttf")
$family = $pfc.Families[0]
$glyph = [char]0xE7E8
$accent = [System.Drawing.Color]::FromArgb(255, 0, 120, 212)

function New-RoundedRectPath([int]$size, [int]$radius) {
    $d = [Math]::Max(2, $radius * 2)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc(0, 0, $d, $d, 180, 90)
    $path.AddArc($size - $d, 0, $d, $d, 270, 90)
    $path.AddArc($size - $d, $size - $d, $d, $d, 0, 90)
    $path.AddArc(0, $size - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-GlyphBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear([System.Drawing.Color]::Transparent)
    $radius = [Math]::Max(2, [int][Math]::Round($size * 0.22))
    $path = New-RoundedRectPath $size $radius
    $brush = New-Object System.Drawing.SolidBrush $accent
    $g.FillPath($brush, $path)
    $brush.Dispose()
    $path.Dispose()
    $font = New-Object System.Drawing.Font $family, ([single]($size * 0.52)), ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
    $sf = New-Object System.Drawing.StringFormat
    $sf.Alignment = [System.Drawing.StringAlignment]::Center
    $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
    $rect = New-Object System.Drawing.RectangleF 0, ($size * 0.02), $size, $size
    $g.DrawString($glyph, $font, [System.Drawing.Brushes]::White, $rect, $sf)
    $font.Dispose()
    $sf.Dispose()
    $g.Dispose()
    return $bmp
}

function Get-BgraBottomUp([System.Drawing.Bitmap]$bmp) {
    $w = $bmp.Width
    $h = $bmp.Height
    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $stride = $data.Stride
    $bytes = New-Object byte[] ($stride * $h)
    [Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
    $bmp.UnlockBits($data)
    $out = New-Object byte[] ($w * $h * 4)
    for ($y = 0; $y -lt $h; $y++) {
        $srcY = $h - 1 - $y
        [Buffer]::BlockCopy($bytes, $srcY * $stride, $out, $y * $w * 4, $w * 4)
    }
    return $out
}

function New-AndMask([byte[]]$bgra, [int]$w, [int]$h) {
    $stride = [int](([Math]::Ceiling($w / 32.0)) * 4)
    $mask = New-Object byte[] ($stride * $h)
    for ($y = 0; $y -lt $h; $y++) {
        for ($x = 0; $x -lt $w; $x++) {
            $alpha = $bgra[($y * $w + $x) * 4 + 3]
            if ($alpha -lt 16) {
                $bit = 7 - ($x % 8)
                $mask[$y * $stride + [int][Math]::Floor($x / 8)] = $mask[$y * $stride + [int][Math]::Floor($x / 8)] -bor (1 -shl $bit)
            }
        }
    }
    return $mask
}

function New-IconImage([System.Drawing.Bitmap]$bmp) {
    $w = $bmp.Width
    $h = $bmp.Height
    $xor = Get-BgraBottomUp $bmp
    $and = New-AndMask $xor $w $h
    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter $ms
    $bw.Write([uint32]40)
    $bw.Write([int32]$w)
    $bw.Write([int32]($h * 2))
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]0)
    $bw.Write([uint32]$xor.Length)
    $bw.Write([int32]0)
    $bw.Write([int32]0)
    $bw.Write([uint32]0)
    $bw.Write([uint32]0)
    $bw.Write($xor, 0, $xor.Length)
    $bw.Write($and, 0, $and.Length)
    $bw.Flush()
    return $ms.ToArray()
}

$sizes = @(16, 24, 32, 48, 64, 256)
$images = New-Object "System.Collections.Generic.List[byte[]]"
foreach ($s in $sizes) {
    $bmp = New-GlyphBitmap $s
    $images.Add((New-IconImage $bmp))
    $bmp.Dispose()
}

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $ms
$bw.Write([uint16]0)
$bw.Write([uint16]1)
$bw.Write([uint16]$images.Count)
$offset = 6 + 16 * $images.Count
foreach ($img in $images) {
    $w = [BitConverter]::ToInt32($img, 4)
    $entryW = if ($w -ge 256) { 0 } else { $w }
    $bw.Write([byte]$entryW)
    $bw.Write([byte]$entryW)
    $bw.Write([byte]0)
    $bw.Write([byte]0)
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]$img.Length)
    $bw.Write([uint32]$offset)
    $offset += $img.Length
}
foreach ($img in $images) { $bw.Write($img, 0, $img.Length) }
$bw.Flush()
[IO.File]::WriteAllBytes($outIco, $ms.ToArray())
Write-Host "wrote $outIco ($($ms.Length) bytes, classic ICO)"
