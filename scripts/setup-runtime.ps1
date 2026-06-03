param(
  [string]$RuntimeDir = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $RuntimeDir) {
  $RuntimeDir = Join-Path $repoRoot "Runtime\Python312"
}

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
  throw "uv was not found. Install uv before running this script."
}

function Invoke-Checked {
  param([scriptblock]$Command)
  & $Command
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
}

Invoke-Checked { uv python install 3.12 }
$pythonExe = (& uv python find 3.12).Trim()
if ($LASTEXITCODE -ne 0 -or -not $pythonExe) {
  throw "Python 3.12 was not found after uv install."
}

$pythonHome = Split-Path -Parent $pythonExe
if (-not (Test-Path -LiteralPath (Join-Path $pythonHome "python.exe"))) {
  throw "Python source is incomplete: $pythonHome"
}

if (Test-Path -LiteralPath $RuntimeDir) {
  Remove-Item -LiteralPath $RuntimeDir -Recurse -Force
}
New-Item -ItemType Directory -Path $RuntimeDir -Force | Out-Null
Get-ChildItem -LiteralPath $pythonHome -Force | ForEach-Object {
  Copy-Item -LiteralPath $_.FullName -Destination $RuntimeDir -Recurse -Force
}

$packagedPython = Join-Path $RuntimeDir "python.exe"
$version = & $packagedPython --version
if ($LASTEXITCODE -ne 0 -or $version -notmatch "^Python 3\.12\.") {
  throw "Runtime verification failed: $packagedPython"
}

Write-Host "UGCAudit Python runtime ready: $RuntimeDir ($version)"
