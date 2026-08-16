# Build a multi-size .ico from the official Segoe Fluent Icons "Power" glyph.
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$ErrorActionPreference = "Stop"
$outIco = $args[0]
if (-not $outIco) {
    $outIco = Join-Path $PSScriptRoot "..\assets\app.ico"
}
$outIco = [System.IO.Path]::GetFullPath($outIco)
$tmp = Join-Path $env:TEMP "mqtt-shutdown-icon"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

$fontPath = "C:\Windows\Fonts\SegoeIcons.ttf"
$pfc = New-Object System.Drawing.Text.PrivateFontCollection
$pfc.AddFontFile($fontPath)
$family = $pfc.Families[0]
Write-Host "Font family: $($family.Name)"

# Segoe Fluent Icons / MDL2: PowerButton
$glyph = [char]0xE7E8
$accent = [System.Drawing.Color]::FromArgb(255, 0, 120, 212) # #0078D4
$white = [System.Drawing.Color]::White

function New-RoundedRectPath([int]$size, [int]$radius) {
    $d = $radius * 2
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc(0, 0, $d, $d, 180, 90)
    $path.AddArc($size - $d, 0, $d, $d, 270, 90)
    $path.AddArc($size - $d, $size - $d, $d, $d, 0, 90)
    $path.AddArc(0, $size - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-IconPng([int]$size) {
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

    $fontSize = [single]($size * 0.52)
    $font = New-Object System.Drawing.Font $family, $fontSize, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
    $sf = New-Object System.Drawing.StringFormat
    $sf.Alignment = [System.Drawing.StringAlignment]::Center
    $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
    $sf.FormatFlags = [System.Drawing.StringFormatFlags]::NoWrap
    $rect = New-Object System.Drawing.RectangleF 0, ($size * 0.02), $size, $size
    $g.DrawString($glyph, $font, [System.Drawing.Brushes]::White, $rect, $sf)
    $font.Dispose()
    $sf.Dispose()
    $g.Dispose()

    $png = Join-Path $tmp ("icon-{0}.png" -f $size)
    $bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    return $png
}

$sizes = @(16, 24, 32, 48, 64, 256)
$pngs = @()
foreach ($s in $sizes) {
    $pngs += New-IconPng $s
}

# Pack PNG frames into a Vista+ ICO.
python -c @"
import struct, pathlib
pngs = [pathlib.Path(p) for p in r'''$($pngs -join '|')'''.split('|')]
blobs = [p.read_bytes() for p in pngs]
count = len(blobs)
offset = 6 + 16 * count
entries = bytearray()
payload = bytearray()
for blob in blobs:
    # PNG IHDR: width/height at bytes 16-23
    w = blob[16]
    h = blob[20]
    entries += struct.pack('<BBBBHHII', w % 256, h % 256, 0, 0, 1, 32, len(blob), offset)
    payload += blob
    offset += len(blob)
out = struct.pack('<HHH', 0, 1, count) + entries + payload
pathlib.Path(r'$outIco').write_bytes(out)
print('wrote', r'$outIco', 'bytes', len(out))
"@

Write-Host "done"
