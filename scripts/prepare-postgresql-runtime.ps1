param(
  [string]$PostgresArchive
)

$ErrorActionPreference = "Stop"

$serverDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$manifestPath = Join-Path $serverDir "packaging\postgresql-windows-x64.json"
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
if (-not $PostgresArchive) {
  $PostgresArchive = Join-Path $serverDir "target\downloads\$($manifest.archiveFileName)"
}
if (-not (Test-Path -LiteralPath $PostgresArchive -PathType Leaf)) {
  throw "PostgreSQL archive was not found. Download $($manifest.sourceUrl) to $PostgresArchive or pass -PostgresArchive."
}
$PostgresArchive = (Resolve-Path -LiteralPath $PostgresArchive).Path
if ((Get-FileHash -Algorithm SHA256 -LiteralPath $PostgresArchive).Hash -ne $manifest.archiveSha256) {
  throw "PostgreSQL archive SHA-256 mismatch: $PostgresArchive"
}

$stagingRoot = Join-Path $serverDir "target\postgresql-runtime-prep"
$stagingRuntime = Join-Path $stagingRoot "windows-x64"
$destination = Join-Path $serverDir "binaries\windows-x64\postgresql"

function Assert-WorkspacePath([string]$Path) {
  $fullPath = [IO.Path]::GetFullPath($Path)
  $root = $serverDir.TrimEnd('\') + '\'
  if (-not $fullPath.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to modify a path outside the repository: $fullPath"
  }
}

Assert-WorkspacePath $stagingRoot
Assert-WorkspacePath $destination
foreach ($path in @($stagingRoot, $destination)) {
  if (Test-Path -LiteralPath $path) {
    Remove-Item -LiteralPath $path -Recurse -Force
  }
}
New-Item -ItemType Directory -Force -Path $stagingRuntime | Out-Null

function Export-ZipEntry($Entry, [string]$RelativePath) {
  if ($RelativePath -match '(^|[\\/])\.\.([\\/]|$)') {
    throw "Unsafe archive path: $RelativePath"
  }
  $outputPath = [IO.Path]::GetFullPath((Join-Path $stagingRuntime $RelativePath))
  $root = [IO.Path]::GetFullPath($stagingRuntime).TrimEnd('\') + '\'
  if (-not $outputPath.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to extract outside the runtime: $RelativePath"
  }
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPath) | Out-Null
  $input = $Entry.Open()
  $output = [IO.File]::Create($outputPath)
  try {
    $input.CopyTo($output)
  } finally {
    $output.Dispose()
    $input.Dispose()
  }
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($PostgresArchive)
try {
  foreach ($entry in $archive.Entries) {
    if (-not $entry.Name -or -not $entry.FullName.StartsWith("pgsql/")) {
      continue
    }
    $relative = $entry.FullName.Substring(6)
    $include = $false
    if ($relative.StartsWith("bin/")) {
      $include = $manifest.requiredBinFiles -contains $entry.Name
    } elseif ($relative -in @("lib/plpgsql.dll", "lib/dict_snowball.dll")) {
      $include = $true
    } elseif ($relative.StartsWith("share/")) {
      $include = -not $relative.StartsWith("share/locale/") -and
        (-not $relative.StartsWith("share/extension/") -or $entry.Name.StartsWith("plpgsql"))
    } elseif ($relative -in @("server_license.txt", "commandlinetools_3rd_party_licenses.txt")) {
      $include = $true
    }
    if ($include) {
      Export-ZipEntry $entry $relative
    }
  }
} finally {
  $archive.Dispose()
}

$vcRuntime = Get-ChildItem "${env:ProgramFiles}\Microsoft Visual Studio" -Recurse -Filter "vcruntime140.dll" -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -match '\\Redist\\MSVC\\[^\\]+\\x64\\Microsoft\.VC\d+\.CRT\\vcruntime140\.dll$' } |
  Sort-Object FullName -Descending |
  Select-Object -First 1
if (-not $vcRuntime) {
  throw "vcruntime140.dll was not found in the Visual Studio C++ redistributable files."
}
Copy-Item -LiteralPath $vcRuntime.FullName -Destination (Join-Path $stagingRuntime "bin")

$requiredRuntimeFiles = @()
foreach ($name in $manifest.requiredBinFiles) {
  $requiredRuntimeFiles += "bin\$name"
}
$requiredRuntimeFiles += $manifest.requiredFiles
foreach ($relativePath in $requiredRuntimeFiles) {
  $runtimePath = Join-Path $stagingRuntime $relativePath
  if (-not (Test-Path -LiteralPath $runtimePath -PathType Leaf)) {
    throw "Prepared PostgreSQL runtime is incomplete; missing: $runtimePath"
  }
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
Move-Item -LiteralPath $stagingRuntime -Destination $destination
Remove-Item -LiteralPath $stagingRoot -Recurse -Force

Write-Host "Tracked PostgreSQL runtime prepared at:"
Write-Host $destination
