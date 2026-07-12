@echo off
setlocal
cd /d "%~dp0"

start "" /min powershell.exe -NoProfile -WindowStyle Hidden -Command "$port=3789; $config='data\config\runtime.json'; if (Test-Path -LiteralPath $config) { try { $value=(Get-Content -Raw -LiteralPath $config | ConvertFrom-Json).serverPort; if ($value -ge 1024 -and $value -le 65535) { $port=[int]$value } } catch {} }; $baseUrl='http://127.0.0.1:'+$port; for ($i=0; $i -lt 60; $i++) { try { Invoke-WebRequest -UseBasicParsing -Uri ($baseUrl+'/health') -TimeoutSec 1 | Out-Null; Start-Process ($baseUrl+'/admin/'); exit 0 } catch { Start-Sleep -Seconds 1 } }"

madlibrary-server.exe
if errorlevel 1 (
  echo.
  echo Mad Library Server failed to start. See data\logs for details.
  pause
)
