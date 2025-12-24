# Windows Build Script with Automatic VS Environment Setup
# This script imports the Visual Studio environment before running the build

param(
    [switch]$Clean,
    [switch]$SkipInstaller,
    [switch]$Help
)

# Check if already in VS environment
if ($env:VSCMD_VER -or $env:VisualStudioVersion) {
    Write-Host "[INFO] Already in Visual Studio environment, proceeding with build..." -ForegroundColor Green
    & "$PSScriptRoot\build-windows.ps1" @PSBoundParameters
    exit $LASTEXITCODE
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Simsapa Windows Build with VS Environment" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Find Visual Studio installation
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"

if (-not (Test-Path $vsWhere)) {
    Write-Host "[ERROR] Visual Studio not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please install Visual Studio 2022 with C++ development tools" -ForegroundColor Yellow
    Write-Host "Or use Developer PowerShell for VS 2022 instead" -ForegroundColor Yellow
    exit 1
}

$vsPath = & $vsWhere -latest -property installationPath

if (-not $vsPath) {
    Write-Host "[ERROR] Could not locate Visual Studio installation" -ForegroundColor Red
    exit 1
}

Write-Host "[INFO] Found Visual Studio at: $vsPath" -ForegroundColor Green

# Find the VS DevShell script
$devShellScript = Join-Path $vsPath "Common7\Tools\Launch-VsDevShell.ps1"

if (-not (Test-Path $devShellScript)) {
    Write-Host "[ERROR] Developer Shell script not found at: $devShellScript" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please use Developer PowerShell for VS 2022 instead:" -ForegroundColor Yellow
    Write-Host "  1. Open Start Menu" -ForegroundColor Yellow
    Write-Host "  2. Search: Developer PowerShell for VS 2022" -ForegroundColor Yellow
    Write-Host "  3. Run: .\build-windows.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host "[INFO] Importing Visual Studio environment..." -ForegroundColor Green
Write-Host ""

# Import VS environment
try {
    & $devShellScript -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
    
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to import VS environment"
    }
    
    Write-Host "[OK] Visual Studio environment imported successfully" -ForegroundColor Green
    Write-Host ""
    
    # Now run the build script in this environment
    Write-Host "[INFO] Running build script..." -ForegroundColor Green
    Write-Host ""
    
    & "$PSScriptRoot\build-windows.ps1" @PSBoundParameters
    exit $LASTEXITCODE
    
} catch {
    Write-Host ""
    Write-Host "[ERROR] Failed to set up Visual Studio environment: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "RECOMMENDED: Use Developer PowerShell for VS 2022 instead" -ForegroundColor Yellow
    Write-Host "  1. Open Start Menu" -ForegroundColor Yellow
    Write-Host "  2. Search: Developer PowerShell for VS 2022" -ForegroundColor Yellow
    Write-Host "  3. Navigate to: $PSScriptRoot" -ForegroundColor Yellow
    Write-Host "  4. Run: .\build-windows.ps1" -ForegroundColor Yellow
    Write-Host ""
    exit 1
}
