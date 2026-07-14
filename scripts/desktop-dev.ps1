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
  if (-not (Test-Path (Join-Path (Get-Location) "node_modules\@tauri-apps\cli\tauri.js"))) {
    npm ci
  }
  npm run tauri -- dev
} finally {
  Pop-Location
}
