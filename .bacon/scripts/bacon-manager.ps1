# .bacon/scripts/bacon-manager.ps1 - PowerShell Management Dashboard
# Comprehensive management interface for the Bacon autonomous coding system

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("start", "stop", "status", "logs", "metrics", "cleanup", "test", "apply-approved", "rotate-keys", "report", "dashboard")]
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
        (Join-Path $BaconDir "bacon.toml")
    )
    $details.Files = @()
    foreach ($file in $requiredFiles) {
        $fileExists = Test-Path $file
        $details.Files += @{
            Path = $file
            Exists = $fileExists
            Executable = $null
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
    try {
        $process = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "bacon" -WorkingDirectory $ProjectRoot -PassThru -WindowStyle Hidden

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
    Write-BaconLog "INFO" "Applying approved Bacon patches via Rust supervisor..."
    
    $args = @("run", "--bin", "bacon", "--", "--auto-apply")
    if ($DryRun) { $args += "--dry-run" }
    
    try {
        Start-Process -FilePath "cargo" -ArgumentList $args -WorkingDirectory $ProjectRoot -Wait -NoNewWindow
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to apply patches: $($_.Exception.Message)"
        return $false
    }
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
    $tempBase = if ($env:TEMP) { $env:TEMP } else { "/tmp" }
    $tempDirs = Get-ChildItem -Path $tempBase -Filter "norino_shadow_*" -ErrorAction SilentlyContinue
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
        Write-Host "[PASS] Prerequisites test passed" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Prerequisites test failed" -ForegroundColor Red
    }

    # Test configuration parsing
    $testsTotal++
    try {
        Get-Content (Join-Path $BaconDir "bacon.toml") -Raw -ErrorAction Stop | Out-Null
        Write-Host "[PASS] Configuration file readable" -ForegroundColor Green
        $testsPassed++
    } catch {
        Write-Host "[FAIL] Configuration file error: $($_.Exception.Message)" -ForegroundColor Red
    }

    # Test script availability
    $testsTotal++
    $requiredScripts = @("bacon-manager.ps1")
    $scriptsFound = 0
    foreach ($script in $requiredScripts) {
        if (Test-Path (Join-Path $ScriptsDir $script)) {
            $scriptsFound++
        }
    }
    if ($scriptsFound -eq $requiredScripts.Count) {
        Write-Host "[PASS] All required scripts found" -ForegroundColor Green
        $testsPassed++
    } else {
        Write-Host "[FAIL] Missing scripts: $scriptsFound/$($requiredScripts.Count) found" -ForegroundColor Red
    }

    # Test directory permissions
    $testsTotal++
    try {
        $testFile = Join-Path $SessionsDir "test_write_$(Get-Date -Format 'yyyyMMddHHmmss')"
        "test" | Out-File -FilePath $testFile -Encoding UTF8 -ErrorAction Stop
        Remove-Item $testFile -Force -ErrorAction Stop
        Write-Host "[PASS] Directory permissions OK" -ForegroundColor Green
        $testsPassed++
    } catch {
        Write-Host "[FAIL] Directory permission error: $($_.Exception.Message)" -ForegroundColor Red
    }

    Write-Host "`nTest Results: $testsPassed/$testsTotal tests passed" -ForegroundColor $(if ($testsPassed -eq $testsTotal) { "Green" } else { "Yellow" })

    return $testsPassed -eq $testsTotal
}

# Rotate API keys for external LLM providers
function Invoke-BaconRotateKeys {
    Write-BaconLog "INFO" "Rotating API keys for external LLM providers..." -Component "Security"

    $envFile = Join-Path $ProjectRoot ".env"
    $envBackup = Join-Path $ProjectRoot ".env.backup.$(Get-Date -Format 'yyyyMMddHHmmss')"

    # Backup current .env file
    if (Test-Path $envFile) {
        Copy-Item $envFile $envBackup -Force
        Write-BaconLog "INFO" "Backed up .env to $envBackup" -Component "Security"
    }

    # Read current configuration
    $configFile = Join-Path $BaconDir "bacon.toml"
    if (-not (Test-Path $configFile)) {
        Write-BaconLog "ERROR" "Configuration file not found: $configFile" -Component "Security"
        return $false
    }

    try {
        $configContent = Get-Content $configFile -Raw
        $keysToRotate = @()

        # Find API key references in configuration
        if ($configContent -match '{env:([^}]+)}') {
            $matches | ForEach-Object {
                if ($_ -match '{env:([^}]+)}') {
                    $keyName = $matches[1]
                    if ($keyName -match '(?i)(api[_-]?key|token|secret|password)') {
                        $keysToRotate += $keyName
                    }
                }
            }
        }

        if ($keysToRotate.Count -eq 0) {
            Write-BaconLog "INFO" "No API keys found in configuration" -Component "Security"
            return $true
        }

        Write-Host "`n=== API Key Rotation ===" -ForegroundColor Cyan
        Write-Host "Found $($keysToRotate.Count) API keys to rotate:" -ForegroundColor Yellow
        
        $newEnvContent = @()
        if (Test-Path $envFile) {
            $newEnvContent = Get-Content $envFile
        }

        foreach ($key in $keysToRotate | Select-Object -Unique) {
            Write-Host "`nKey: $key" -ForegroundColor White
            
            # Check if key exists
            $currentValue = $null
            foreach ($line in $newEnvContent) {
                if ($line -match "^$key=(.+)$") {
                    $currentValue = $matches[1]
                    break
                }
            }

            if ($currentValue) {
                Write-Host "Current value: $($currentValue.Substring(0, [math]::Min(8, $currentValue.Length)))..." -ForegroundColor Gray
            } else {
                Write-Host "Not currently set" -ForegroundColor Yellow
            }

            $choice = Read-Host "`nAction: (r)otate, (s)kip, or (d)elete [r/s/d]"
            
            switch ($choice.ToLower()) {
                "r" {
                    $newValue = Read-Host "Enter new value for $key" -AsSecureString
                    $plainText = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
                        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($newValue)
                    )
                    
                    # Update or add the key
                    $updated = $false
                    for ($i = 0; $i -lt $newEnvContent.Count; $i++) {
                        if ($newEnvContent[$i] -match "^$key=") {
                            $newEnvContent[$i] = "$key=$plainText"
                            $updated = $true
                            break
                        }
                    }
                    
                    if (-not $updated) {
                        $newEnvContent += "$key=$plainText"
                    }
                    
                    Write-BaconLog "INFO" "Rotated key: $key" -Component "Security"
                    Write-Host "Key rotated successfully" -ForegroundColor Green
                }
                "s" {
                    Write-Host "Skipping $key" -ForegroundColor Yellow
                }
                "d" {
                    # Remove the key
                    $newEnvContent = $newEnvContent | Where-Object { $_ -notmatch "^$key=" }
                    Write-BaconLog "INFO" "Deleted key: $key" -Component "Security"
                    Write-Host "Key deleted" -ForegroundColor Red
                }
                default {
                    Write-Host "Invalid choice, skipping" -ForegroundColor Red
                }
            }
        }

        # Write updated .env file
        $newEnvContent | Out-File $envFile -Encoding UTF8
        Write-BaconLog "INFO" "Updated .env file with rotated keys" -Component "Security"

        # Update file permissions
        try {
            icacls $envFile /inheritance:r /grant:r "$($env:USERNAME):(R,W)" 2>&1 | Out-Null
            Write-BaconLog "INFO" "Set restrictive permissions on .env file" -Component "Security"
        } catch {
            Write-BaconLog "WARN" "Failed to set permissions on .env: $($_.Exception.Message)" -Component "Security"
        }

        Write-Host "`nKey rotation completed. Backup saved to $envBackup" -ForegroundColor Green
        return $true

    } catch {
        Write-BaconLog "ERROR" "Failed to rotate keys: $($_.Exception.Message)" -Component "Security"
        return $false
    }
}

