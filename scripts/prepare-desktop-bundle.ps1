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
  Get-ChildItem -LiteralPath $runtime -Force | Where-Object { $_.Name -ne ".gitkeep" } | ForEach-Object {
    try {
      Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction Stop
    } catch {
      Write-Warning "Skipping locked runtime item: $($_.FullName)"
    }
  }
}
New-Item -ItemType Directory -Force -Path (Join-Path $runtime "admin-ui") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $runtime "postgresql") | Out-Null
Copy-Item -LiteralPath $serverExe -Destination (Join-Path $runtime "madlibrary-server.exe")
Copy-Item -Path (Join-Path $root "admin-ui\dist\*") -Destination (Join-Path $runtime "admin-ui") -Recurse
$postgresSource = Join-Path $root "binaries\windows-x64\postgresql"
$postgresTarget = Join-Path $runtime "postgresql"
& robocopy $postgresSource $postgresTarget /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -gt 7) { throw "Failed to copy PostgreSQL runtime. Robocopy exit code: $LASTEXITCODE" }
