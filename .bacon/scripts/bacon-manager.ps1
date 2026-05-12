# .bacon/scripts/bacon-manager.ps1 - PowerShell Management Dashboard
# Comprehensive management interface for the Bacon autonomous coding system

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("start", "stop", "status", "logs", "metrics", "cleanup", "test", "apply-approved")]
    [string]$Action = "status",

    [Parameter(Mandatory=$false)]
    [string]$Config = ".bacon/bacon.toml",

    [Parameter(Mandatory=$false)]
    [string]$Patch = "",

    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$RunCheck,

    [Parameter(Mandatory=$false)]
    [switch]$Force,

    [Parameter(Mandatory=$false)]
    [switch]$ScriptVerbose
)

# Global variables
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$BaconDir = Join-Path $ProjectRoot ".bacon"
$SessionsDir = Join-Path $BaconDir "sessions"
$ScriptsDir = Join-Path $BaconDir "scripts"
$LogFile = Join-Path $SessionsDir "bacon_manager.log"

function Get-BaconBashPath {
    $candidates = @(
        $env:BACON_BASH,
        (Join-Path $env:ProgramFiles "Git\bin\bash.exe"),
        (Join-Path ${env:ProgramFiles(x86)} "Git\bin\bash.exe"),
        "C:\Program Files\Git\bin\bash.exe"
    ) | Where-Object { $_ }

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    $cmd = Get-Command "bash" -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    return $null
}

# Enhanced logging function with structured output
function Write-BaconLog {
    param(
        [Parameter(Mandatory=$true)]
        [ValidateSet("DEBUG", "INFO", "WARN", "ERROR", "CRITICAL")]
        [string]$Level,

        [Parameter(Mandatory=$true)]
        [string]$Message,

        [Parameter(Mandatory=$false)]
        [string]$Component = "Manager",

        [Parameter(Mandatory=$false)]
        [hashtable]$Context = @{}
    )

    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $contextStr = if ($Context.Count -gt 0) { " [$($Context.Keys -join ':')]" } else { "" }
    $logEntry = "[$timestamp] [$Level] [$Component] $Message$contextStr"

    # Console output with colors
    $color = switch ($Level) {
        "DEBUG" { "Cyan" }
        "INFO" { "Green" }
        "WARN" { "Yellow" }
        "ERROR" { "Red" }
        "CRITICAL" { "Magenta" }
        default { "White" }
    }

    if ($ScriptVerbose -or $Level -in @("WARN", "ERROR", "CRITICAL")) {
        Write-Host $logEntry -ForegroundColor $color
    }

    # File logging with rotation
    try {
        if ((Test-Path $LogFile) -and (Get-Item $LogFile).Length -gt 50MB) {
            $backupFile = $LogFile.Replace(".log", "_$(Get-Date -Format 'yyyyMMddHHmmss').log")
            Move-Item $LogFile $backupFile -Force
        }
        Add-Content -Path $LogFile -Value $logEntry -ErrorAction SilentlyContinue
    } catch {
        Write-Host "Failed to write to log file: $($_.Exception.Message)" -ForegroundColor Red
    }
}

