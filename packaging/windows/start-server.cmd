@echo off
setlocal
cd /d "%~dp0"

start "" /min powershell.exe -NoProfile -WindowStyle Hidden -Command "$url='http://127.0.0.1:3789/admin/'; for ($i=0; $i -lt 60; $i++) { try { Invoke-WebRequest -UseBasicParsing -Uri 'http://127.0.0.1:3789/health' -TimeoutSec 1 | Out-Null; Start-Process $url; exit 0 } catch { Start-Sleep -Seconds 1 } }"

madlibrary-server.exe
if errorlevel 1 (
  echo.
  echo Mad Library Server failed to start. See data\logs for details.
  pause
)
