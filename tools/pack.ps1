# Pack mqtt-shutdown.exe + bootstrap.dll as a single compressed exe.
# WinUI / Windows App Runtime stays on the machine (framework-dependent).
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Remove-Item Env:MQTT_SHUTDOWN_SELF_CONTAINED -ErrorAction SilentlyContinue
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

Write-Host "Building framework-dependent mqtt-shutdown..."
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$rel = Join-Path $root "target\release"
$stage = Join-Path $root "target\pack-stage"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

$exe = Join-Path $rel "mqtt-shutdown.exe"
if (-not (Test-Path $exe)) { throw "mqtt-shutdown.exe missing from $rel" }
Copy-Item $exe (Join-Path $stage "mqtt-shutdown.exe")

$bootstrapNames = @(
    "microsoft.windowsappruntime.bootstrap.dll",
    "Microsoft.WindowsAppRuntime.Bootstrap.dll"
)
$bootstrap = @(
    ($bootstrapNames | ForEach-Object { Join-Path $rel $_ }),
    (Join-Path $root "..\vendor\windows-rs\crates\libs\reactor-setup\bootstrap\x64\Microsoft.WindowsAppRuntime.Bootstrap.dll")
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $bootstrap) {
    throw "microsoft.windowsappruntime.bootstrap.dll missing from build output"
}
Copy-Item $bootstrap (Join-Path $stage "microsoft.windowsappruntime.bootstrap.dll")

$payloadTar = Join-Path $root "target\payload.tar"
$payload = Join-Path $root "target\payload.tar.gz"
if (Test-Path $payloadTar) { Remove-Item $payloadTar -Force }
if (Test-Path $payload) { Remove-Item $payload -Force }
& "$env:SystemRoot\System32\tar.exe" -cf $payloadTar -C $stage .
if ($LASTEXITCODE -ne 0) { throw "tar failed" }

Add-Type -AssemblyName System.IO.Compression
$in = [System.IO.File]::OpenRead($payloadTar)
try {
    $out = [System.IO.File]::Create($payload)
    try {
        $gzip = New-Object System.IO.Compression.GZipStream(
            $out,
            [System.IO.Compression.CompressionLevel]::Optimal
        )
        try {
            $in.CopyTo($gzip)
        } finally {
            $gzip.Dispose()
        }
    } finally {
        $out.Dispose()
    }
} finally {
    $in.Dispose()
}

Write-Host "Building single-file stub..."
$env:MQTT_SHUTDOWN_PAYLOAD = $payload
cargo build --release --manifest-path (Join-Path $root "tools\stub\Cargo.toml") --target-dir (Join-Path $root "target\stub")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$outExe = Join-Path $dist "mqtt-shutdown.exe"
Copy-Item (Join-Path $root "target\stub\release\mqtt-shutdown-stub.exe") $outExe -Force
Write-Host "Packed $outExe"
Get-ChildItem $stage | Select-Object Name, @{N = "KB"; E = { [math]::Round($_.Length / 1KB, 1) } }
Get-Item $payload, $outExe | Select-Object Name, @{N = "MB"; E = { [math]::Round($_.Length / 1MB, 2) } }, Length
