# Build Performance Benchmark Script
# Measures baseline vs lld-link across dev/release.
# Cargo benchmark harness source lives in src/benchmarks/; this script only measures build performance.
# Usage: .\benchmark-builds.ps1
#        .\benchmark-builds.ps1 -Quick

param([switch]$Quick)

$repoRoot = $PSScriptRoot
$configFile = Join-Path $repoRoot ".cargo\config.toml"
$configBackup = "$configFile.bak"
$resultsDir = Join-Path $repoRoot "target\reports\build-performance"
$resultsFile = Join-Path $resultsDir "benchmark-results.csv"
$results = @()
$startTime = Get-Date
$threadCount = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors | Measure-Object -Sum | Select-Object -ExpandProperty Sum
$ramGB = [math]::Round(((Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property Capacity -Sum).Sum) / 1GB, 0)

$script:originalConfigExisted = Test-Path $configFile
if ($script:originalConfigExisted) {
    Copy-Item -LiteralPath $configFile -Destination $configBackup -Force
}

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
}

function Set-CargoConfig {
    param([bool]$LldLink)
    $ll = if ($LldLink) { 'linker = "C:\\Program Files\\LLVM\\bin\\lld-link.exe"' } else { '# linker = "C:\\Program Files\\LLVM\\bin\\lld-link.exe"' }
    $config = @"
# Build Performance Benchmark Config

[build]
jobs = 16

[target.x86_64-pc-windows-msvc]
$ll

[profile.dev]
debug = "none"
opt-level = 0
lto = "off"
codegen-units = 16
incremental = true
overflow-checks = false

[profile.dev.package."*"]
debug = false
opt-level = 0

[profile.dev.build-override]
opt-level = 3
codegen-units = 16

[profile.release]
lto = "thin"
codegen-units = 1
debug = false
opt-level = 3
strip = "debuginfo"
panic = "unwind"
overflow-checks = false

[profile.release.build-override]
opt-level = 3
codegen-units = 16

[profile.fast-release]
inherits = "release"
lto = "off"
codegen-units = 64
opt-level = 2
"@
    Set-Content -LiteralPath $configFile -Value $config -Encoding UTF8
}

function Restore-CargoConfig {
    if ($script:originalConfigExisted) {
        Move-Item -LiteralPath $configBackup -Destination $configFile -Force
        return
    }

    if (Test-Path $configFile) {
        Remove-Item -LiteralPath $configFile -Force
    }
}

function Invoke-CargoClean {
    cargo clean | Out-Null
}

function Measure-Build {
    param([string]$Profile, [string]$Description)
    Write-Host "  Building $Profile " -NoNewline

    $args = if ($Profile -eq "release") { "build --release" } else { "build" }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    cargo $args | Out-Null
    $sw.Stop()
    $elapsed = $sw.Elapsed.TotalSeconds

    if ($LASTEXITCODE -ne 0) {
        Write-Host ("FAILED ({0}s)" -f [math]::Round($elapsed, 1)) -ForegroundColor Red
        $script:results += @{ Time = $elapsed; Profile = $Profile; Description = $Description; Success = $false }
        return
    }

    Write-Host ("{0,6}s" -f [math]::Round($elapsed, 1)) -ForegroundColor Green
    $script:results += @{ Time = $elapsed; Profile = $Profile; Description = $Description; Success = $true }
}

try {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " BUILD PERFORMANCE BENCHMARK" -ForegroundColor Cyan
    Write-Host (" {0} | {1} threads, {2}GB RAM" -f (Get-Date -Format "HH:mm:ss"), $threadCount, $ramGB) -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ("Mode: " + $(if ($Quick) { "QUICK (dev only)" } else { "FULL (dev + release)" })) -ForegroundColor Yellow

    Write-Section "CONFIG 1: BASELINE (lld=OFF)"
    Set-CargoConfig -LldLink $false
    Invoke-CargoClean
    Measure-Build -Profile "dev" -Description "BASELINE dev COLD"
    Measure-Build -Profile "dev" -Description "BASELINE dev WARM"
    if (-not $Quick) {
        Measure-Build -Profile "release" -Description "BASELINE release COLD"
        Measure-Build -Profile "release" -Description "BASELINE release WARM"
    }

    Write-Section "CONFIG 2: LLD-LINK (lld=ON)"
    Set-CargoConfig -LldLink $true
    Invoke-CargoClean
    Measure-Build -Profile "dev" -Description "LLD dev COLD"
    Measure-Build -Profile "dev" -Description "LLD dev WARM"
    if (-not $Quick) {
        Measure-Build -Profile "release" -Description "LLD release COLD"
        Measure-Build -Profile "release" -Description "LLD release WARM"
    }

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " RESULTS SUMMARY" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan

    foreach ($profile in @("dev", "release")) {
        $profileResults = $results | Where-Object { $_.Profile -eq $profile -and $_.Success }
        if (-not $profileResults) { continue }
        Write-Host ""
        Write-Host ("--- {0} ---" -f $profile.ToUpper()) -ForegroundColor Yellow
        Write-Host ("  {0,-24} {1,7} {2,12}" -f "Config", "Time(s)", "vs Baseline")
        Write-Host "  " + ("-" * 48)

        $baselineCold = $profileResults | Where-Object { $_.Description -match "BASELINE.*COLD" } | Select-Object -First 1
        $baselineTime = if ($baselineCold) { $baselineCold.Time } else { 0 }

        foreach ($desc in ($profileResults | Select-Object -ExpandProperty Description | Sort-Object)) {
            $r = $profileResults | Where-Object { $_.Description -eq $desc } | Select-Object -First 1
            $pct = if ($baselineTime -gt 0) {
                $delta = ($baselineTime - $r.Time) / $baselineTime * 100
                if ($delta -gt 0.5) { "+" + ([math]::Round($delta, 1).ToString()) + "%" }
                elseif ($delta -lt -0.5) { ([math]::Round($delta, 1).ToString()) + "%" }
                else { "0%" }
            } else { "-" }
            Write-Host ("  {0,-24} {1,7} {2,12}" -f $desc, [math]::Round($r.Time, 1), $pct)
        }
    }
}
finally {
    Restore-CargoConfig
}

$elapsed = (Get-Date) - $startTime
Write-Host ""
Write-Host ("Total benchmark time: {0}m {1}s" -f [math]::Floor($elapsed.TotalMinutes), $elapsed.Seconds) -ForegroundColor Gray

New-Item -ItemType Directory -Path $resultsDir -Force | Out-Null
$results | Where-Object { $_.Success } | Export-Csv -Path $resultsFile -NoTypeInformation -Encoding UTF8
Write-Host ("Results exported to {0}" -f $resultsFile) -ForegroundColor Gray
