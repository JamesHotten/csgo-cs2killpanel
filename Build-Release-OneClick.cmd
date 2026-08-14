@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Build-Release.ps1"
if errorlevel 1 (
  echo.
  echo Release build failed. Review the error above.
  pause
  exit /b 1
)
echo.
echo Transfer ZIP and release assets were generated successfully.
pause
