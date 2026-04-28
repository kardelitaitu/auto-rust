$start = Get-Date
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

Write-Output "========================================"
Write-Output "Build Configuration Report"
Write-Output "========================================"

$totalThreads = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors | Measure-Object -Sum | Select-Object -ExpandProperty Sum
$totalRAM = [math]::Round(((Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property Capacity -Sum).Sum) / 1GB, 0)
Write-Output "System: $totalThreads threads, $totalRAM GB RAM"

$configPath = "C:\My Script\auto-rust\.cargo\config.toml"
$config = Get-Content $configPath -Raw

$linkerEnabled = $false
foreach ($line in ($config -split "`n")) {
    if ($line -match '^\s*linker\s*=\s*"C:\\Program Files\\LLVM\\bin\\lld-link\.exe"') { $linkerEnabled = $true; break }
}
Write-Output "Linker: $(if ($linkerEnabled) { 'LLD (enabled)' } else { 'MSVC (default)' })"

if ($config -match '(?m)^jobs\s*=\s*(\d+)') { $jobs = $Matches[1] } else { $jobs = "auto" }
Write-Output "Build jobs: $jobs"

$devCodegen = "default"; $releaseCodegen = "default"
if ($config -match '(?ms)\[profile\.dev\][^[]*?codegen-units\s*=\s*(\d+)') { $devCodegen = $Matches[1] }
if ($config -match '(?ms)\[profile\.release\][^[]*?codegen-units\s*=\s*(\d+)') { $releaseCodegen = $Matches[1] }
Write-Output "Codegen units: dev=$devCodegen, release=$releaseCodegen"

Write-Output "========================================"
Write-Output "Starting cargo build --release at $start"

Write-Output "Cleaning previous build..."
cargo clean 2>&1 | Out-Null

Write-Output "Running cargo build --release..."
cargo build --release 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Output "Build FAILED"
    exit 1
}

$end = Get-Date
$duration = $end - $start

Write-Output "========================================"
Write-Output "Build completed at $end"
Write-Output "Total time: $([math]::Round($duration.TotalSeconds, 1)) seconds"
