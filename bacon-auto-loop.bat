@echo off
setlocal enabledelayedexpansion

:: =============================================================================
:: Bacon Autonomous Loop
:: Runs the pipeline every 10 seconds (after the previous run completes)
:: in a new window with a unique log file.
:: =============================================================================

set "LOG_DIR=bacon-logs"
set "INTERVAL_SECS=10"
set "TIMESTAMP_FORMAT=yyyyMMdd_HHmmss"

:: Ensure we are in the project root directory
cd /d "%~dp0"

:: Create log directory if it doesn't exist
if not exist %LOG_DIR% mkdir %LOG_DIR%

echo [%DATE% %TIME%] Starting Bacon Autonomous Loop Master Process...
echo Interval: %INTERVAL_SECS% seconds (after each run completes)

:loop
set "CURRENT_TIMESTAMP=!DATE:~10,4!!DATE:~4,2!!DATE:~7,2!_!TIME:~0,2!!TIME:~3,2!!TIME:~6,2!"
set "RUN_LOG_FILE=%LOG_DIR%\bacon-run_!CURRENT_TIMESTAMP!.log"

echo.
echo [%DATE% %TIME%] Running Bacon Pipeline... (Log: !RUN_LOG_FILE!)
echo [%DATE% %TIME%] Running Bacon Pipeline... (Log: !RUN_LOG_FILE!) >> %LOG_DIR%\bacon-master.log

:: Start bacon in a new detached command prompt window
:: /wait makes the START command wait for the new window to close.
:: This ensures the master script waits for the pipeline to finish.
start "Bacon Pipeline Run !CURRENT_TIMESTAMP!" /wait cmd /c (
    echo [%DATE% %TIME%] Starting cargo run --bin bacon -- --auto ... > "!RUN_LOG_FILE!" 2>&1
    cargo run --bin bacon -- --auto >> "!RUN_LOG_FILE!" 2>&1
    if !ERRORLEVEL! NEQ 0 (
        echo [%DATE% %TIME%] Pipeline finished with errors (Exit Code: !ERRORLEVEL!). >> "!RUN_LOG_FILE!" 2>&1
        echo [%DATE% %TIME%] Pipeline FAILED (Log: !RUN_LOG_FILE! / Exit: !ERRORLEVEL!). | tee -a %LOG_DIR%\bacon-master.log
    ) else (
        echo [%DATE% %TIME%] Pipeline cycle completed successfully. >> "!RUN_LOG_FILE!" 2>&1
        echo [%DATE% %TIME%] Pipeline SUCCESS (Log: !RUN_LOG_FILE!). | tee -a %LOG_DIR%\bacon-master.log
    )
)

echo [%DATE% %TIME%] Bacon Pipeline finished. Waiting %INTERVAL_SECS% seconds before next cycle...
echo [%DATE% %TIME%] Waiting %INTERVAL_SECS% seconds before next cycle... >> %LOG_DIR%\bacon-master.log
timeout /t %INTERVAL_SECS% /nobreak > nul

goto loop
