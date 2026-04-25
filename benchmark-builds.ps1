# Build Performance Benchmark Script
# Tests sccache and lld-link across all build configurations
# Usage:   .\benchmark-builds.ps1       (full: dev + release)
#          .\benchmark-builds.ps1 -Quick  (dev only)

param([switch]$Quick)

$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
$results = @()
$configFile = "C:\My Script\auto-rust\.cargo\config.toml"
$startTime = Get-Date
$threadCount = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors | Measure-Object -Sum | Select-Object -ExpandProperty Sum
$ramGB = [math]::Round(((Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property Capacity -Sum).Sum) / 1GB, 0)

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " $Title" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
}

function Set-CargoConfig {
    param([bool]$Sccache, [bool]$LldLink)
    $sc = if ($Sccache) { 'rustc-wrapper = "sccache"' } else { '# rustc-wrapper = "sccache"' }
    $ll = if ($LldLink) { 'linker = "lld-link.exe"' } else { '# linker = "lld-link.exe"' }
    $config = @"
# Build Performance Benchmark Config

[build]
jobs = 32
$sc

[target.x86_64-pc-windows-msvc]
$ll

[profile.dev]
debug = "line-tables-only"
opt-level = 1
lto = "off"
codegen-units = 32
incremental = true

[profile.dev.build-override]
opt-level = 3
codegen-units = 16

[profile.release]
lto = "thin"
codegen-units = 32
debug = "line-tables-only"
opt-level = 3

[profile.release.build-override]
opt-level = 3
codegen-units = 16

[profile.fast-release]
inherits = "release"
lto = "off"
codegen-units = 128
opt-level = 2
"@
    Set-Content -Path $configFile -Value $config -Encoding UTF8
}

function Clear-Sccache {
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cmd"; $psi.Arguments = "/c sccache --stop-server"
        $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
        $null = [System.Diagnostics.Process]::Start($psi)
    } catch {}
    try {
        $psi2 = New-Object System.Diagnostics.ProcessStartInfo
        $psi2.FileName = "cmd"; $psi2.Arguments = "/c sccache --zero-stats"
        $psi2.UseShellExecute = $false; $psi2.CreateNoWindow = $true
        $null = [System.Diagnostics.Process]::Start($psi2)
    } catch {}
    $cacheDir = "$env:LOCALAPPDATA\Mozilla\sccache\cache"
    if (Test-Path $cacheDir) {
        Remove-Item -Path "$cacheDir\*" -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Stop-Sccache {
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cmd"; $psi.Arguments = "/c sccache --stop-server"
        $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
        $null = [System.Diagnostics.Process]::Start($psi)
    } catch {}
}

function Start-SccacheServer {
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cmd"; $psi.Arguments = "/c sccache --start-server"
        $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
        $null = [System.Diagnostics.Process]::Start($psi)
    } catch {}
}

function Get-SccacheStats {
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cmd"; $psi.Arguments = "/c sccache --show-stats"
        $psi.RedirectStandardOutput = $true; $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
        $p = [System.Diagnostics.Process]::Start($psi)
        $out = $p.StandardOutput.ReadToEnd()
        $p.WaitForExit()
        $hits = (($out -split "`n" | Where-Object { $_ -match "Cache hits rate" }) -split ":")[1].Trim()
        $size = (($out -split "`n" | Where-Object { $_ -match "^Cache size" }) -split ":").Trim()[1]
        return @{ Hits = $hits; Size = $size }
    } catch { return @{ Hits = "N/A"; Size = "N/A" } }
}

function Invoke-CargoClean {
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cmd"; $psi.Arguments = "/c cargo clean"
        $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
        $p = [System.Diagnostics.Process]::Start($psi)
        $p.WaitForExit()
    } catch {}
}

