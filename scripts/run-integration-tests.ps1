<#
.SYNOPSIS
    Run browser-backed integration tests locally or in CI.

.DESCRIPTION
    Launches a headless Chromium-based browser on port 9222, sets
    TASK_API_TEST_WS, and runs all #[ignore] integration tests.

    Supported browsers (checked in order): Chrome, Brave, Edge, Chromium.

.PARAMETER Port
    CDP debugging port (default: 9222).

.PARAMETER NoLaunch
    Skip browser launch — use an already-running browser on Port.

.PARAMETER TestFilter
    Optional substring filter for test names (e.g. "query" runs query tests only).

.EXAMPLE
    .\scripts\run-integration-tests.ps1
    .\scripts\run-integration-tests.ps1 -Port 9002
    .\scripts\run-integration-tests.ps1 -TestFilter query
    .\scripts\run-integration-tests.ps1 -TimeoutSec 120
#>

param(
    [int]    $Port       = 9222,
    [int]    $TimeoutSec = 60,
    [switch] $NoLaunch,
    [switch] $IncludeOrchestrator,
    [string] $TestFilter = ""
)

$ErrorActionPreference = "Stop"
$proc = $null
$hostAddr = "127.0.0.1"
$wsUrl = "ws://${hostAddr}:$Port"

# ── 1. Locate and launch browser ─────────────────────────────────────────────
if (-not $NoLaunch) {
    $browserPaths = @(
        "${env:ProgramFiles}\Google\Chrome\Application\chrome.exe",
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
        "${env:LOCALAPPDATA}\BraveSoftware\Brave-Browser\Application\brave.exe",
        "${env:ProgramFiles}\BraveSoftware\Brave-Browser\Application\brave.exe",
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
        "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
    )

    $browserExe = $null
    foreach ($path in $browserPaths) {
        if (Test-Path -LiteralPath $path) {
            $browserExe = $path
            break
        }
    }

    if (-not $browserExe) {
        Write-Warning "No supported browser found. Install Chrome, Brave, or Edge."
        Write-Warning "Falling back to manual browser setup."
        Write-Warning "Set TASK_API_TEST_WS=$wsUrl and run tests directly."
        exit 1
    }

    Write-Host "Launching: $browserExe (headless, port $Port)" -ForegroundColor Cyan
    $proc = Start-Process -FilePath $browserExe -ArgumentList @(
        "--headless",
        "--remote-debugging-port=$Port",
        "--no-first-run",
        "--disable-gpu",
        "--disable-extensions",
        "--disable-sync",
        "--no-sandbox",
        "--disable-dev-shm-usage"
    ) -PassThru -WindowStyle Hidden

    Write-Host "Waiting for browser to be ready..." -ForegroundColor Cyan
    $ready = $false
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $resp = Invoke-WebRequest -Uri "http://${hostAddr}:$Port/json/version" -TimeoutSec 2 -ErrorAction Stop
            if ($resp.StatusCode -eq 200) { $ready = $true; break }
        } catch { }
        Start-Sleep -Milliseconds 500
    }

    if (-not $ready) {
        Write-Error "Browser did not respond on port $Port within 15s."
        if ($proc -and -not $proc.HasExited) { $proc.Kill() }
        exit 1
    }

    Write-Host "Browser ready on port $Port" -ForegroundColor Green
}

# ── 2. Set env var ───────────────────────────────────────────────────────────
$env:TASK_API_TEST_WS = $wsUrl
Write-Host "TASK_API_TEST_WS=$wsUrl" -ForegroundColor Cyan

# ── 3. Determine test targets ────────────────────────────────────────────────
$testFiles = @(
    "navigation_integration",
    "task_context_integration"
)

if ($IncludeOrchestrator) {
    $testFiles += "orchestrator_integration"
}

# orchestrator_integration uses discover_browsers() which requires configured
# browser profiles. Use -IncludeOrchestrator only after setting up profiles.

$extraArgs = if ($TestFilter) { @("--", "--ignored", $TestFilter, "--test-threads=1") }
             else            { @("--", "--ignored", "--test-threads=1") }

$allPassed = $true

foreach ($file in $testFiles) {
    Write-Host "`n>>> Running: $file (timeout: ${TimeoutSec}s)" -ForegroundColor Yellow

    $cmdStr = "test --test $file $($extraArgs -join ' ')"
    $outFile = [System.IO.Path]::GetTempFileName()
    $codeFile = [System.IO.Path]::GetTempFileName()
    $job = Start-Job -ScriptBlock {
        param($a, $of, $cf)
        Invoke-Expression "cargo $a" *> $of
        Set-Content -Path $cf -Value $LASTEXITCODE
    } -ArgumentList $cmdStr, $outFile, $codeFile

    $finished = $job | Wait-Job -Timeout $TimeoutSec

    if ($finished) {
        if (Test-Path $outFile) { Get-Content $outFile -ErrorAction SilentlyContinue }
        $exitCode = 1
        if (Test-Path $codeFile) {
            $rawCode = Get-Content $codeFile -ErrorAction SilentlyContinue
            if ($rawCode) { $exitCode = [int]$rawCode }
        }
        Remove-Job $job -ErrorAction SilentlyContinue
        if ($exitCode -ne 0) {
            Write-Host "FAIL: $file" -ForegroundColor Red
            $allPassed = $false
        } else {
            Write-Host "PASS: $file" -ForegroundColor Green
        }
    } else {
        Write-Host "TIMEOUT: $file exceeded ${TimeoutSec}s - skipping" -ForegroundColor Red
        Stop-Job $job -ErrorAction SilentlyContinue
        Remove-Job $job -Force -ErrorAction SilentlyContinue
        $allPassed = $false
    }

    Remove-Item $outFile -Force -ErrorAction SilentlyContinue
    Remove-Item $codeFile -Force -ErrorAction SilentlyContinue
}

# ── 4. Cleanup ───────────────────────────────────────────────────────────────
if (-not $NoLaunch -and $proc -and -not $proc.HasExited) {
    Write-Host "`nKilling browser process..." -ForegroundColor Cyan
    $proc.Kill()
}

if ($allPassed) {
    Write-Host "`nAll integration tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`nSome integration tests failed." -ForegroundColor Red
    exit 1
}
