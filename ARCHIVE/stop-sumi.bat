@echo off
title Stop Sumi Workstation
echo ========================================================
echo        Stopping Sumi Local Replay Workstation
echo ========================================================
echo.

echo Searching and stopping processes on port 8000 and 5173...

:: Kill processes on port 8000 (Backend)
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":8000" ^| findstr "LISTENING"') do (
    echo Stopping Backend process PID: %%a
    taskkill /F /PID %%a >nul 2>&1
)

:: Kill processes on port 5173 (Frontend)
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":5173" ^| findstr "LISTENING"') do (
    echo Stopping Frontend process PID: %%a
    taskkill /F /PID %%a >nul 2>&1
)

echo.
echo [OK] All Sumi services stopped.
timeout /t 3 >nul