function Measure-Build {
    param([string]$Profile, [bool]$WithSccache, [string]$Description)
    Write-Host "  Building $Profile " -NoNewline

    if ($WithSccache) {
        $env:RUSTC_WRAPPER = "sccache"
        Start-SccacheServer
    } else {
        $env:RUSTC_WRAPPER = $null
        Stop-Sccache
    }

    $exe = "cmd"; $args = "/c cargo build" + $(if ($Profile -eq "release") { " --release" })
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe; $psi.Arguments = $args
    $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true

    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $p.WaitForExit()
    $sw.Stop()
    $elapsed = $sw.Elapsed.TotalSeconds
    $exitCode = $p.ExitCode

    if ($exitCode -ne 0) {
        Write-Host "FAILED ($([math]::Round($elapsed, 1))s)" -ForegroundColor Red
        $script:results += @{ Time = $elapsed; Profile = $Profile; Description = $Description; CacheHits = "ERR"; CacheSize = "ERR"; Success = $false }
        return
    }
    Write-Host ("{0,6}s" -f [math]::Round($elapsed, 1)) -ForegroundColor Green
    $stats = Get-SccacheStats
    $script:results += @{ Time = $elapsed; Profile = $Profile; Description = $Description; CacheHits = $stats.Hits; CacheSize = $stats.Size; Success = $true }
}

# ============================================================================
# HEADER
# ============================================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " BUILD PERFORMANCE BENCHMARK" -ForegroundColor Cyan
Write-Host (" {0} | {1} threads, {2}GB RAM" -f (Get-Date -Format "HH:mm:ss"), $threadCount, $ramGB) -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host ("Mode: " + $(if ($Quick) { "QUICK (dev only)" } else { "FULL (dev + release)" })) -ForegroundColor Yellow
$env:RUSTC_WRAPPER = $null
Stop-Sccache

# ============================================================================
# CONFIG 1: BASELINE
# ============================================================================
Write-Section "CONFIG 1: BASELINE (sccache=OFF, lld=OFF)"
Set-CargoConfig -Sccache $false -LldLink $false
Invoke-CargoClean
Measure-Build -Profile "dev" -WithSccache $false -Description "BASELINE dev COLD"
Measure-Build -Profile "dev" -WithSccache $false -Description "BASELINE dev WARM"
if (-not $Quick) {
    Measure-Build -Profile "release" -WithSccache $false -Description "BASELINE release COLD"
    Measure-Build -Profile "release" -WithSccache $false -Description "BASELINE release WARM"
}

# ============================================================================
# CONFIG 2: SCCACHE ONLY
# ============================================================================
Write-Section "CONFIG 2: SCCACHE ONLY (sccache=ON, lld=OFF)"
Set-CargoConfig -Sccache $true -LldLink $false
Clear-Sccache
Invoke-CargoClean
Measure-Build -Profile "dev" -WithSccache $true -Description "SCCACHE dev COLD"
Measure-Build -Profile "dev" -WithSccache $true -Description "SCCACHE dev WARM"
if (-not $Quick) {
    Measure-Build -Profile "release" -WithSccache $true -Description "SCCACHE release COLD"
    Measure-Build -Profile "release" -WithSccache $true -Description "SCCACHE release WARM"
}

# ============================================================================
# CONFIG 3: LLD-LINK ONLY
# ============================================================================
Write-Section "CONFIG 3: LLD-LINK ONLY (sccache=OFF, lld=ON)"
Set-CargoConfig -Sccache $false -LldLink $true
$env:RUSTC_WRAPPER = $null
Stop-Sccache
Invoke-CargoClean
Measure-Build -Profile "dev" -WithSccache $false -Description "LLD dev COLD"
Measure-Build -Profile "dev" -WithSccache $false -Description "LLD dev WARM"
if (-not $Quick) {
    Measure-Build -Profile "release" -WithSccache $false -Description "LLD release COLD"
    Measure-Build -Profile "release" -WithSccache $false -Description "LLD release WARM"
}

# ============================================================================
# CONFIG 4: BOTH
# ============================================================================
Write-Section "CONFIG 4: BOTH (sccache=ON, lld=ON)"
Set-CargoConfig -Sccache $true -LldLink $true
Clear-Sccache
Invoke-CargoClean
Measure-Build -Profile "dev" -WithSccache $true -Description "BOTH dev COLD"
Measure-Build -Profile "dev" -WithSccache $true -Description "BOTH dev WARM"
if (-not $Quick) {
    Measure-Build -Profile "release" -WithSccache $true -Description "BOTH release COLD"
    Measure-Build -Profile "release" -WithSccache $true -Description "BOTH release WARM"
}

# ============================================================================
# RESULTS
# ============================================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " RESULTS SUMMARY" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$configs = @("BASELINE", "SCCACHE", "LLD", "BOTH")
$phases = @("COLD", "WARM")

