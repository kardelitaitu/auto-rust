@echo off
setlocal EnableExtensions DisableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "ROOT=%%~fI"

if not exist "%ROOT%\Cargo.toml" (
  echo auto: Cargo.toml not found at project root.
  exit /b 1
)

pushd "%ROOT%" || (
  echo auto: failed to enter project directory.
  exit /b 1
)

where cargo >nul 2>nul || (
  echo auto: cargo not found on PATH.
  popd
  exit /b 1
)

cargo run --release -- %*
set "EXIT_CODE=%ERRORLEVEL%"

popd
endlocal & exit /b %EXIT_CODE%
