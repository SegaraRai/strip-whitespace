param(
  [Parameter(Mandatory = $true)]
  [string]$Target,

  [int]$Runs = 0,

  [string]$Toolchain = "nightly"
)

$ErrorActionPreference = "Stop"

$asanDll = Get-ChildItem "$env:ProgramFiles\Microsoft Visual Studio" -Recurse -ErrorAction SilentlyContinue -Filter "clang_rt.asan_dynamic-x86_64.dll" |
  Select-Object -First 1

if (-not $asanDll) {
  throw "Could not find clang_rt.asan_dynamic-x86_64.dll under $env:ProgramFiles\\Microsoft Visual Studio"
}

$env:PATH = "$($asanDll.Directory.FullName);$env:PATH"

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
  if ($Runs -gt 0) {
    $runArg = "-runs=$Runs"
    & cargo "+$Toolchain" fuzz run $Target -- $runArg
  } else {
    & cargo "+$Toolchain" fuzz run $Target
  }
}
finally {
  Pop-Location
}
