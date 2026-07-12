$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$artifacts = Join-Path $root "artifacts\windows-x64"
$target = Join-Path $root "target\desktop"
$bundle = Join-Path $target "release\bundle\nsis"

if (Test-Path -LiteralPath $artifacts) { Remove-Item -LiteralPath $artifacts -Recurse -Force }
New-Item -ItemType Directory -Force -Path $artifacts | Out-Null

$env:CARGO_TARGET_DIR = $target
Push-Location (Join-Path $root "desktop")
try {
  npm ci
  npm run tauri -- build
} finally {
  Pop-Location
}

$setup = Get-ChildItem -LiteralPath $bundle -Filter "*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { throw "Tauri did not create an NSIS installer." }
$destination = Join-Path $artifacts "Mad-Library-Server_0.1.0_windows-x64-setup.exe"
Copy-Item -LiteralPath $setup.FullName -Destination $destination -Force
$hash = Get-FileHash -Algorithm SHA256 -LiteralPath $destination
Set-Content -LiteralPath (Join-Path $artifacts "SHA256SUMS.txt") -Value ("{0}  {1}" -f $hash.Hash, (Split-Path $destination -Leaf)) -Encoding ascii

if (Test-Path -LiteralPath (Join-Path $target "release\bundle")) { Remove-Item -LiteralPath (Join-Path $target "release\bundle") -Recurse -Force }
$runtime = Join-Path $root "desktop\bundle-resources\runtime"
if (Test-Path -LiteralPath $runtime) {
  Get-ChildItem -LiteralPath $runtime -Force | Where-Object { $_.Name -ne ".gitkeep" } | Remove-Item -Recurse -Force
}
Write-Host "Release created: $destination"
