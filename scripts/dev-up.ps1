$ErrorActionPreference = "Stop"

$serverDir = Resolve-Path (Join-Path $PSScriptRoot "..")
$envPath = Join-Path $serverDir ".env"
$devEnvPath = Join-Path $serverDir ".env.dev.example"
$composePath = Join-Path $serverDir "docker-compose.dev.yml"
$storageDir = Join-Path $serverDir ".dev\storage"
$adminUiOutDir = Join-Path $serverDir "target\build-dev\frontend\admin-ui"

if (-not (Test-Path $envPath)) {
  Copy-Item -LiteralPath $devEnvPath -Destination $envPath
  Write-Host "Created .env from .env.dev.example"
}

New-Item -ItemType Directory -Force -Path $storageDir | Out-Null

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Write-Host "Docker was not found. Install Docker Desktop or run your own PostgreSQL, then update .env."
  exit 1
}

Push-Location $serverDir
try {
  docker compose -f $composePath up -d postgres
} finally {
  Pop-Location
}

Write-Host "Waiting for PostgreSQL on 127.0.0.1:54329..."
$ready = $false
for ($i = 0; $i -lt 40; $i++) {
  $connection = Test-NetConnection -ComputerName 127.0.0.1 -Port 54329 -InformationLevel Quiet
  if ($connection) {
    $ready = $true
    break
  }
  Start-Sleep -Seconds 1
}

if (-not $ready) {
  Write-Host "PostgreSQL did not become reachable on port 54329."
  exit 1
}

Push-Location (Join-Path $serverDir "admin-ui")
try {
  $env:STARARY_ADMIN_UI_OUT_DIR = $adminUiOutDir
  npm run build
} finally {
  Remove-Item Env:STARARY_ADMIN_UI_OUT_DIR -ErrorAction SilentlyContinue
  Pop-Location
}

Write-Host "Starting Starary Server..."
Write-Host "Open http://127.0.0.1:3789/admin after the server starts."

Push-Location $serverDir
try {
  $env:STARARY_ADMIN_ASSETS_DIR = $adminUiOutDir
  cargo run --manifest-path .\Cargo.toml
} finally {
  Remove-Item Env:STARARY_ADMIN_ASSETS_DIR -ErrorAction SilentlyContinue
  Pop-Location
}
