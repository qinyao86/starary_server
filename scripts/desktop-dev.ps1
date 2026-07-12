$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$env:CARGO_TARGET_DIR = Join-Path $root "target\desktop"
$env:MADLIBRARY_DESKTOP_RUNTIME = Join-Path $root "target\desktop-runtime"

Push-Location (Join-Path $root "admin-ui")
try {
  npm run build
} finally {
  Pop-Location
}

powershell -ExecutionPolicy Bypass -File (Join-Path $root "scripts\prepare-desktop-bundle.ps1") -Development

Push-Location (Join-Path $root "desktop")
try {
  npm ci
  npm run tauri -- dev
} finally {
  Pop-Location
}
