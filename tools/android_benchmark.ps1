param(
    [ValidateSet('tablet','poco')][string]$Device = 'tablet',
    [string]$Session = 'tablet-console-20261104',
    [switch]$Unattended,
    [switch]$Demo,
    [switch]$RetryFailed,
    [switch]$ContinueOnError,
    [ValidateSet('compatibility','batching','opengl')][string[]]$Renderers = @('compatibility'),
    [string[]]$Cases
)
$ErrorActionPreference = 'Stop'
$benchmarkRoot = Split-Path -Parent $PSScriptRoot
$previousEncoding = $env:PYTHONIOENCODING
Push-Location -LiteralPath $benchmarkRoot
try {
    $env:PYTHONIOENCODING = 'utf-8'
    $benchmarkArgs = @('tools/android_benchmark_console.py', '--device', $Device, '--session', $Session)
    $benchmarkArgs += '--renderers'; $benchmarkArgs += $Renderers
    if ($Unattended) { $benchmarkArgs += '--unattended' }
    if ($Demo) { $benchmarkArgs += '--demo' }
    if ($RetryFailed) { $benchmarkArgs += '--retry-failed' }
    if ($ContinueOnError) { $benchmarkArgs += '--continue-on-error' }
    if ($Cases) { $benchmarkArgs += '--cases'; $benchmarkArgs += $Cases }
    python @benchmarkArgs
    if ($LASTEXITCODE -ne 0) { throw "Benchmark exited with code $LASTEXITCODE" }
} finally {
    $env:PYTHONIOENCODING = $previousEncoding
    Pop-Location
}
