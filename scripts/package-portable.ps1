$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$publishRoot = Join-Path $repoRoot "Publish\UGCAudit"

Push-Location $repoRoot
try {
  npm run build
}
finally {
  Pop-Location
}

$exePath = Join-Path $publishRoot "cargo-target\release\ugc-audit.exe"
if (-not (Test-Path -LiteralPath $exePath)) {
  throw "Release executable not found: $exePath"
}

$appDir = Join-Path $publishRoot "App"
$zipPath = Join-Path $publishRoot "UGCAudit-portable.zip"

$resolvedRepo = (Resolve-Path -LiteralPath $repoRoot).Path
$resolvedDistParent = (Resolve-Path -LiteralPath (Split-Path -Parent $publishRoot)).Path
if (-not $resolvedDistParent.StartsWith($resolvedRepo, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to write outside repository: $publishRoot"
}
New-Item -ItemType Directory -Path $publishRoot -Force | Out-Null

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
New-Item -ItemType Directory -Path (Join-Path $appDir "Schemes") -Force | Out-Null

Copy-Item -LiteralPath $exePath -Destination (Join-Path $appDir "ugc-audit.exe")

function Resolve-PythonSource {
  if ($env:UGCAUDIT_PYTHON_SOURCE -and (Test-Path -LiteralPath (Join-Path $env:UGCAUDIT_PYTHON_SOURCE "python.exe"))) {
    return $env:UGCAUDIT_PYTHON_SOURCE
  }

  $repoRuntime = Join-Path $repoRoot "Runtime\Python312"
  if (Test-Path -LiteralPath (Join-Path $repoRuntime "python.exe")) {
    return $repoRuntime
  }

  if (Get-Command uv -ErrorAction SilentlyContinue) {
    $uvPython = (& uv python find 3.12 2>$null)
    if ($LASTEXITCODE -eq 0 -and $uvPython) {
      $candidate = Split-Path -Parent $uvPython.Trim()
      if (Test-Path -LiteralPath (Join-Path $candidate "python.exe")) {
        return $candidate
      }
    }
  }

  return $null
}

$pythonSource = Resolve-PythonSource
$runtimeRoot = Join-Path $appDir "Runtime"
$runtimePythonDir = Join-Path $runtimeRoot "Python312"

if (-not $pythonSource) {
  throw "Python 3.12 runtime source not found. Run npm run runtime:setup first, place Python at Runtime\Python312, install Python 3.12 with uv, or set UGCAUDIT_PYTHON_SOURCE to a Python folder containing python.exe."
}

New-Item -ItemType Directory -Path $runtimePythonDir -Force | Out-Null
Get-ChildItem -LiteralPath $pythonSource -Force | ForEach-Object {
  Copy-Item -LiteralPath $_.FullName -Destination $runtimePythonDir -Recurse -Force
}

$packagedPython = Join-Path $runtimePythonDir "python.exe"
if (-not (Test-Path -LiteralPath $packagedPython)) {
  throw "Packaged Python runtime is incomplete: $packagedPython"
}

$pythonVersion = & $packagedPython --version
if ($LASTEXITCODE -ne 0) {
  throw "Packaged Python runtime cannot start: $packagedPython"
}
if ($pythonVersion -notmatch "^Python 3\.12\.") {
  throw "Packaged Python runtime must be Python 3.12, got: $pythonVersion"
}

Set-Content -LiteralPath (Join-Path $appDir "README_RUN.txt") -Encoding UTF8 -Value @(
  "UGCAudit portable demo",
  "",
  "Double click ugc-audit.exe to run.",
  "Windows WebView2 is required. Windows 11 usually already includes it.",
  "",
  "Schemes stores audit scheme files created by the client.",
  "",
  "Runtime\Python312 contains the packaged Python runtime.",
  "Module dependencies are not packaged here. Set the dependency storage path in the client settings; installs and module runs will use that local path."
)

$cliDocSource = Join-Path $repoRoot "docs\CLI_USAGE.md"
if (Test-Path -LiteralPath $cliDocSource) {
  $cliDocName = "CLI$([char]0x4F7F)$([char]0x7528)$([char]0x8BF4)$([char]0x660E).md"
  Copy-Item -LiteralPath $cliDocSource -Destination (Join-Path $appDir $cliDocName) -Force
}

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
Write-Host "Python runtime:  $packagedPython ($pythonVersion)"
Write-Host "Executable SHA256: $hashText"
