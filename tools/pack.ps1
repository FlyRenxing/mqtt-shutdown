# Build a self-contained payload and wrap it as a single MQTT关机.exe
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$env:MQTT_SHUTDOWN_SELF_CONTAINED = "1"
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

Write-Host "Building self-contained mqtt-shutdown..."
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$rel = Join-Path $root "target\release"
$stage = Join-Path $root "target\pack-stage"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

$keepLocales = @("en-us", "zh-cn", "zh-tw")
Get-ChildItem $rel -Force | ForEach-Object {
    $name = $_.Name
    if ($name -in @("mqtt-shutdown.exe", "mqtt-shutdown.pdb", "build", "deps", "examples", "incremental", ".fingerprint")) {
        if ($name -eq "mqtt-shutdown.exe") {
            Copy-Item $_.FullName (Join-Path $stage $name)
        }
        return
    }
    if ($_.PSIsContainer) {
        if ($keepLocales -contains $name.ToLowerInvariant()) {
            Copy-Item $_.FullName (Join-Path $stage $name) -Recurse
        }
        return
    }
    $ext = $_.Extension.ToLowerInvariant()
    if ($ext -in @(".exe", ".dll", ".pri")) {
        Copy-Item $_.FullName (Join-Path $stage $name)
    }
}

if (-not (Test-Path (Join-Path $stage "mqtt-shutdown.exe"))) {
    throw "mqtt-shutdown.exe missing from stage"
}

$payload = Join-Path $root "target\payload.tar"
if (Test-Path $payload) { Remove-Item $payload -Force }
& "$env:SystemRoot\System32\tar.exe" -cf $payload -C $stage .
if ($LASTEXITCODE -ne 0) { throw "tar failed" }

Write-Host "Building single-file stub..."
$env:MQTT_SHUTDOWN_PAYLOAD = $payload
cargo build --release --manifest-path (Join-Path $root "tools\stub\Cargo.toml") --target-dir (Join-Path $root "target\stub")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$out = Join-Path $dist "MQTT关机.exe"
Copy-Item (Join-Path $root "target\stub\release\mqtt-shutdown-stub.exe") $out -Force
Write-Host "Packed $out"
Get-Item $out | Select-Object FullName, Length, LastWriteTime
