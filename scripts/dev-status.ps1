$ErrorActionPreference = "Stop"

Write-Host "Ports:"
Get-NetTCPConnection -LocalPort 3789,54329 -State Listen -ErrorAction SilentlyContinue |
  Select-Object LocalAddress, LocalPort, OwningProcess

if (Get-Command docker -ErrorAction SilentlyContinue) {
  Write-Host ""
  Write-Host "Docker containers:"
  docker ps --filter "name=starary-team-postgres-dev"
} else {
  Write-Host ""
  Write-Host "Docker was not found."
}
