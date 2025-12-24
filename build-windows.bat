@echo off
REM Windows Build Script Wrapper
REM This batch file sets up the Visual Studio environment and runs the PowerShell build script

echo ========================================
echo Simsapa Windows Build
echo ========================================
echo.

REM Check if running from Developer Command Prompt
if defined VSCMD_VER (
    echo [OK] Running in Visual Studio Developer environment
    goto :RunBuild
)

REM Try to find and run vcvars64.bat
set "VCVARS_PATH=%ProgramFiles%\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS_PATH%" (
    echo [INFO] Setting up Visual Studio 2022 Community environment...
    call "%VCVARS_PATH%"
    goto :RunBuild
)

set "VCVARS_PATH=%ProgramFiles%\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS_PATH%" (
    echo [INFO] Setting up Visual Studio 2022 Professional environment...
    call "%VCVARS_PATH%"
    goto :RunBuild
)

set "VCVARS_PATH=%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS_PATH%" (
    echo [INFO] Setting up Visual Studio 2022 Build Tools environment...
    call "%VCVARS_PATH%"
    goto :RunBuild
)

echo [WARNING] Visual Studio environment not found
echo [WARNING] The build may fail if MSVC is not in PATH
echo.
echo Recommended: Run this from "Developer Command Prompt for VS 2022"
echo   Or open: "Developer PowerShell for VS 2022" and run: .\build-windows.ps1
echo.
pause

:RunBuild
echo.
echo [INFO] Running PowerShell build script...
echo.

REM Run the PowerShell script with execution policy bypass
powershell -ExecutionPolicy Bypass -File "%~dp0build-windows.ps1" %*

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ========================================
    echo Build completed successfully!
    echo ========================================
) else (
    echo.
    echo ========================================
    echo Build failed with error code: %ERRORLEVEL%
    echo ========================================
    echo.
    echo Try running from "Developer PowerShell for VS 2022" instead
)

echo.
pause
