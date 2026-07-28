<#
.SYNOPSIS
    Regenerate the icon set from resources/ellipsoid.svg.

.DESCRIPTION
    Produces everything downstream of the artwork, so ellipsoid.svg is the only file anyone
    edits:

      icons/<n>x<n>.png   16 through 1024, the plain sized set
      icon.png            256x256, what the installer renders its banner and panel from
      icon.ico            16/32/48/64/128/256, embedded in the .exe and used by the MSI
      icon.icns           the macOS container, for completeness

    Rendered once at 1024 and downsampled, rather than rasterising the SVG separately at each
    size. Downsampling from 4x or more is supersampling, which antialiases a wireframe of thin
    strokes far better than asking any renderer for a 16x16 in one step — at that size the
    strokes are a fraction of a pixel wide and rendering directly drops most of them.

    Microsoft Edge does the rasterising. It is not a build dependency: the outputs are committed,
    and this runs only when the artwork changes.

.PARAMETER Svg
    Source artwork. Defaults to resources/ellipsoid.svg.

.PARAMETER Browser
    Path to a Chromium-based browser. Defaults to Microsoft Edge, then Chrome.
#>
[CmdletBinding()]
param(
    [string]$Svg,
    [string]$Browser
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Add-Type -AssemblyName System.Drawing

$ResourcesDir = $PSScriptRoot
if (-not $Svg) { $Svg = Join-Path $ResourcesDir 'ellipsoid.svg' }
if (-not (Test-Path -LiteralPath $Svg)) { throw "artwork not found: $Svg" }

$IconsDir = Join-Path $ResourcesDir 'icons'
New-Item -ItemType Directory -Force -Path $IconsDir | Out-Null

# The sized set. 1024 is also the master render everything else is derived from.
$Sizes = @(16, 24, 32, 48, 64, 96, 128, 256, 512, 1024)
# What goes in the .ico. More sizes only inflate a file Windows picks one entry out of.
$IcoSizes = @(16, 32, 48, 64, 128, 256)
# Apple's PNG-based icns types, by pixel size.
$IcnsTypes = @{ 16 = 'icp4'; 32 = 'icp5'; 64 = 'icp6'; 128 = 'ic07'; 256 = 'ic08'; 512 = 'ic09'; 1024 = 'ic10' }

function Write-Step($message) { Write-Host "==> $message" -ForegroundColor Cyan }
function Write-Note($message) { Write-Host "    $message" -ForegroundColor DarkGray }

function Resolve-Browser {
    if ($Browser) {
        if (-not (Test-Path -LiteralPath $Browser)) { throw "browser not found: $Browser" }
        return $Browser
    }
    $candidates = @(
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe"
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe"
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe"
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) { return $candidate }
    }
    throw 'No Chromium-based browser found. Pass -Browser with a path to msedge.exe or chrome.exe.'
}

<#
    Rasterise the SVG to a PNG of the given square size.

    Loaded through a wrapper page rather than opened directly. ellipsoid.svg carries an intrinsic
    width and height of 256, and a browser honours those: asked for a 1024x1024 shot of the file
    itself it paints a 256x256 icon in the corner and leaves the rest transparent. Stretching it
    to the viewport is what makes the render resolution independent of the artwork's declared
    size — the viewBox does the scaling, so this is still vector all the way down.

    --default-background-color=00000000 is what keeps the corners outside the squircle
    transparent; without it the shot comes back on opaque white and every rounded corner
    acquires a white triangle.
#>
function Invoke-Render {
    param([Parameter(Mandatory)][string]$Source,
          [Parameter(Mandatory)][string]$Destination,
          [Parameter(Mandatory)][int]$Size)

    $browser = Resolve-Browser
    $profile = Join-Path $env:TEMP 'ellipsoid-icon-render'
    $svgUri = 'file:///' + ((Resolve-Path -LiteralPath $Source).Path -replace '\\', '/')
    $wrapper = Join-Path $env:TEMP 'ellipsoid-icon-render.html'
    @"
<!doctype html>
<meta charset="utf-8">
<style>
  html, body { margin: 0; padding: 0; background: transparent; overflow: hidden; }
  img { display: block; width: 100vw; height: 100vh; }
</style>
<img src="$svgUri">
"@ | Set-Content -LiteralPath $wrapper -Encoding utf8
    $uri = 'file:///' + ($wrapper -replace '\\', '/')
    $arguments = @(
        '--headless'
        '--disable-gpu'
        "--user-data-dir=$profile"
        '--default-background-color=00000000'
        '--force-device-scale-factor=1'
        "--window-size=$Size,$Size"
        "--screenshot=$Destination"
        $uri
    )
    # Windows PowerShell wraps anything a native program writes to stderr in an ErrorRecord, and
    # with $ErrorActionPreference = 'Stop' that aborts the script even on success. The browser
    # announces "N bytes written to file ..." on stderr, so this is not hypothetical. Whether the
    # file appeared is the only signal worth trusting.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $browser @arguments 2>&1 | Out-Null } finally { $ErrorActionPreference = $previous }

    if (-not (Test-Path -LiteralPath $Destination)) {
        throw "the browser produced no image; tried: $browser"
    }
    $rendered = New-Object System.Drawing.Bitmap($Destination)
    try {
        if ($rendered.Width -ne $Size -or $rendered.Height -ne $Size) {
            throw "expected ${Size}x${Size}, got $($rendered.Width)x$($rendered.Height)"
        }
        # A fully transparent render means the page never painted.
        if ($rendered.GetPixel([int]($Size / 2), [int]($Size / 2)).A -eq 0) {
            throw 'the centre of the render is transparent; the SVG did not paint'
        }
    } finally { $rendered.Dispose() }
}

