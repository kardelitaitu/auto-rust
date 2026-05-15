# .bacon/scripts/bacon-service.ps1 - Bacon service lifecycle
# Usage: .\bacon-service.ps1 -Action start|stop|status|logs [-Tail 50]
# Dot-sources bacon-logging.ps1 for Write-BaconLog paths

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("start", "stop", "status", "logs")]
    [string]$Action,

    [Parameter(Mandatory=$false)]
    [int]$Tail = 50,

    [Parameter(Mandatory=$false)]
    [switch]$ScriptVerbose
)

# Load shared logging
. "$PSScriptRoot\bacon-logging.ps1"

# --- Functions ---

function Start-BaconOrchestration {
    Write-BaconLog "INFO" "Starting Bacon orchestration..." -Component "Service"

    # Quick sanity check before launching
    if (-not (Test-Path (Join-Path $BaconDir "bacon.toml"))) {
        Write-BaconLog "ERROR" "bacon.toml not found — cannot start" -Component "Service"
        return $false
    }
    try {
        $null = Get-Command "cargo" -ErrorAction Stop
    } catch {
        Write-BaconLog "ERROR" "cargo not found in PATH — cannot start" -Component "Service"
        return $false
    }

    $pidFile = Join-Path $SessionsDir "orchestrate.pid"
    if (Test-Path $pidFile) {
        try {
            $existingPid = Get-Content $pidFile -ErrorAction Stop
            $process = Get-Process -Id $existingPid -ErrorAction SilentlyContinue
            if ($process) {
                Write-BaconLog "WARN" "Bacon orchestration already running (PID: $existingPid)" -Component "Service"
                return $false
            }
        } catch {
            Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
        }
    }

    try {
        $process = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "bacon" -WorkingDirectory $ProjectRoot -PassThru -WindowStyle Hidden
        $process.Id | Out-File -FilePath $pidFile -Encoding UTF8
        Write-BaconLog "INFO" "Bacon orchestration started (PID: $($process.Id))" -Component "Service"
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to start Bacon orchestration: $($_.Exception.Message)" -Component "Service"
        return $false
    }
}

function Stop-BaconOrchestration {
    Write-BaconLog "INFO" "Stopping Bacon orchestration..." -Component "Service"

    $pidFile = Join-Path $SessionsDir "orchestrate.pid"

    if (-not (Test-Path $pidFile)) {
        Write-BaconLog "WARN" "No Bacon orchestration PID file found" -Component "Service"
        return $false
    }

    try {
        $processId = Get-Content $pidFile -ErrorAction Stop
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue

        if ($process) {
            $process.Kill()
            Write-BaconLog "INFO" "Bacon orchestration stopped (PID: $processId)" -Component "Service"
        } else {
            Write-BaconLog "WARN" "Process not found for PID: $processId" -Component "Service"
        }

        Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to stop Bacon orchestration: $($_.Exception.Message)" -Component "Service"
        return $false
    }
}