# Enhanced prerequisites check with detailed reporting
function Test-BaconPrerequisites {
    Write-BaconLog "INFO" "Checking Bacon system prerequisites..." -Component "Prerequisites"

    $issues = @()
    $warnings = @()
    $details = @{}

    # Check required directories
    $requiredDirs = @($BaconDir, $SessionsDir, $ScriptsDir)
    $details.Directories = @()
    foreach ($dir in $requiredDirs) {
        $dirExists = Test-Path $dir
        $details.Directories += @{
            Path = $dir
            Exists = $dirExists
            Writable = if ($dirExists) {
                try {
                    $testFile = Join-Path $dir "test_write_$(Get-Random)"
                    "test" | Out-File -FilePath $testFile -Encoding UTF8 -ErrorAction Stop
                    Remove-Item $testFile -Force -ErrorAction SilentlyContinue
                    $true
                } catch { $false }
            } else { $false }
        }
        if (-not $dirExists) {
            $issues += "Required directory missing: $dir"
        }
    }

    # Check required files
    $requiredFiles = @(
        (Join-Path $BaconDir "bacon.toml"),
        (Join-Path $ScriptsDir "bacon-orchestrate.sh"),
        (Join-Path $ScriptsDir "bacon-apply-shadow.sh"),
        (Join-Path $ScriptsDir "bacon-sentinel.sh"),
        (Join-Path $ScriptsDir "bacon-observer.sh"),
        (Join-Path $ScriptsDir "bacon-strategist.sh"),
        (Join-Path $ScriptsDir "bacon-coder.sh"),
        (Join-Path $ScriptsDir "bacon-auditor.sh"),
        (Join-Path $ScriptsDir "bacon-apply-approved.sh")
    )
    $details.Files = @()
    foreach ($file in $requiredFiles) {
        $fileExists = Test-Path $file
        $details.Files += @{
            Path = $file
            Exists = $fileExists
            Executable = if ($fileExists -and $file.EndsWith(".sh")) {
                try {
                    $null = bash -c "test -x '$file'" 2>&1
                    $LASTEXITCODE -eq 0
                } catch { $false }
            } else { $null }
        }
        if (-not $fileExists) {
            $issues += "Required file missing: $file"
        }
    }

    # Check required commands
    $requiredCommands = @("git", "cargo")
    $details.Commands = @()
    foreach ($cmd in $requiredCommands) {
        try {
            $cmdInfo = Get-Command $cmd -ErrorAction Stop
            $details.Commands += @{
                Name = $cmd
                Found = $true
                Path = $cmdInfo.Source
                Version = if ($cmd -eq "git") {
                    try { git --version 2>&1 | Select-Object -First 1 } catch { "Unknown" }
                } elseif ($cmd -eq "cargo") {
                    try { cargo --version 2>&1 | Select-Object -First 1 } catch { "Unknown" }
                } else { "Unknown" }
            }
        } catch {
            $details.Commands += @{
                Name = $cmd
                Found = $false
                Path = $null
                Version = $null
            }
            $issues += "Required command not found: $cmd"
        }
    }

    $bashPath = Get-BaconBashPath
    if ($bashPath) {
        Write-BaconLog "INFO" "Using bash: $bashPath" -Component "Prerequisites"
    } else {
        $issues += "Required command not found: Git Bash"
    }

    # Check optional but recommended commands
    $optionalCommands = @("bacon", "jq")
    foreach ($cmd in $optionalCommands) {
        try {
            $null = Get-Command $cmd -ErrorAction Stop
            Write-BaconLog "INFO" "Optional command found: $cmd" -Component "Prerequisites"
        } catch {
            $warnings += "Optional command not found: $cmd"
        }
    }

    # Check system resources
    $details.System = @{
        Memory = (Get-CimInstance -ClassName Win32_OperatingSystem | Select-Object TotalVisibleMemorySize, FreePhysicalMemory | ForEach-Object {
            "$([math]::Round($_.FreePhysicalMemory / 1MB, 2))GB free of $([math]::Round($_.TotalVisibleMemorySize / 1MB, 2))GB total"
        })
        Disk = Get-Volume -DriveLetter (Get-Location).Drive.Name | Select-Object @{Name="FreeSpace";Expression={"$([math]::Round($_.SizeRemaining / 1GB, 2))GB"}}, @{Name="TotalSpace";Expression={"$([math]::Round($_.Size / 1GB, 2))GB"}}
    }

    # Log detailed results
    if ($ScriptVerbose) {
        Write-BaconLog "DEBUG" "Prerequisites check completed" -Component "Prerequisites" -Context $details
    }

    if ($issues.Count -gt 0) {
        Write-BaconLog "ERROR" "Prerequisites check failed:" -Component "Prerequisites"
        foreach ($issue in $issues) {
            Write-BaconLog "ERROR" "  - $issue" -Component "Prerequisites"
        }
    }

    if ($warnings.Count -gt 0) {
        Write-BaconLog "WARN" "Optional components missing:" -Component "Prerequisites"
        foreach ($warning in $warnings) {
            Write-BaconLog "WARN" "  - $warning" -Component "Prerequisites"
        }
    }

    return $issues.Count -eq 0
}