<#
    Resize with the settings that actually matter for icons.

    SourceCopy rather than the default blend: the destination starts transparent, and blending
    a partly transparent source onto it darkens the edge pixels. TileFlipXY stops the bicubic
    kernel sampling off the edge of the source, which otherwise leaves a faint border.
#>
function Resize-Bitmap {
    param([Parameter(Mandatory)][System.Drawing.Bitmap]$Source,
          [Parameter(Mandatory)][int]$Size)

    $out = New-Object System.Drawing.Bitmap($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $out.SetResolution($Source.HorizontalResolution, $Source.VerticalResolution)
    $g = [System.Drawing.Graphics]::FromImage($out)
    try {
        $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
        $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $attributes = New-Object System.Drawing.Imaging.ImageAttributes
        try {
            $attributes.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
            $rect = New-Object System.Drawing.Rectangle(0, 0, $Size, $Size)
            $g.DrawImage($Source, $rect, 0, 0, $Source.Width, $Source.Height,
                         [System.Drawing.GraphicsUnit]::Pixel, $attributes)
        } finally { $attributes.Dispose() }
    } finally { $g.Dispose() }
    return $out
}

<#
    Write a multi-size .ico.

    Every entry is an uncompressed 32-bit DIB, which is what Windows has always accepted. PNG
    entries are legal from Vista on and would make the file far smaller, but the consumers here
    include the Windows Installer's icon handling, and a DIB costs nothing but disk.

    Each image is a BITMAPINFOHEADER whose height is doubled — the format expects a colour
    bitmap followed by an AND mask — then bottom-up BGRA rows, then the mask. The mask is all
    zeroes because the alpha channel already carries the transparency; it cannot be omitted.
#>
function Write-Ico {
    param([Parameter(Mandatory)][hashtable]$Images,
          [Parameter(Mandatory)][int[]]$Sizes,
          [Parameter(Mandatory)][string]$Path)

    $stream = [System.IO.File]::Create($Path)
    $writer = New-Object System.IO.BinaryWriter($stream)
    try {
        $writer.Write([uint16]0)               # reserved
        $writer.Write([uint16]1)               # 1 = icon
        $writer.Write([uint16]$Sizes.Count)

        # Directory entries come first, so every image's offset has to be known up front.
        $offset = 6 + 16 * $Sizes.Count
        $payloads = @()
        foreach ($size in $Sizes) {
            $bitmap = $Images[$size]
            $maskRow = [math]::Ceiling($size / 8.0)
            if ($maskRow % 4 -ne 0) { $maskRow += 4 - ($maskRow % 4) }
            $bytes = New-Object byte[] (40 + $size * $size * 4 + $maskRow * $size)

            # BITMAPINFOHEADER
            [BitConverter]::GetBytes([int]40).CopyTo($bytes, 0)
            [BitConverter]::GetBytes([int]$size).CopyTo($bytes, 4)
            [BitConverter]::GetBytes([int]($size * 2)).CopyTo($bytes, 8)
            [BitConverter]::GetBytes([uint16]1).CopyTo($bytes, 12)
            [BitConverter]::GetBytes([uint16]32).CopyTo($bytes, 14)
            [BitConverter]::GetBytes([int]0).CopyTo($bytes, 16)   # BI_RGB
            [BitConverter]::GetBytes([int]($size * $size * 4 + $maskRow * $size)).CopyTo($bytes, 20)

            # Pixels, bottom-up. LockBits hands back BGRA in memory order, which is exactly
            # what the DIB wants, so rows only need reversing.
            $rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
            $data = $bitmap.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly,
                                     [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
            try {
                $row = New-Object byte[] ($size * 4)
                for ($y = 0; $y -lt $size; $y++) {
                    $source = [IntPtr]::Add($data.Scan0, $data.Stride * ($size - 1 - $y))
                    [System.Runtime.InteropServices.Marshal]::Copy($source, $row, 0, $row.Length)
                    $row.CopyTo($bytes, 40 + $y * $size * 4)
                }
            } finally { $bitmap.UnlockBits($data) }

            $payloads += , $bytes
            $writer.Write([byte]($(if ($size -ge 256) { 0 } else { $size })))
            $writer.Write([byte]($(if ($size -ge 256) { 0 } else { $size })))
            $writer.Write([byte]0)             # palette entries
            $writer.Write([byte]0)             # reserved
            $writer.Write([uint16]1)           # colour planes
            $writer.Write([uint16]32)          # bits per pixel
            $writer.Write([int]$bytes.Length)
            $writer.Write([int]$offset)
            $offset += $bytes.Length
        }
        foreach ($payload in $payloads) { $writer.Write($payload) }
    } finally { $writer.Dispose(); $stream.Dispose() }
}

<#
    Write a PNG-based .icns.

    The container is a four-byte magic, a big-endian total length, then one chunk per image:
    a four-character type, a big-endian length covering the header too, and the PNG bytes.
    Nothing in this repository reads it — it is here so the icon set is not half of one design
    and half of another if a macOS bundle is ever built.
#>
function Write-Icns {
    param([Parameter(Mandatory)][hashtable]$Images,
          [Parameter(Mandatory)][hashtable]$Types,
          [Parameter(Mandatory)][string]$Path)

    $chunks = @()
    foreach ($size in ($Types.Keys | Sort-Object)) {
        $memory = New-Object System.IO.MemoryStream
        try {
            $Images[$size].Save($memory, [System.Drawing.Imaging.ImageFormat]::Png)
            $png = $memory.ToArray()
        } finally { $memory.Dispose() }

        $header = [System.Text.Encoding]::ASCII.GetBytes($Types[$size])
        $length = [BitConverter]::GetBytes([int]($png.Length + 8))
        [array]::Reverse($length)              # icns is big-endian throughout
        $chunks += , ($header + $length + $png)
    }

    $total = 8
    foreach ($chunk in $chunks) { $total += $chunk.Length }
    $totalBytes = [BitConverter]::GetBytes([int]$total)
    [array]::Reverse($totalBytes)

    $stream = [System.IO.File]::Create($Path)
    try {
        $magic = [System.Text.Encoding]::ASCII.GetBytes('icns')
        $stream.Write($magic, 0, 4)
        $stream.Write($totalBytes, 0, 4)
        foreach ($chunk in $chunks) { $stream.Write($chunk, 0, $chunk.Length) }
    } finally { $stream.Dispose() }
}

# ---------------------------------------------------------------------------------------

Write-Step "Rendering $(Split-Path -Leaf $Svg) at 1024x1024"
Write-Note "using $(Resolve-Browser)"
$master = Join-Path $env:TEMP 'ellipsoid-icon-1024.png'
Invoke-Render -Source $Svg -Destination $master -Size 1024

$source = New-Object System.Drawing.Bitmap($master)
$images = @{}
try {
    Write-Step 'Downsampling'
    foreach ($size in $Sizes) {
        $images[$size] = if ($size -eq 1024) { New-Object System.Drawing.Bitmap($source) }
                         else { Resize-Bitmap -Source $source -Size $size }
        $png = Join-Path $IconsDir "${size}x${size}.png"
        $images[$size].Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
        Write-Note ("{0,-13} {1,8:N0} bytes" -f "${size}x${size}.png", (Get-Item $png).Length)
    }

    $iconPng = Join-Path $ResourcesDir 'icon.png'
    $images[256].Save($iconPng, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Note ("{0,-13} {1,8:N0} bytes" -f 'icon.png', (Get-Item $iconPng).Length)

    Write-Step 'Assembling icon.ico'
    $ico = Join-Path $ResourcesDir 'icon.ico'
    Write-Ico -Images $images -Sizes $IcoSizes -Path $ico
    Write-Note ("{0} at {1}  {2:N0} bytes" -f 'icon.ico', ($IcoSizes -join '/'), (Get-Item $ico).Length)

    Write-Step 'Assembling icon.icns'
    $icns = Join-Path $ResourcesDir 'icon.icns'
    Write-Icns -Images $images -Types $IcnsTypes -Path $icns
    Write-Note ("{0} {1:N0} bytes" -f 'icon.icns', (Get-Item $icns).Length)
} finally {
    foreach ($image in $images.Values) { $image.Dispose() }
    $source.Dispose()
}

Write-Host ''
Write-Note 'The .exe icon and the installer artwork are rebuilt from these; nothing else to do.'
