@echo off
setlocal enabledelayedexpansion

:: =============================================================================
:: Bacon Autonomous Loop
:: Runs the pipeline every 10 seconds (after the previous run completes)
:: with a unique log file.
:: =============================================================================

set "LOG_DIR=bacon-logs"
set "INTERVAL_SECS=10"
set "RUN_TIMEOUT_SECS=300"
if not "%BACON_INTERVAL_SECS%"=="" set "INTERVAL_SECS=%BACON_INTERVAL_SECS%"
if not "%BACON_RUN_TIMEOUT_SECS%"=="" set "RUN_TIMEOUT_SECS=%BACON_RUN_TIMEOUT_SECS%"

:: Ensure we are in the project root directory
cd /d "%~dp0"

:: Create log directory if it doesn't exist
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"

echo [%DATE% %TIME%] Starting Bacon Autonomous Loop Master Process...
echo Interval: %INTERVAL_SECS% seconds after each run completes
echo Run timeout: %RUN_TIMEOUT_SECS% seconds

:loop
for /f %%I in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd_HHmmss"') do set "CURRENT_TIMESTAMP=%%I"
set "RUN_LOG_FILE=%LOG_DIR%\bacon-run_!CURRENT_TIMESTAMP!.log"

echo.
echo [%DATE% %TIME%] Running Bacon Pipeline... (Log: !RUN_LOG_FILE!)
echo [%DATE% %TIME%] Running Bacon Pipeline... (Log: !RUN_LOG_FILE!) >> "%LOG_DIR%\bacon-master.log"

:: Run one Bacon cycle with a watchdog so a slow provider call cannot hang the loop.
powershell -NoProfile -ExecutionPolicy Bypass -File ".bacon\scripts\bacon-run-once.ps1" -LogFile "!RUN_LOG_FILE!" -TimeoutSeconds %RUN_TIMEOUT_SECS%
set "RUN_EXIT_CODE=!ERRORLEVEL!"

if !RUN_EXIT_CODE! NEQ 0 (
    echo [%DATE% %TIME%] Pipeline finished with errors - Exit Code: !RUN_EXIT_CODE!. >> "!RUN_LOG_FILE!" 2>&1
    echo [%DATE% %TIME%] Pipeline FAILED - Log: !RUN_LOG_FILE! / Exit: !RUN_EXIT_CODE!.
    echo [%DATE% %TIME%] Pipeline FAILED - Log: !RUN_LOG_FILE! / Exit: !RUN_EXIT_CODE!. >> "%LOG_DIR%\bacon-master.log"
) else (
    echo [%DATE% %TIME%] Pipeline cycle completed successfully. >> "!RUN_LOG_FILE!" 2>&1
    echo [%DATE% %TIME%] Pipeline SUCCESS - Log: !RUN_LOG_FILE!.
    echo [%DATE% %TIME%] Pipeline SUCCESS - Log: !RUN_LOG_FILE!. >> "%LOG_DIR%\bacon-master.log"
)

if /I "%BACON_LOOP_ONCE%"=="1" (
    echo [%DATE% %TIME%] One-shot mode complete. Exiting with code !RUN_EXIT_CODE!.
    echo [%DATE% %TIME%] One-shot mode complete. Exit: !RUN_EXIT_CODE!. >> "%LOG_DIR%\bacon-master.log"
    goto finish
)

echo [%DATE% %TIME%] Bacon Pipeline finished. Waiting %INTERVAL_SECS% seconds before next cycle...
echo [%DATE% %TIME%] Waiting %INTERVAL_SECS% seconds before next cycle... >> "%LOG_DIR%\bacon-master.log"
timeout /t %INTERVAL_SECS% /nobreak > nul

goto loop

:finish
endlocal & exit %RUN_EXIT_CODE%
