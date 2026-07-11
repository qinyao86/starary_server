$ErrorActionPreference = "Stop"

$serverDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$postgresManifestPath = Join-Path $serverDir "packaging\postgresql-windows-x64.json"
$postgresManifest = Get-Content -Raw -LiteralPath $postgresManifestPath | ConvertFrom-Json
$postgresSource = Join-Path $serverDir "binaries\windows-x64\postgresql"

$outputDir = Join-Path $serverDir "release"
$packageWorkDir = Join-Path $serverDir "target\package"
$stagingDir = Join-Path $packageWorkDir "windows-x64"
$packageDir = Join-Path $stagingDir "madlibrary-server-windows-x64"
$packageZip = Join-Path $outputDir "madlibrary-server-windows-x64.zip"

function Assert-WorkspacePath([string]$Path) {
  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $root = $serverDir.TrimEnd('\') + '\'
  if (-not $fullPath.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to modify a path outside the repository: $fullPath"
  }
}

Assert-WorkspacePath $outputDir
Assert-WorkspacePath $packageWorkDir
Assert-WorkspacePath $stagingDir
Assert-WorkspacePath $packageDir

if (Test-Path -LiteralPath $stagingDir) {
  Remove-Item -LiteralPath $stagingDir -Recurse -Force
}
if (Test-Path -LiteralPath $packageZip) {
  Remove-Item -LiteralPath $packageZip -Force
}

Write-Host "Building admin UI..."
Push-Location (Join-Path $serverDir "admin-ui")
try {
  npm ci
  npm run build
} finally {
  Pop-Location
}

Write-Host "Building release server..."
Push-Location $serverDir
try {
  cargo build --release --locked
} finally {
  Pop-Location
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
New-Item -ItemType Directory -Force -Path $packageDir | Out-Null
$postgresDestination = Join-Path $packageDir "postgresql"
New-Item -ItemType Directory -Force -Path $postgresDestination | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $packageDir "admin-ui") | Out-Null

Copy-Item -LiteralPath (Join-Path $serverDir "target\release\madlibrary-server.exe") -Destination $packageDir
Copy-Item -Path (Join-Path $serverDir "admin-ui\dist\*") -Destination (Join-Path $packageDir "admin-ui") -Recurse
Copy-Item -LiteralPath (Join-Path $serverDir "packaging\windows\start-server.cmd") -Destination $packageDir
Copy-Item -LiteralPath (Join-Path $serverDir "packaging\windows\README.txt") -Destination $packageDir

Write-Host "Copying tracked PostgreSQL $($postgresManifest.version) runtime..."
$requiredRuntimeFiles = @()
foreach ($name in $postgresManifest.requiredBinFiles) {
  $requiredRuntimeFiles += "bin\$name"
}
$requiredRuntimeFiles += $postgresManifest.requiredFiles
foreach ($relativePath in $requiredRuntimeFiles) {
  $sourcePath = Join-Path $postgresSource $relativePath
  if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "Tracked PostgreSQL runtime is incomplete; missing: $sourcePath"
  }
}
Copy-Item -Path (Join-Path $postgresSource "*") -Destination $postgresDestination -Recurse

Write-Host "Creating portable ZIP..."
Compress-Archive -LiteralPath $packageDir -DestinationPath $packageZip -CompressionLevel Optimal
Remove-Item -LiteralPath $stagingDir -Recurse -Force
if ((Test-Path -LiteralPath $packageWorkDir) -and
    -not (Get-ChildItem -LiteralPath $packageWorkDir -Force)) {
  Remove-Item -LiteralPath $packageWorkDir -Force
}

Write-Host ""
Write-Host "Portable package created:"
Write-Host $packageZip
