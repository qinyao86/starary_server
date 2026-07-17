$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$releaseDir = Join-Path $root "target\release\windows-x64"
$buildTarget = Join-Path $root "target\build\desktop"
$bundle = Join-Path $buildTarget "release\bundle\nsis"

if (Test-Path -LiteralPath $releaseDir) { Remove-Item -LiteralPath $releaseDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

$env:CARGO_TARGET_DIR = $buildTarget
Push-Location (Join-Path $root "desktop")
try {
  npm ci
  npm run tauri -- build
} finally {
  Pop-Location
}

$setup = Get-ChildItem -LiteralPath $bundle -Filter "*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { throw "Tauri did not create an NSIS installer." }
$destination = Join-Path $releaseDir "Mad-Library-Server_0.1.0_windows-x64-setup.exe"
Move-Item -LiteralPath $setup.FullName -Destination $destination -Force
$hash = Get-FileHash -Algorithm SHA256 -LiteralPath $destination
Set-Content -LiteralPath (Join-Path $releaseDir "SHA256SUMS.txt") -Value ("{0}  {1}" -f $hash.Hash, (Split-Path $destination -Leaf)) -Encoding ascii

$runtime = Join-Path $root "target\build\desktop\runtime"
if (Test-Path -LiteralPath $runtime) {
  Get-ChildItem -LiteralPath $runtime -Force | Remove-Item -Recurse -Force
}
Write-Host "Release created: $destination"