foreach ($profile in @("dev", "release")) {
    $profileResults = $results | Where-Object { $_.Profile -eq $profile -and $_.Success }
    if (-not $profileResults) { continue }
    Write-Host ""
    Write-Host ("--- {0} ---" -f $profile.ToUpper()) -ForegroundColor Yellow
    Write-Host ("  {0,-26} {1,7} {2,14} {3,12}" -f "Config", "Time(s)", "Cache Hits", "vs Baseline")
    Write-Host "  " + ("-" * 64)

    $baselineCold = $profileResults | Where-Object { $_.Description -match "BASELINE.*COLD" } | Select-Object -First 1
    $baselineTime = if ($baselineCold) { $baselineCold.Time } else { 0 }

    foreach ($desc in ($profileResults | Select-Object -ExpandProperty Description | Sort-Object)) {
        $r = $profileResults | Where-Object { $_.Description -eq $desc } | Select-Object -First 1
        $pct = if ($baselineTime -gt 0) {
            $delta = ($baselineTime - $r.Time) / $baselineTime * 100
            if ($delta -gt 0.5) { "+" + [math]::Round($delta, 1) + "%" }
            elseif ($delta -lt -0.5) { [math]::Round($delta, 1) + "%" }
            else { "0%" }
        } else { "-" }
        $short = $desc -replace "BASELINE |SCCACHE |LLD |BOTH "
        Write-Host ("  {0,-26} {1,7} {2,14} {3,12}" -f $short, [math]::Round($r.Time, 1), $r.CacheHits, $pct)
    }
}

# RECOMMENDATION
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " RECOMMENDATIONS" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$devWarm = $results | Where-Object { $_.Profile -eq "dev" -and $_.Success -and $_.Description -match "WARM" }
$relWarm = $results | Where-Object { $_.Profile -eq "release" -and $_.Success -and $_.Description -match "WARM" }
$bestDev = ($devWarm | Sort-Object Time | Select-Object -First 1)
$bestRel = ($relWarm | Sort-Object Time | Select-Object -First 1)

Write-Host ""
Write-Host ("Best dev WARM:       {0}  -- {1}s" -f $bestDev.Description, [math]::Round($bestDev.Time, 1)) -ForegroundColor Green
if ($bestRel) { Write-Host ("Best release WARM:   {0} -- {1}s" -f $bestRel.Description, [math]::Round($bestRel.Time, 1)) -ForegroundColor Green }

# Verdict per config
Write-Host ""
foreach ($cfg in $configs) {
    $devBaseline = $results | Where-Object { $_.Profile -eq "dev" -and $_.Description -match "BASELINE.*WARM" }
    $devCfgWarm = $results | Where-Object { $_.Profile -eq "dev" -and $_.Description -match "$cfg.*WARM" }
    if ($devBaseline -and $devCfgWarm) {
        $diff = ($devCfgWarm.Time - $devBaseline.Time) / $devBaseline.Time * 100
        $label = switch ($cfg) {
            "BASELINE" { "Dev (no accelerators)" }
            "SCCACHE"  { "Dev (sccache only)" }
            "LLD"      { "Dev (lld-link only)" }
            "BOTH"     { "Dev (sccache + lld)" }
        }
        if ($diff -gt 5) {
            Write-Host ("  SLOWER: {0}  ({1}+{2}%) vs baseline" -f $label, [math]::Round($diff, 1)) -ForegroundColor Red
        } elseif ($diff -lt -5) {
            Write-Host ("  FASTER: {0}  ({1}% faster) vs baseline" -f $label, [math]::Round([Math]::Abs($diff), 1)) -ForegroundColor Green
        } else {
            Write-Host ("  NEUTRAL: {0}  ({1}% vs baseline)" -f $label, [math]::Round($diff, 1)) -ForegroundColor Yellow
        }
    }
}

Write-Host ""
$elapsed = Get-Date - $startTime
Write-Host ("Total benchmark time: {0}m {1}s" -f [math]::Floor($elapsed.TotalMinutes), [math]::Round($elapsed.Seconds)) -ForegroundColor Gray

Stop-Sccache
$results | Where-Object { $_.Success } | Export-Csv -Path "C:\My Script\auto-rust\benchmark-results.csv" -NoTypeInformation -Encoding UTF8
Write-Host "Results exported to benchmark-results.csv" -ForegroundColor Gray