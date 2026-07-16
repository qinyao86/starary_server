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
$postgresManifest = Get-Content -Raw -Path (Join-Path $root "packaging\postgresql-windows-x64.json") | ConvertFrom-Json

if ($Development) {
  cargo build --manifest-path (Join-Path $root "Cargo.toml")
} else {
  Push-Location (Join-Path $root "admin-ui")
  try { npm run build } finally { Pop-Location }
  cargo build --release --locked --manifest-path (Join-Path $root "Cargo.toml")
}

if ((-not $Development) -and (Test-Path -LiteralPath $runtime)) {
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
$serverTarget = Join-Path $runtime "madlibrary-server.exe"
try {
  Copy-Item -LiteralPath $serverExe -Destination $serverTarget -Force -ErrorAction Stop
} catch {
  if ($Development -and (Test-Path -LiteralPath $serverTarget)) {
    Write-Warning "Skipping locked development server executable: $serverTarget"
  } else {
    throw
  }
}
$adminSource = Join-Path $root "admin-ui\dist"
$adminTarget = Join-Path $runtime "admin-ui"
& robocopy $adminSource $adminTarget /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -gt 7) { throw "Failed to copy admin UI assets. Robocopy exit code: $LASTEXITCODE" }
$postgresSource = Join-Path $root "binaries\windows-x64\postgresql"
$postgresTarget = Join-Path $runtime "postgresql"
& robocopy $postgresSource $postgresTarget /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -gt 7) { throw "Failed to copy PostgreSQL runtime. Robocopy exit code: $LASTEXITCODE" }
foreach ($fileName in $postgresManifest.requiredBinFiles) {
  $target = Join-Path (Join-Path $postgresTarget "bin") $fileName
  if (-not (Test-Path -LiteralPath $target)) {
    throw "PostgreSQL runtime is incomplete after copy; missing: $target"
  }
}
foreach ($fileName in $postgresManifest.requiredFiles) {
  $target = Join-Path $postgresTarget $fileName
  if (-not (Test-Path -LiteralPath $target)) {
    throw "PostgreSQL runtime is incomplete after copy; missing: $target"
  }
}
