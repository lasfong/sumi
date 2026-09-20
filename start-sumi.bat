@echo off
title Sumi Workstation Starter
echo ========================================================
echo        Starting Sumi Local Replay Workstation
echo ========================================================
echo.

set ROOT_DIR=%~dp0
cd /d "%ROOT_DIR%"

:: 1. Check Python virtual environment
if exist "%ROOT_DIR%backend\.venv\Scripts\python.exe" (
    set "PYTHON_EXE=%ROOT_DIR%backend\.venv\Scripts\python.exe"
    echo [OK] Using Python venv at backend\.venv
) else (
    set "PYTHON_EXE=python"
    echo [NOTE] Using system python
)

:: 2. Check Frontend node_modules
if not exist "%ROOT_DIR%frontend\node_modules" (
    echo [WARN] frontend\node_modules not found. Installing dependencies...
    cd /d "%ROOT_DIR%frontend"
    call npm install
    cd /d "%ROOT_DIR%"
)

:: 3. Start Backend in separate window
echo [1/2] Starting Backend (FastAPI on http://localhost:8000)...
start "Sumi Backend (Port 8000)" cmd /k "cd /d %ROOT_DIR%backend && set PYTHONPATH=. && "%PYTHON_EXE%" -m uvicorn app.main:app --port 8000"

:: 4. Start Frontend in separate window
echo [2/2] Starting Frontend (Vite on http://localhost:5173)...
start "Sumi Frontend (Port 5173)" cmd /k "cd /d %ROOT_DIR%frontend && npm run dev"

:: 5. Wait for servers to spin up, then open browser
echo.
echo Waiting for servers to initialize...
timeout /t 4 /nobreak >nul

echo Opening browser at http://localhost:5173 ...
start http://localhost:5173

echo.
echo ========================================================
echo  Sumi is running!
echo  - Frontend Web App: http://localhost:5173
echo  - Backend API:      http://localhost:8000
echo  - API Docs:         http://localhost:8000/docs
echo.
echo  To stop Sumi, run stop-sumi.bat or close the 2 server windows.
echo ========================================================
timeout /t 5 >nul