# Generate HTML report
function Invoke-BaconReport {
    Write-BaconLog "INFO" "Generating Bacon system report..." -Component "Reporting"

    $reportFile = Join-Path $SessionsDir "report_$(Get-Date -Format 'yyyyMMdd').html"
    $status = Get-BaconStatus
    $metricsFile = Join-Path $SessionsDir "metrics.json"

    try {
        $html = @"
<!DOCTYPE html>
<html>
<head>
    <title>Bacon System Report - $(Get-Date -Format 'yyyy-MM-dd')</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #2c3e50; color: white; padding: 20px; border-radius: 5px; }
        .section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }
        .health-excellent { background: #d4edda; border-color: #c3e6cb; }
        .health-good { background: #d1ecf1; border-color: #bee5eb; }
        .health-fair { background: #fff3cd; border-color: #ffeaa7; }
        .health-poor { background: #f8d7da; border-color: #f5c6cb; }
        .health-critical { background: #dc3545; color: white; }
        .metric { display: inline-block; margin: 10px; padding: 10px; background: #f8f9fa; border-radius: 3px; }
        .issue { color: #dc3545; }
        .warning { color: #ffc107; }
        .success { color: #28a745; }
        table { width: 100%; border-collapse: collapse; }
        th, td { padding: 8px; text-align: left; border-bottom: 1px solid #ddd; }
        th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Bacon System Report</h1>
        <p>Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')</p>
    </div>

    <div class="section health-$($status.Health.Overall.ToLower())">
        <h2>System Health: $($status.Health.Overall) ($($status.Health.Score)/100)</h2>
        <p>Timestamp: $($status.Timestamp)</p>
    </div>

    <div class="section">
        <h2>Orchestration Status</h2>
        <p><strong>Status:</strong> $($status.Orchestration.Status)</p>
"@

        if ($status.Orchestration.PID) {
            $html += @"
        <p><strong>PID:</strong> $($status.Orchestration.PID)</p>
        <p><strong>Uptime:</strong> $($status.Orchestration.Uptime) minutes</p>
"@
        }

        $html += @"
    </div>

    <div class="section">
        <h2>Sessions</h2>
        <div class="metric">Total: $($status.Sessions.Total)</div>
        <div class="metric">Hotspots: $($status.Sessions.Hotspots)</div>
        <div class="metric">Resolved: $($status.Sessions.Resolved)</div>
        <div class="metric">Failed: $($status.Sessions.Failed)</div>
        <div class="metric">In Progress: $($status.Sessions.InProgress)</div>
        <div class="metric">Rollbacks: $($status.Sessions.Rollbacks)</div>
"@

        if ($status.Sessions.LastUpdated) {
            $html += @"
        <p><strong>Last Updated:</strong> $($status.Sessions.LastUpdated)</p>
"@
        }

        $html += @"
    </div>

    <div class="section">
        <h2>System Resources</h2>
"@

        if ($status.System.MemoryUsage) {
            $html += @"
        <p><strong>Memory Usage:</strong> $($status.System.MemoryUsage.TotalMB) MB</p>
        <p><strong>Process Count:</strong> $($status.System.ProcessCount)</p>
"@
        }

        if ($status.System.DiskUsage) {
            $html += @"
        <p><strong>Disk Usage:</strong> $($status.System.DiskUsage.FreeGB) GB free of $($status.System.DiskUsage.TotalGB) GB ($($status.System.DiskUsage.UsagePercent)%)</p>
"@
        }

        $html += @"
    </div>
"@

        # Issues and warnings
        if ($status.Health.Issues.Count -gt 0) {
            $html += @"
    <div class="section">
        <h2 class="issue">Issues</h2>
        <ul>
"@
            foreach ($issue in $status.Health.Issues) {
                $html += @"
            <li class="issue">$issue</li>
"@
            }
            $html += @"
        </ul>
    </div>
"@
        }

        if ($status.Health.Warnings.Count -gt 0) {
            $html += @"
    <div class="section">
        <h2 class="warning">Warnings</h2>
        <ul>
"@
            foreach ($warning in $status.Health.Warnings) {
                $html += @"
            <li class="warning">$warning</li>
"@
            }
            $html += @"
        </ul>
    </div>
"@
        }

        # Recommendations
        if ($status.Health.Recommendations.Count -gt 0) {
            $html += @"
    <div class="section">
        <h2 class="success">Recommendations</h2>
        <ul>
"@
            foreach ($rec in $status.Health.Recommendations) {
                $html += @"
            <li class="success">$rec</li>
"@
            }
            $html += @"
        </ul>
    </div>
"@
        }

        # Recent metrics if available
        if (Test-Path $metricsFile) {
            try {
                $metrics = Get-Content $metricsFile -Raw | ConvertFrom-Json -ErrorAction Stop
                $recentMetrics = $metrics | Where-Object {
                    [DateTime]$_.timestamp -gt (Get-Date).AddHours(-24)
                } | Sort-Object timestamp -Descending | Select-Object -First 20
                
                if ($recentMetrics.Count -gt 0) {
                    $html += @"
    <div class="section">
        <h2>Recent Activity (Last 24 Hours)</h2>
        <table>
            <tr>
                <th>Timestamp</th>
                <th>Event</th>
                <th>Status</th>
                <th>Duration (ms)</th>
            </tr>
"@
                    foreach ($metric in $recentMetrics) {
                        $statusColor = if ($metric.status -eq "success") { "success" } elseif ($metric.status -eq "failed") { "issue" } else { "warning" }
                        $html += @"
            <tr>
                <td>$($metric.timestamp)</td>
                <td>$($metric.event)</td>
                <td class="$statusColor">$($metric.status)</td>
                <td>$($metric.duration_ms)</td>
            </tr>
"@
                    }
                    $html += @"
        </table>
    </div>
"@
                }
            } catch {
                $html += @"
    <div class="section">
        <h2 class="warning">Metrics</h2>
        <p class="warning">Failed to load metrics data</p>
    </div>
"@
            }
        }

        $html += @"
    <div class="section">
        <h2>Report Information</h2>
        <p>This report was generated by bacon-manager.ps1</p>
        <p>Configuration: $Config</p>
        <p>Report file: $reportFile</p>
    </div>
</body>
</html>
"@

        $html | Out-File $reportFile -Encoding UTF8
        Write-BaconLog "INFO" "Report generated: $reportFile" -Component "Reporting"
        Write-Host "Report generated: $reportFile" -ForegroundColor Green
        
        # Try to open the report in default browser
        try {
            Start-Process $reportFile
            Write-Host "Opening report in browser..." -ForegroundColor Cyan
        } catch {
            Write-Host "Report saved to: $reportFile" -ForegroundColor Yellow
        }
        
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to generate report: $($_.Exception.Message)" -Component "Reporting"
        return $false
    }
}

# Start web dashboard
function Start-BaconDashboard {
    Write-BaconLog "INFO" "Starting Bacon web dashboard..." -Component "Dashboard"
    
    $dashboardPort = 8080
    $dashboardScript = @"
using namespace System.Net
using namespace System.Text

# Simple HTTP server for Bacon dashboard
\$listener = New-Object System.Net.HttpListener
\$listener.Prefixes.Add("http://localhost:$dashboardPort/")
\$listener.Start()

Write-Host "Bacon Dashboard running on http://localhost:$dashboardPort/" -ForegroundColor Green

while (\$true) {
    \$context = \$listener.GetContext()
    \$request = \$context.Request
    \$response = \$context.Response
    
    try {
        if (\$request.Url.LocalPath -eq "/") {
            \$status = Get-BaconStatus
            \$html = Generate-DashboardHtml -Status \$status
            \$buffer = [Text.Encoding]::UTF8.GetBytes(\$html)
        } elseif (\$request.Url.LocalPath -eq "/metrics") {
            \$metrics = Get-Content (Join-Path \$SessionsDir "metrics.json") -Raw
            \$buffer = [Text.Encoding]::UTF8.GetBytes(\$metrics)
            \$response.ContentType = "application/json"
        } elseif (\$request.Url.LocalPath -eq "/health") {
            \$health = @{status="ok"; timestamp=(Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")} | ConvertTo-Json
            \$buffer = [Text.Encoding]::UTF8.GetBytes(\$health)
            \$response.ContentType = "application/json"
        } else {
            \$response.StatusCode = 404
            \$buffer = [Text.Encoding]::UTF8.GetBytes("Not Found")
        }
        
        \$response.ContentLength64 = \$buffer.Length
        \$output = \$response.OutputStream
        \$output.Write(\$buffer, 0, \$buffer.Length)
        \$output.Close()
    } catch {
        \$response.StatusCode = 500
        \$buffer = [Text.Encoding]::UTF8.GetBytes("Internal Server Error")
        \$response.ContentLength64 = \$buffer.Length
        \$output = \$response.OutputStream
        \$output.Write(\$buffer, 0, \$buffer.Length)
        \$output.Close()
    }
}
"@

    # Check if port is available
    try {
        $testConnection = Test-NetConnection -ComputerName localhost -Port $dashboardPort -WarningAction SilentlyContinue -ErrorAction SilentlyContinue
        if ($testConnection.TcpTestSucceeded) {
            Write-BaconLog "WARN" "Port $dashboardPort is already in use" -Component "Dashboard"
            Write-Host "Port $dashboardPort is already in use. Try a different port or stop the existing service." -ForegroundColor Yellow
            return $false
        }
    } catch {
        # Port is available
    }

    # Start dashboard in background job
    try {
        $dashboardJob = Start-Job -ScriptBlock {
            param($script, $sessionsDir, $projectRoot)
            
            # Define helper functions in job context
            function Get-BaconStatus {
                # Simplified status for dashboard
                @{
                    Health = @{ Overall = "Good"; Score = 85 }
                    Orchestration = @{ Status = "Running" }
                    Sessions = @{ Total = 10; Hotspots = 2 }
                    Timestamp = Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ"
                }
            }
            
            function Generate-DashboardHtml {
                param($Status)
                
                $html = @"
<!DOCTYPE html>
<html>
<head>
    <title>Bacon Dashboard</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background: #2c3e50; color: white; padding: 20px; border-radius: 5px; }
        .metric { display: inline-block; margin: 10px; padding: 15px; background: #f8f9fa; border-radius: 5px; }
        .health-good { color: #28a745; }
        .health-fair { color: #ffc107; }
        .health-poor { color: #dc3545; }
    </style>
    <script>
        function refreshMetrics() {
            fetch('/metrics')
                .then(response => response.json())
                .then(data => {
                    console.log('Metrics updated:', data);
                });
        }
        setInterval(refreshMetrics, 30000); // Refresh every 30 seconds
    </script>
</head>
<body>
    <div class="header">
        <h1>Bacon Dashboard</h1>
        <p>Live monitoring of autonomous coding pipeline</p>
    </div>
    
    <div class="metric">
        <h3>Health</h3>
        <p class="health-$($Status.Health.Overall.ToLower())">$($Status.Health.Overall) ($($Status.Health.Score)/100)</p>
    </div>
    
    <div class="metric">
        <h3>Orchestration</h3>
        <p>$($Status.Orchestration.Status)</p>
    </div>
    
    <div class="metric">
        <h3>Sessions</h3>
        <p>Total: $($Status.Sessions.Total)</p>
        <p>Hotspots: $($Status.Sessions.Hotspots)</p>
    </div>
    
    <p>Last updated: $($Status.Timestamp)</p>
    <p>Auto-refreshing every 30 seconds...</p>
</body>
</html>
"@
                return $html
            }
            
            # Run the dashboard server
            Add-Type -TypeDefinition @"
using System;
using System.Net;
using System.Text;

public class SimpleHttpServer {
    private HttpListener listener;
    private string sessionsDir;
    private string projectRoot;
    
    public SimpleHttpServer(string prefix, string sessionsDir, string projectRoot) {
        this.listener = new HttpListener();
        this.listener.Prefixes.Add(prefix);
        this.sessionsDir = sessionsDir;
        this.projectRoot = projectRoot;
    }
    
    public void Start() {
        listener.Start();
        Console.WriteLine("Dashboard started on " + string.Join(", ", listener.Prefixes));
        
        while (listener.IsListening) {
            var context = listener.GetContext();
            ProcessRequest(context);
        }
    }
    
    public void Stop() {
        listener.Stop();
        listener.Close();
    }
    
    private void ProcessRequest(HttpListenerContext context) {
        var response = context.Response;
        
        try {
            string responseString = "<html><body><h1>Bacon Dashboard</h1><p>Under construction</p></body></html>";
            byte[] buffer = Encoding.UTF8.GetBytes(responseString);
            
            response.ContentLength64 = buffer.Length;
            response.OutputStream.Write(buffer, 0, buffer.Length);
        } catch (Exception ex) {
            Console.WriteLine("Error: " + ex.Message);
            response.StatusCode = 500;
        } finally {
            response.OutputStream.Close();
        }
    }
}
"@
            
            $server = New-Object SimpleHttpServer "http://localhost:8080/", $sessionsDir, $projectRoot
            $server.Start()
            
        } -ArgumentList $dashboardScript, $SessionsDir, $ProjectRoot -Name "BaconDashboard"
        
        # Store job ID
        $dashboardJob.Id | Out-File (Join-Path $SessionsDir "dashboard.pid") -Encoding UTF8
        
        Write-BaconLog "INFO" "Dashboard started (Job ID: $($dashboardJob.Id))" -Component "Dashboard"
        Write-Host "Bacon Dashboard started on http://localhost:$dashboardPort/" -ForegroundColor Green
        Write-Host "Job ID: $($dashboardJob.Id)" -ForegroundColor Cyan
        Write-Host "PID file: $(Join-Path $SessionsDir "dashboard.pid")" -ForegroundColor Cyan
        
        # Wait a moment and try to open browser
        Start-Sleep -Seconds 2
        try {
            Start-Process "http://localhost:$dashboardPort/"
        } catch {
            Write-Host "Open http://localhost:$dashboardPort/ in your browser" -ForegroundColor Yellow
        }
        
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to start dashboard: $($_.Exception.Message)" -Component "Dashboard"
        return $false
    }
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

        "rotate-keys" {
            Invoke-BaconRotateKeys
        }

        "report" {
            Invoke-BaconReport
        }

        "dashboard" {
            Start-BaconDashboard
        }

        default {
            Write-Host "Unknown action: $Action" -ForegroundColor Red
            Write-Host "Available actions: start, stop, status, logs, metrics, cleanup, test, apply-approved, rotate-keys, report, dashboard" -ForegroundColor Yellow
        }
    }
}

# Run main function
Main