# Start Bacon orchestration
function Start-BaconOrchestration {
    Write-BaconLog "INFO" "Starting Bacon orchestration..."

    # Check if already running
    $pidFile = Join-Path $SessionsDir "orchestrate.pid"
    if (Test-Path $pidFile) {
        try {
            $existingPid = Get-Content $pidFile -ErrorAction Stop
            $process = Get-Process -Id $existingPid -ErrorAction SilentlyContinue
            if ($process) {
                Write-BaconLog "WARN" "Bacon orchestration already running (PID: $existingPid)"
                return $false
            }
        } catch {
            # PID file exists but process not running, clean up
            Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
        }
    }

    # Start orchestration in background
    $orchestrateScript = Join-Path $ScriptsDir "bacon-orchestrate.sh"

    try {
        $bashPath = Get-BaconBashPath
        if ($bashPath) {
            $process = Start-Process -FilePath $bashPath -ArgumentList $orchestrateScript -PassThru -WindowStyle Hidden
        } else {
            Write-BaconLog "ERROR" "Git Bash not found"
            return $false
        }

        # Store PID
        $process.Id | Out-File -FilePath $pidFile -Encoding UTF8

        Write-BaconLog "INFO" "Bacon orchestration started (PID: $($process.Id))"
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to start Bacon orchestration: $($_.Exception.Message)"
        return $false
    }
}

# Stop Bacon orchestration
function Stop-BaconOrchestration {
    Write-BaconLog "INFO" "Stopping Bacon orchestration..."

    $pidFile = Join-Path $SessionsDir "orchestrate.pid"

    if (-not (Test-Path $pidFile)) {
        Write-BaconLog "WARN" "No Bacon orchestration PID file found"
        return $false
    }

    try {
        $processId = Get-Content $pidFile -ErrorAction Stop
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue

        if ($process) {
            $process.Kill()
            Write-BaconLog "INFO" "Bacon orchestration stopped (PID: $processId)"
        } else {
            Write-BaconLog "WARN" "Process not found for PID: $processId"
        }

        Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to stop Bacon orchestration: $($_.Exception.Message)"
        return $false
    }
}

# Apply an approved Bacon patch candidate
function Invoke-BaconApplyApproved {
    Write-BaconLog "INFO" "Applying approved Bacon patch..."

    $applyScript = Join-Path $ScriptsDir "bacon-apply-approved.sh"
    if (-not (Test-Path $applyScript)) {
        Write-BaconLog "ERROR" "Apply script not found: $applyScript"
        return $false
    }

    $bashPath = Get-BaconBashPath
    if (-not $bashPath) {
        Write-BaconLog "ERROR" "Git Bash is required to apply approved patches"
        return $false
    }

    $args = @($applyScript)
    if ($Patch) { $args += $Patch }
    if ($DryRun) { $args += "--dry-run" }
    if ($RunCheck) { $args += "--run-check" }
    if ($Force) { $args += "--force" }

    & $bashPath @args
    return $LASTEXITCODE -eq 0
}

