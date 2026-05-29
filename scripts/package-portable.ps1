$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

Push-Location $repoRoot
try {
  npm run build
}
finally {
  Pop-Location
}

$exePath = Join-Path $repoRoot "src-tauri\target\release\ugc-audit.exe"
if (-not (Test-Path -LiteralPath $exePath)) {
  throw "Release executable not found: $exePath"
}

$distRoot = Join-Path $repoRoot "dist-portable"
$appDir = Join-Path $distRoot "UGCAudit"
$zipPath = Join-Path $distRoot "UGCAudit-portable.zip"

$resolvedRepo = (Resolve-Path -LiteralPath $repoRoot).Path
$resolvedDistParent = (Resolve-Path -LiteralPath (Split-Path -Parent $distRoot)).Path
if (-not $resolvedDistParent.StartsWith($resolvedRepo, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to write outside repository: $distRoot"
}
New-Item -ItemType Directory -Path $distRoot -Force | Out-Null

foreach ($path in @($appDir, $zipPath)) {
  $parent = if (Test-Path -LiteralPath $path) {
    (Resolve-Path -LiteralPath $path).Path
  } else {
    (Resolve-Path -LiteralPath (Split-Path -Parent $path)).Path
  }

  if (-not $parent.StartsWith($resolvedRepo, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to write outside repository: $path"
  }
}

if (Test-Path -LiteralPath $appDir) {
  Remove-Item -LiteralPath $appDir -Recurse -Force
}
New-Item -ItemType Directory -Path $appDir | Out-Null

Copy-Item -LiteralPath $exePath -Destination (Join-Path $appDir "ugc-audit.exe")
Set-Content -LiteralPath (Join-Path $appDir "README_RUN.txt") -Encoding UTF8 -Value @(
  "UGCAudit portable demo",
  "",
  "Double click ugc-audit.exe to run.",
  "Windows WebView2 is required. Windows 11 usually already includes it."
)

if (Test-Path -LiteralPath $zipPath) {
  Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath $appDir -DestinationPath $zipPath

$copiedExe = Join-Path $appDir "ugc-audit.exe"
$stream = [System.IO.File]::OpenRead($copiedExe)
try {
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  try {
    $hashBytes = $sha256.ComputeHash($stream)
    $hashText = [BitConverter]::ToString($hashBytes).Replace("-", "")
  }
  finally {
    $sha256.Dispose()
  }
}
finally {
  $stream.Dispose()
}

Write-Host "Portable folder: $appDir"
Write-Host "Portable zip:    $zipPath"
Write-Host "Executable SHA256: $hashText"