function Get-BaconStatus {
    Write-BaconLog "INFO" "Getting Bacon system status..." -Component "Status"

    $status = @{
        Orchestration = @{
            Status = "Stopped"
            PID = $null
            Uptime = $null
            Cycles = 0
            LastActivity = $null
        }
        Sessions = @{
            Total = 0
            Hotspots = 0
            Resolved = 0
            Failed = 0
            InProgress = 0
            Rollbacks = 0
            LastUpdated = $null
        }
        Metrics = $null
        Health = @{
            Overall = "Unknown"
            Score = 0
            Issues = @()
            Warnings = @()
            Recommendations = @()
        }
        System = @{
            MemoryUsage = $null
            DiskUsage = $null
            ProcessCount = 0
        }
        Timestamp = Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ"
    }

    # Check orchestration status
    $pidFile = Join-Path $SessionsDir "orchestrate.pid"
    if (Test-Path $pidFile) {
        try {
            $orchestratePid = Get-Content $pidFile -ErrorAction Stop
            $process = Get-Process -Id $orchestratePid -ErrorAction SilentlyContinue
            if ($process) {
                $status.Orchestration.Status = "Running"
                $status.Orchestration.PID = $orchestratePid
                $status.Orchestration.Uptime = [math]::Round((Get-Date) - $process.StartTime).TotalMinutes
                $status.Orchestration.LastActivity = $process.StartTime
            } else {
                $status.Orchestration.Status = "Stopped (stale PID file)"
                $status.Health.Issues += "Stale PID file detected"
            }
        } catch {
            $status.Orchestration.Status = "Error reading PID"
            $status.Health.Issues += "Failed to read orchestration PID"
        }
    }

    # Session analysis
    if (Test-Path $SessionsDir) {
        $sessionFiles = Get-ChildItem -Path $SessionsDir -Filter "*.json" -ErrorAction SilentlyContinue
        $status.Sessions.Total = $sessionFiles.Count
        $status.Sessions.Hotspots = ($sessionFiles | Where-Object { $_.Name -like "*hotspot*" }).Count
        $status.Sessions.Resolved = ($sessionFiles | Where-Object { $_.Name -like "*resolved*" }).Count
        $status.Sessions.Failed = ($sessionFiles | Where-Object { $_.Name -like "*failed*" }).Count
        $status.Sessions.InProgress = ($sessionFiles | Where-Object { $_.Name -like "*cycle_*" }).Count
        $status.Sessions.Rollbacks = ($sessionFiles | Where-Object { $_.Name -like "*rollback*" }).Count

        if ($sessionFiles.Count -gt 0) {
            $status.Sessions.LastUpdated = ($sessionFiles | Sort-Object LastWriteTime -Descending | Select-Object -First 1).LastWriteTime
        }

        if ($status.Sessions.Hotspots -gt 5) {
            $status.Health.Warnings += "High number of unresolved hotspots ($($status.Sessions.Hotspots))"
        }
        if ($status.Sessions.Failed -gt 2) {
            $status.Health.Issues += "Multiple failed sessions detected ($($status.Sessions.Failed))"
        }
    }

    # Metrics analysis
    $metricsFile = Join-Path $SessionsDir "metrics.json"
    if (Test-Path $metricsFile) {
        try {
            $status.Metrics = Get-Content $metricsFile -Raw | ConvertFrom-Json -ErrorAction Stop

            $recentMetrics = $status.Metrics | Where-Object {
                [DateTime]$_.timestamp -gt (Get-Date).AddHours(-1)
            } | Sort-Object timestamp -Descending

            if ($recentMetrics.Count -gt 0) {
                $successRate = ($recentMetrics | Where-Object { $_.status -eq "success" }).Count / $recentMetrics.Count * 100
                if ($successRate -lt 80) {
                    $status.Health.Issues += "Low success rate in last hour: $([math]::Round($successRate, 1))%"
                }
            }
        } catch {
            Write-BaconLog "WARN" "Failed to parse metrics file" -Component "Status"
            $status.Health.Warnings += "Metrics file corrupted or unreadable"
        }
    } else {
        $status.Health.Warnings += "No metrics file available"
    }

    # System resource monitoring
    try {
        $status.System.MemoryUsage = Get-Process | Where-Object { $_.ProcessName -like "*bacon*" } | Measure-Object WorkingSet -Sum | Select-Object @{Name="TotalMB";Expression={[math]::Round($_.Sum / 1MB, 2)}}
        $status.System.ProcessCount = (Get-Process | Where-Object { $_.ProcessName -like "*bacon*" }).Count

        $drive = Get-Volume -DriveLetter (Get-Location).Drive.Name
        $status.System.DiskUsage = @{
            FreeGB = [math]::Round($drive.SizeRemaining / 1GB, 2)
            TotalGB = [math]::Round($drive.Size / 1GB, 2)
            UsagePercent = [math]::Round((1 - $drive.SizeRemaining / $drive.Size) * 100, 1)
        }

        if ($status.System.DiskUsage.UsagePercent -gt 90) {
            $status.Health.Issues += "High disk usage: $($status.System.DiskUsage.UsagePercent)%"
        }
    } catch {
        $status.Health.Warnings += "Failed to gather system metrics"
    }

    # Health score calculation
    $score = 100
    $score -= ($status.Health.Issues.Count * 20)
    $score -= ($status.Health.Warnings.Count * 5)
    $score = [math]::Max(0, $score)
    $status.Health.Score = $score

    if ($score -ge 90) { $status.Health.Overall = "Excellent" }
    elseif ($score -ge 75) { $status.Health.Overall = "Good" }
    elseif ($score -ge 60) { $status.Health.Overall = "Fair" }
    elseif ($score -ge 40) { $status.Health.Overall = "Poor" }
    else { $status.Health.Overall = "Critical" }

    # Recommendations
    if ($status.Orchestration.Status -ne "Running") {
        $status.Health.Recommendations += "Consider starting Bacon orchestration"
    }
    if ($status.Sessions.Hotspots -gt 3) {
        $status.Health.Recommendations += "Review and resolve outstanding hotspots"
    }
    if ($status.System.DiskUsage.UsagePercent -gt 80) {
        $status.Health.Recommendations += "Run cleanup to free disk space"
    }

    return $status
}