# Enhanced status with detailed health metrics
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

    # Check orchestration status with enhanced details
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

    # Enhanced session analysis
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

        # Health warnings for sessions
        if ($status.Sessions.Hotspots -gt 5) {
            $status.Health.Warnings += "High number of unresolved hotspots ($($status.Sessions.Hotspots))"
        }
        if ($status.Sessions.Failed -gt 2) {
            $status.Health.Issues += "Multiple failed sessions detected ($($status.Sessions.Failed))"
        }
    }

    # Enhanced metrics analysis
    $metricsFile = Join-Path $SessionsDir "metrics.json"
    if (Test-Path $metricsFile) {
        try {
            $status.Metrics = Get-Content $metricsFile -Raw | ConvertFrom-Json -ErrorAction Stop

            # Analyze recent activity
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

    # Calculate overall health score
    $score = 100
    $issues = $status.Health.Issues.Count
    $warnings = $status.Health.Warnings.Count

    $score -= ($issues * 20)  # Each issue reduces score by 20 points
    $score -= ($warnings * 5) # Each warning reduces score by 5 points
    $score = [math]::Max(0, $score)

    $status.Health.Score = $score

    # Determine overall health status
    if ($score -ge 90) {
        $status.Health.Overall = "Excellent"
    } elseif ($score -ge 75) {
        $status.Health.Overall = "Good"
    } elseif ($score -ge 60) {
        $status.Health.Overall = "Fair"
    } elseif ($score -ge 40) {
        $status.Health.Overall = "Poor"
    } else {
        $status.Health.Overall = "Critical"
    }

    # Generate recommendations
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

# Show logs
function Show-BaconLogs {
    param(
        [Parameter(Mandatory=$false)]
        [int]$Tail = 50
    )

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
            Get-Content $logFile -Tail $Tail -ErrorAction SilentlyContinue
        }
    }
}

# Show metrics
function Show-BaconMetrics {
    $metricsFile = Join-Path $SessionsDir "metrics.json"

    if (-not (Test-Path $metricsFile)) {
        Write-Host "No metrics file found" -ForegroundColor Yellow
        return
    }

    try {
        $metrics = Get-Content $metricsFile -Raw | ConvertFrom-Json -ErrorAction Stop

        Write-Host "`n=== Bacon Metrics ===" -ForegroundColor Green

        # Event summary
        $eventCounts = $metrics | Group-Object event | ForEach-Object {
            @{
                Event = $_.Name
                Count = $_.Count
                Success = ($_.Group | Where-Object { $_.status -eq "success" }).Count
                Failed = ($_.Group | Where-Object { $_.status -eq "failed" }).Count
            }
        }

        foreach ($count in $eventCounts) {
            Write-Host "$($count.Event): $($count.Count) total ($($count.Success) success, $($count.Failed) failed)"
        }

        # Recent activity
        Write-Host "`nRecent Activity:" -ForegroundColor Cyan
        $metrics | Sort-Object timestamp -Descending | Select-Object -First 10 | ForEach-Object {
            $statusColor = if ($_.status -eq "success") { "Green" } elseif ($_.status -eq "failed") { "Red" } else { "Yellow" }
            Write-Host "[$($_.timestamp)] $($_.event) - $($_.status)" -ForegroundColor $statusColor
        }

    } catch {
        Write-BaconLog "ERROR" "Failed to parse metrics: $($_.Exception.Message)"
    }
}

