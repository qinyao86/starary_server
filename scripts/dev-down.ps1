$ErrorActionPreference = "Stop"

$serverDir = Resolve-Path (Join-Path $PSScriptRoot "..")
$composePath = Join-Path $serverDir "docker-compose.dev.yml"

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Write-Host "Docker was not found."
  exit 1
}

Push-Location $serverDir
try {
  docker compose -f $composePath down
} finally {
  Pop-Location
}