function Show-BaconLogs {
    param([int]$TailCount = 50)

    $logFiles = @(
        (Join-Path $SessionsDir "bacon_orchestrate.log"),
        (Join-Path $SessionsDir "bacon_apply.log"),
        (Join-Path $SessionsDir "observer.log"),
        (Join-Path $SessionsDir "strategist.log"),
        (Join-Path $SessionsDir "coder.log"),
        (Join-Path $SessionsDir "auditor.log"),
        $LogFile
    )

    foreach ($logFile in $logFiles) {
        if (Test-Path $logFile) {
            Write-Host "`n=== $(Split-Path $logFile -Leaf) ===" -ForegroundColor Cyan
            Get-Content $logFile -Tail $TailCount -ErrorAction SilentlyContinue
        }
    }
}

# --- Main ---

function Main {
    switch ($Action) {
        "start" { Start-BaconOrchestration }
        "stop"  { Stop-BaconOrchestration }
        "status" {
            $status = Get-BaconStatus
            Write-Host "`n=== Bacon System Status ===" -ForegroundColor Cyan

            $healthColor = switch ($status.Health.Overall) {
                "Excellent" { "Green" }; "Good" { "Green" }; "Fair" { "Yellow" }
                "Poor" { "Red" }; "Critical" { "Magenta" }; default { "White" }
            }
            Write-Host "Health: $($status.Health.Overall) (Score: $($status.Health.Score)/100)" -ForegroundColor $healthColor

            $orchColor = if ($status.Orchestration.Status -eq "Running") { "Green" } else { "Yellow" }
            Write-Host "Orchestration: $($status.Orchestration.Status)" -ForegroundColor $orchColor
            if ($status.Orchestration.PID) {
                Write-Host "  PID: $($status.Orchestration.PID)"
                if ($status.Orchestration.Uptime) { Write-Host "  Uptime: $($status.Orchestration.Uptime) minutes" }
            }

            Write-Host "Sessions: $($status.Sessions.Total) total"
            Write-Host "  Hotspots: $($status.Sessions.Hotspots)"
            Write-Host "  Resolved: $($status.Sessions.Resolved)"
            Write-Host "  Failed: $($status.Sessions.Failed)"
            Write-Host "  In Progress: $($status.Sessions.InProgress)"
            if ($status.Sessions.LastUpdated) { Write-Host "  Last Updated: $($status.Sessions.LastUpdated)" }

            if ($status.System.MemoryUsage) {
                Write-Host "System Resources:"
                Write-Host "  Memory Usage: $($status.System.MemoryUsage.TotalMB) MB"
                Write-Host "  Process Count: $($status.System.ProcessCount)"
            }
            if ($status.System.DiskUsage) {
                Write-Host "  Disk Usage: $($status.System.DiskUsage.FreeGB) GB free of $($status.System.DiskUsage.TotalGB) GB ($($status.System.DiskUsage.UsagePercent)% used)"
            }

            if ($status.Health.Issues.Count -gt 0) {
                Write-Host "`nIssues:" -ForegroundColor Red
                foreach ($issue in $status.Health.Issues) { Write-Host "  - $issue" -ForegroundColor Red }
            }
            if ($status.Health.Warnings.Count -gt 0) {
                Write-Host "`nWarnings:" -ForegroundColor Yellow
                foreach ($warning in $status.Health.Warnings) { Write-Host "  - $warning" -ForegroundColor Yellow }
            }
            if ($status.Health.Recommendations.Count -gt 0) {
                Write-Host "`nRecommendations:" -ForegroundColor Cyan
                foreach ($rec in $status.Health.Recommendations) { Write-Host "  - $rec" -ForegroundColor Cyan }
            }
            if ($status.Metrics) { Write-Host "`nMetrics: Available" }
            else { Write-Host "`nMetrics: Not available" -ForegroundColor Yellow }
        }
        "logs" { Show-BaconLogs -TailCount $Tail }
    }
}

Main
