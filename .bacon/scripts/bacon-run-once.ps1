param(
    [Parameter(Mandatory = $true)]
    [string]$LogFile,

    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"

$logDir = Split-Path -Parent $LogFile
if ($logDir) {
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null
} else {
    $logDir = "."
}

function Write-RunLog {
    param([string]$Message)
    Add-Content -LiteralPath $LogFile -Encoding UTF8 -Value ("[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Message)
}

$cargoPath = (& rustup which cargo 2>$null)
if ([string]::IsNullOrWhiteSpace($cargoPath)) {
    $cargoPath = "cargo"
}

$stamp = Get-Date -Format "yyyyMMdd_HHmmss_ffff"
$stdoutFile = Join-Path $logDir "bacon-stdout_$stamp.tmp"
$stderrFile = Join-Path $logDir "bacon-stderr_$stamp.tmp"
$process = $null

try {
    Write-RunLog "Starting cargo run --bin bacon -- --auto ..."
    Write-RunLog "Cargo executable: $cargoPath"
    Write-RunLog "Timeout: $TimeoutSeconds seconds"

    $escapedCargo = $cargoPath.Replace('"', '""')
    $escapedStdout = (Resolve-Path -LiteralPath $logDir).Path + "\" + (Split-Path -Leaf $stdoutFile)
    $escapedStderr = (Resolve-Path -LiteralPath $logDir).Path + "\" + (Split-Path -Leaf $stderrFile)
    $escapedStdout = $escapedStdout.Replace('"', '""')
    $escapedStderr = $escapedStderr.Replace('"', '""')
    $command = "`"$escapedCargo`" run --bin bacon -- --auto > `"$escapedStdout`" 2> `"$escapedStderr`""

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "cmd.exe"
    $startInfo.Arguments = "/d /s /c `"$command`""
    $startInfo.WorkingDirectory = (Get-Location).Path
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo

    if (-not $process.Start()) {
        Write-RunLog "Failed to start cargo process"
        exit 1
    }

    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        Write-RunLog "Timed out after $TimeoutSeconds seconds. Terminating process tree for PID $($process.Id)."
        & taskkill /PID $process.Id /T /F | ForEach-Object { Write-RunLog $_ }
        $process.WaitForExit()

        if (Test-Path -LiteralPath $stdoutFile) {
            Get-Content -LiteralPath $stdoutFile | Add-Content -LiteralPath $LogFile -Encoding UTF8
        }
        if (Test-Path -LiteralPath $stderrFile) {
            Get-Content -LiteralPath $stderrFile | Add-Content -LiteralPath $LogFile -Encoding UTF8
        }

        Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
        exit 124
    }

    if (Test-Path -LiteralPath $stdoutFile) {
        Get-Content -LiteralPath $stdoutFile | Add-Content -LiteralPath $LogFile -Encoding UTF8
    }
    if (Test-Path -LiteralPath $stderrFile) {
        Get-Content -LiteralPath $stderrFile | Add-Content -LiteralPath $LogFile -Encoding UTF8
    }

    $exitCode = [int]$process.ExitCode
    Write-RunLog "Process exited with code $exitCode"
    Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
    exit $exitCode
}
finally {
    if ($null -ne $process) {
        $process.Dispose()
    }
    Remove-Item -LiteralPath $stdoutFile, $stderrFile -Force -ErrorAction SilentlyContinue
}
