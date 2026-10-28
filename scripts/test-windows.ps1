param([string]$Distribution = '', [switch]$SkipGui)

$ErrorActionPreference = 'Stop'
if (!$Distribution) {
    $Distribution = if (Test-Path (Join-Path $PSScriptRoot 'RetinaCare.exe')) {
        $PSScriptRoot
    } else {
        Join-Path $PSScriptRoot '../dist/RetinaCare'
    }
}
$Distribution = (Resolve-Path $Distribution).Path
$Exe = Join-Path $Distribution 'RetinaCare.exe'
$Python = Join-Path $Distribution 'runtime/python.exe'
$Smoke = Join-Path $Distribution 'windows-smoke-test.json'
$GuiSmoke = Join-Path $Distribution 'windows-gui-smoke-test.json'

foreach ($Path in @($Exe, $Python, (Join-Path $Distribution 'resources/ai_engine/src/retinacare_ai/worker.py'))) {
    if (!(Test-Path $Path -PathType Leaf)) { throw "Missing packaged file: $Path" }
}

$before = @(Get-Process python -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Python } | Select-Object -ExpandProperty Id)
$process = Start-Process -FilePath $Exe -ArgumentList '--smoke-test', "`"$Smoke`"" -PassThru -Wait
if ($process.ExitCode -ne 0) { throw "Application smoke test exited $($process.ExitCode)" }
$report = Get-Content $Smoke -Raw | ConvertFrom-Json
if (!$report.passed -or !$report.encrypted_database -or $report.worker_health.status -ne 'ok') {
    throw 'Application, encrypted database, or worker health smoke test failed'
}

if (!$SkipGui) {
    $process = Start-Process -FilePath $Exe -ArgumentList '--gui-smoke-test', "`"$GuiSmoke`"" -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "GUI smoke test exited $($process.ExitCode)" }
    $gui = Get-Content $GuiSmoke -Raw | ConvertFrom-Json
    if (!$gui.passed -or $gui.navigation_screens -ne 8) { throw 'GUI lifecycle/navigation smoke test failed' }
}

Start-Sleep -Milliseconds 300
$after = @(Get-Process python -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Python -and $_.Id -notin $before })
if ($after.Count -ne 0) { throw "Packaged Python worker remained running: $($after.Id -join ', ')" }

Write-Host "PASS: $Exe"
Write-Host "Device: $($report.worker_health.data.device.device_name)"
Write-Host "CUDA available: $($report.worker_health.data.device.cuda_available)"
Write-Host "Model state: $($report.worker_health.data.model_state)"
Write-Host "Reports: $Smoke $(if (!$SkipGui) { $GuiSmoke })"
