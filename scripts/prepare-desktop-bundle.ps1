param([switch]$Development)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$buildRoot = Join-Path $root "target\build"
$buildModeRoot = if ($Development) { Join-Path $root "target\build-dev" } else { $buildRoot }
$frontendOut = Join-Path $buildModeRoot "frontend\admin-ui"
$desktopFrontendOut = Join-Path $buildModeRoot "frontend\ui"
$desktopRuntime = Join-Path $buildModeRoot "desktop\runtime"
$coreTarget = Join-Path $buildModeRoot "desktop"
$serverExe = if ($Development) { Join-Path $coreTarget "debug\starary-server.exe" } else { Join-Path $coreTarget "release\starary-server.exe" }
$env:CARGO_TARGET_DIR = $coreTarget
$postgresManifest = Get-Content -Raw -Path (Join-Path $root "packaging\postgresql-windows-x64.json") | ConvertFrom-Json

if ($Development) {
  cargo build --manifest-path (Join-Path $root "Cargo.toml")
} else {
  Push-Location (Join-Path $root "admin-ui")
  try {
    $env:STARARY_ADMIN_UI_OUT_DIR = $frontendOut
    npm run build
  } finally {
    Remove-Item Env:STARARY_ADMIN_UI_OUT_DIR -ErrorAction SilentlyContinue
    Pop-Location
  }
  cargo build --release --locked --manifest-path (Join-Path $root "Cargo.toml")
}

New-Item -ItemType Directory -Force -Path $frontendOut | Out-Null
New-Item -ItemType Directory -Force -Path $desktopFrontendOut | Out-Null
& robocopy (Join-Path $root "desktop\ui") $desktopFrontendOut /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -gt 7) { throw "Failed to copy desktop UI assets. Robocopy exit code: $LASTEXITCODE" }

if (Test-Path -LiteralPath $desktopRuntime) {
  Get-ChildItem -LiteralPath $desktopRuntime -Force | ForEach-Object {
    try {
      Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction Stop
    } catch {
      Write-Warning "Skipping locked runtime item: $($_.FullName)"
    }
  }
}
New-Item -ItemType Directory -Force -Path (Join-Path $desktopRuntime "admin-ui") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $desktopRuntime "postgresql") | Out-Null
$serverTarget = Join-Path $desktopRuntime "starary-server.exe"
try {
  Copy-Item -LiteralPath $serverExe -Destination $serverTarget -Force -ErrorAction Stop
} catch {
  if ($Development -and (Test-Path -LiteralPath $serverTarget)) {
    Write-Warning "Skipping locked development server executable: $serverTarget"
  } else {
    throw
  }
}
$adminSource = $frontendOut
$adminTarget = Join-Path $desktopRuntime "admin-ui"
& robocopy $adminSource $adminTarget /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -gt 7) { throw "Failed to copy admin UI assets. Robocopy exit code: $LASTEXITCODE" }
$postgresSource = Join-Path $root "binaries\windows-x64\postgresql"
$postgresTarget = Join-Path $desktopRuntime "postgresql"
& robocopy $postgresSource $postgresTarget /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -gt 7) { throw "Failed to copy PostgreSQL runtime. Robocopy exit code: $LASTEXITCODE" }
foreach ($fileName in $postgresManifest.requiredBinFiles) {
  $target = Join-Path (Join-Path $postgresTarget "bin") $fileName
  if (-not (Test-Path -LiteralPath $target)) {
    throw "PostgreSQL runtime is incomplete after copy; missing: $target"
  }
}

if ($Development) {
  $compatDesktopRuntime = Join-Path $buildRoot "desktop\runtime"
  $compatFrontendOut = Join-Path $buildRoot "frontend\ui"
  New-Item -ItemType Directory -Force -Path $compatDesktopRuntime | Out-Null
  New-Item -ItemType Directory -Force -Path $compatFrontendOut | Out-Null
  & robocopy $desktopRuntime $compatDesktopRuntime /MIR /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
  if ($LASTEXITCODE -gt 7) { throw "Failed to mirror development desktop runtime. Robocopy exit code: $LASTEXITCODE" }
  & robocopy $desktopFrontendOut $compatFrontendOut /MIR /R:0 /W:0 /NFL /NDL /NJH /NJS /NP | Out-Null
  if ($LASTEXITCODE -gt 7) { throw "Failed to mirror development frontend UI. Robocopy exit code: $LASTEXITCODE" }
}
foreach ($fileName in $postgresManifest.requiredFiles) {
  $target = Join-Path $postgresTarget $fileName
  if (-not (Test-Path -LiteralPath $target)) {
    throw "PostgreSQL runtime is incomplete after copy; missing: $target"
  }
}