# Cleanup old files
function Invoke-BaconCleanup {
    Write-BaconLog "INFO" "Starting Bacon cleanup..."

    $cleanupCount = 0

    # Clean old shadow workspaces
    $tempDirs = Get-ChildItem -Path "/tmp" -Filter "norino_shadow_*" -ErrorAction SilentlyContinue
    foreach ($dir in $tempDirs) {
        if ($dir.CreationTime -lt (Get-Date).AddDays(-1)) {
            try {
                Remove-Item $dir.FullName -Recurse -Force -ErrorAction Stop
                $cleanupCount++
                Write-BaconLog "INFO" "Removed old shadow workspace: $($dir.Name)"
            } catch {
                Write-BaconLog "WARN" "Failed to remove $($dir.Name): $($_.Exception.Message)"
            }
        }
    }

    # Clean old session files
    $sessionFiles = Get-ChildItem -Path $SessionsDir -Filter "*.json" -ErrorAction SilentlyContinue
    foreach ($file in $sessionFiles) {
        if ($file.CreationTime -lt (Get-Date).AddDays(-7)) {
            try {
                Remove-Item $file.FullName -Force -ErrorAction Stop
                $cleanupCount++
                Write-BaconLog "INFO" "Removed old session file: $($file.Name)"
            } catch {
                Write-BaconLog "WARN" "Failed to remove $($file.Name): $($_.Exception.Message)"
            }
        }
    }

    # Clean old logs
    $logFiles = Get-ChildItem -Path $SessionsDir -Filter "*.log" -ErrorAction SilentlyContinue
    foreach ($file in $logFiles) {
        if ($file.Length -gt 10MB -or $file.CreationTime -lt (Get-Date).AddDays(-3)) {
            try {
                Remove-Item $file.FullName -Force -ErrorAction Stop
                $cleanupCount++
                Write-BaconLog "INFO" "Removed old log file: $($file.Name)"
            } catch {
                Write-BaconLog "WARN" "Failed to remove $($file.Name): $($_.Exception.Message)"
            }
        }
    }

    Write-BaconLog "INFO" "Cleanup completed. Removed $cleanupCount files."
}

# Test Bacon system
function Test-BaconSystem {
    Write-BaconLog "INFO" "Testing Bacon system..."

    $testsPassed = 0
    $testsTotal = 0

    # Test prerequisites
    $testsTotal++
    if (Test-BaconPrerequisites) {
        $testsPassed++
        Write-Host "✓ Prerequisites test passed" -ForegroundColor Green
    } else {
        Write-Host "✗ Prerequisites test failed" -ForegroundColor Red
    }

    # Test configuration parsing
    $testsTotal++
    try {
        Get-Content (Join-Path $BaconDir "bacon.toml") -Raw -ErrorAction Stop | Out-Null
        Write-Host "✓ Configuration file readable" -ForegroundColor Green
        $testsPassed++
    } catch {
        Write-Host "✗ Configuration file error: $($_.Exception.Message)" -ForegroundColor Red
    }

    # Test script availability
    $testsTotal++
    $requiredScripts = @("bacon-orchestrate.sh", "bacon-apply-shadow.sh", "bacon-apply-approved.sh", "bacon-sentinel.sh")
    $scriptsFound = 0
    foreach ($script in $requiredScripts) {
        if (Test-Path (Join-Path $ScriptsDir $script)) {
            $scriptsFound++
        }
    }
    if ($scriptsFound -eq $requiredScripts.Count) {
        Write-Host "✓ All required scripts found" -ForegroundColor Green
        $testsPassed++
    } else {
        Write-Host "✗ Missing scripts: $scriptsFound/$($requiredScripts.Count) found" -ForegroundColor Red
    }

    # Test directory permissions
    $testsTotal++
    try {
        $testFile = Join-Path $SessionsDir "test_write_$(Get-Date -Format 'yyyyMMddHHmmss')"
        "test" | Out-File -FilePath $testFile -Encoding UTF8 -ErrorAction Stop
        Remove-Item $testFile -Force -ErrorAction Stop
        Write-Host "✓ Directory permissions OK" -ForegroundColor Green
        $testsPassed++
    } catch {
        Write-Host "✗ Directory permission error: $($_.Exception.Message)" -ForegroundColor Red
    }

    Write-Host "`nTest Results: $testsPassed/$testsTotal tests passed" -ForegroundColor $(if ($testsPassed -eq $testsTotal) { "Green" } else { "Yellow" })

    return $testsPassed -eq $testsTotal
}

