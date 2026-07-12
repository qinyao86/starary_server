param([switch]$Development)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$runtime = if ($Development) {
  Join-Path $root "target\desktop-runtime"
} else {
  Join-Path $root "desktop\bundle-resources\runtime"
}
$coreTarget = Join-Path $root "target\core"
$serverExe = if ($Development) { Join-Path $coreTarget "debug\madlibrary-server.exe" } else { Join-Path $coreTarget "release\madlibrary-server.exe" }
$env:CARGO_TARGET_DIR = $coreTarget

if ($Development) {
  cargo build --manifest-path (Join-Path $root "Cargo.toml")
} else {
  Push-Location (Join-Path $root "admin-ui")
  try { npm run build } finally { Pop-Location }
  cargo build --release --locked --manifest-path (Join-Path $root "Cargo.toml")
}

if (Test-Path -LiteralPath $runtime) {
  Get-ChildItem -LiteralPath $runtime -Force | Where-Object { $_.Name -ne ".gitkeep" } | Remove-Item -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $runtime "admin-ui") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $runtime "postgresql") | Out-Null
Copy-Item -LiteralPath $serverExe -Destination (Join-Path $runtime "madlibrary-server.exe")
Copy-Item -Path (Join-Path $root "admin-ui\dist\*") -Destination (Join-Path $runtime "admin-ui") -Recurse
Copy-Item -Path (Join-Path $root "binaries\windows-x64\postgresql\*") -Destination (Join-Path $runtime "postgresql") -Recurse
