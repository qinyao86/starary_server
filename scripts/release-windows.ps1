$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$releaseDir = Join-Path $root "target\release\windows-x64"
$buildTarget = Join-Path $root "target\build\desktop"
$bundle = Join-Path $buildTarget "release\bundle\nsis"
$packageJson = Get-Content -LiteralPath (Join-Path $root "package.json") -Raw | ConvertFrom-Json
$version = $packageJson.version

if (Test-Path -LiteralPath $releaseDir) { Remove-Item -LiteralPath $releaseDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

$env:CARGO_TARGET_DIR = $buildTarget
Push-Location (Join-Path $root "desktop")
try {
  if (-not (Test-Path (Join-Path (Get-Location) "node_modules\@tauri-apps\cli\tauri.js"))) {
    npm install --no-fund --no-audit
  }
  npm run tauri -- build
} finally {
  Pop-Location
}

$setup = Get-ChildItem -LiteralPath $bundle -Filter "*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { throw "Tauri did not create an NSIS installer." }
$destination = Join-Path $releaseDir "Starary-Server_$version`_windows-x64-setup.exe"
Move-Item -LiteralPath $setup.FullName -Destination $destination -Force
$stream = [System.IO.File]::OpenRead($destination)
try {
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  $hash = -join ($sha256.ComputeHash($stream) | ForEach-Object { $_.ToString("x2") })
} finally {
  $stream.Dispose()
  if ($sha256) { $sha256.Dispose() }
}
Set-Content -LiteralPath (Join-Path $releaseDir "SHA256SUMS.txt") -Value ("{0}  {1}" -f $hash, (Split-Path $destination -Leaf)) -Encoding ascii

$runtime = Join-Path $root "target\build\desktop\runtime"
if (Test-Path -LiteralPath $runtime) {
  Get-ChildItem -LiteralPath $runtime -Force | Remove-Item -Recurse -Force
}
Write-Host "Release created: $destination"