# Main execution
function Main {
    # Ensure directories exist
    if (-not (Test-Path $SessionsDir)) {
        New-Item -ItemType Directory -Path $SessionsDir -Force | Out-Null
    }

    Write-BaconLog "INFO" "Bacon Manager action: $Action"

    switch ($Action) {
        "start" {
            if (Test-BaconPrerequisites) {
                Start-BaconOrchestration
            } else {
                Write-BaconLog "ERROR" "Prerequisites check failed, cannot start"
            }
        }

        "stop" {
            Stop-BaconOrchestration
        }

        "status" {
            $status = Get-BaconStatus
            Write-Host "`n=== Bacon System Status ===" -ForegroundColor Cyan

            # Health status with color
            $healthColor = switch ($status.Health.Overall) {
                "Excellent" { "Green" }
                "Good" { "Green" }
                "Fair" { "Yellow" }
                "Poor" { "Red" }
                "Critical" { "Magenta" }
                default { "White" }
            }
            Write-Host "Health: $($status.Health.Overall) (Score: $($status.Health.Score)/100)" -ForegroundColor $healthColor

            # Orchestration details
            $orchColor = if ($status.Orchestration.Status -eq "Running") { "Green" } else { "Yellow" }
            Write-Host "Orchestration: $($status.Orchestration.Status)" -ForegroundColor $orchColor
            if ($status.Orchestration.PID) {
                Write-Host "  PID: $($status.Orchestration.PID)"
                if ($status.Orchestration.Uptime) {
                    Write-Host "  Uptime: $($status.Orchestration.Uptime) minutes"
                }
            }

            # Session details
            Write-Host "Sessions: $($status.Sessions.Total) total"
            Write-Host "  Hotspots: $($status.Sessions.Hotspots)"
            Write-Host "  Resolved: $($status.Sessions.Resolved)"
            Write-Host "  Failed: $($status.Sessions.Failed)"
            Write-Host "  In Progress: $($status.Sessions.InProgress)"
            if ($status.Sessions.LastUpdated) {
                Write-Host "  Last Updated: $($status.Sessions.LastUpdated)"
            }

            # System resources
            if ($status.System.MemoryUsage) {
                Write-Host "System Resources:"
                Write-Host "  Memory Usage: $($status.System.MemoryUsage.TotalMB) MB"
                Write-Host "  Process Count: $($status.System.ProcessCount)"
            }
            if ($status.System.DiskUsage) {
                $diskUsagePercent = $status.System.DiskUsage.UsagePercent
                $diskUsageText = "$diskUsagePercent percent used"
                Write-Host "  Disk Usage: $($status.System.DiskUsage.FreeGB) GB free of $($status.System.DiskUsage.TotalGB) GB ($diskUsageText)"
            }

            # Issues and warnings
            if ($status.Health.Issues.Count -gt 0) {
                Write-Host "`nIssues:" -ForegroundColor Red
                foreach ($issue in $status.Health.Issues) {
                    Write-Host "  - $issue" -ForegroundColor Red
                }
            }

            if ($status.Health.Warnings.Count -gt 0) {
                Write-Host "`nWarnings:" -ForegroundColor Yellow
                foreach ($warning in $status.Health.Warnings) {
                    Write-Host "  - $warning" -ForegroundColor Yellow
                }
            }

            # Recommendations
            if ($status.Health.Recommendations.Count -gt 0) {
                Write-Host "`nRecommendations:" -ForegroundColor Cyan
                foreach ($rec in $status.Health.Recommendations) {
                    Write-Host "  - $rec" -ForegroundColor Cyan
                }
            }

            # Metrics availability
            if ($status.Metrics) {
                Write-Host "`nMetrics: Available"
            } else {
                Write-Host "`nMetrics: Not available" -ForegroundColor Yellow
            }
        }

        "logs" {
            Show-BaconLogs
        }

        "metrics" {
            Show-BaconMetrics
        }

        "cleanup" {
            Invoke-BaconCleanup
        }

        "test" {
            Test-BaconSystem
        }

        "apply-approved" {
            Invoke-BaconApplyApproved
        }

        default {
            Write-Host "Unknown action: $Action" -ForegroundColor Red
            Write-Host "Available actions: start, stop, status, logs, metrics, cleanup, test, apply-approved" -ForegroundColor Yellow
        }
    }
}

# Run main function
Main
