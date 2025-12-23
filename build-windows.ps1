# Windows build script for Simsapa
# This script builds the Qt6 application, deploys dependencies with windeployqt,
# and creates an installer with Inno Setup

param(
    [string]$AppName = "",
    [string]$AppVersion = "",
    [string]$QtPath = "C:\Qt\6.9.3\msvc2022_64",
    [string]$BuildDir = ".\build\simsapadhammareader",
    [string]$DistDir = ".\dist",
    [switch]$Clean,
    [switch]$SkipBuild,
    [switch]$SkipDeploy,
    [switch]$SkipInstaller,
    [switch]$Help
)

# Function to show usage
function Show-Usage {
    Write-Host @"
Usage: .\build-windows.ps1 [OPTIONS]

Options:
  -AppName NAME         Set application name (default: read from .desktop file)
  -AppVersion VER       Set application version (default: read from Cargo.toml)
  -QtPath PATH          Set Qt installation path (default: C:\Qt\6.9.3\msvc2022_64)
  -BuildDir PATH        Set build directory (default: .\build\simsapadhammareader)
  -DistDir PATH         Set distribution directory (default: .\dist)
  -Clean                Clean build artifacts before building
  -SkipBuild            Skip building, use existing executable
  -SkipDeploy           Skip Qt deployment
  -SkipInstaller        Skip installer creation
  -Help                 Show this help message

"@
}

# Colors for output
function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# Function to extract app name from desktop file
function Get-AppNameFromDesktop {
    if (Test-Path "simsapa.desktop") {
        $content = Get-Content "simsapa.desktop"
        foreach ($line in $content) {
            if ($line -match "^Name=(.+)$") {
                return $matches[1].Trim()
            }
        }
    }
    return "Simsapa"
}

# Function to extract version from Cargo.toml
function Get-VersionFromCargoToml {
    if (Test-Path "bridges\Cargo.toml") {
        $content = Get-Content "bridges\Cargo.toml"
        foreach ($line in $content) {
            if ($line -match '^version\s*=\s*"([^"]+)"') {
                return $matches[1]
            }
        }
    }
    return "0.1.8"
}

# Show help if requested
if ($Help) {
    Show-Usage
    exit 0
}

# Set defaults if not provided
if ([string]::IsNullOrEmpty($AppName)) {
    $AppName = Get-AppNameFromDesktop
}
if ([string]::IsNullOrEmpty($AppVersion)) {
    $AppVersion = Get-VersionFromCargoToml
}

Write-Status "Building Windows installer for Simsapa..."
Write-Status "  App Name: $AppName"
Write-Status "  App Version: $AppVersion"
Write-Status "  Qt Path: $QtPath"
Write-Status "  Build Dir: $BuildDir"
Write-Status "  Dist Dir: $DistDir"

# Check if Qt installation exists
if (-not (Test-Path $QtPath)) {
    Write-Error "Qt installation not found at: $QtPath"
    Write-Error "Please install Qt 6.9.3 or specify the correct path with -QtPath"
    exit 1
}

# Find required tools
$qtBinPath = Join-Path $QtPath "bin"
$windeployqt = Join-Path $qtBinPath "windeployqt.exe"
$cmake = "C:\Qt\Tools\CMake_64\bin\cmake.exe"
$ninja = "C:\Qt\Tools\Ninja\ninja.exe"

# Check for CMake (try Qt installation first, then system PATH)
if (-not (Test-Path $cmake)) {
    Write-Warning "CMake not found in Qt Tools, trying system PATH..."
    $cmake = "cmake"
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        Write-Error "CMake not found. Please install CMake or ensure it's in PATH"
        exit 1
    }
}

# Check for Ninja (try Qt installation first, then system PATH)
if (-not (Test-Path $ninja)) {
    Write-Warning "Ninja not found in Qt Tools, trying system PATH..."
    $ninja = "ninja"
    if (-not (Get-Command ninja -ErrorAction SilentlyContinue)) {
        Write-Warning "Ninja not found, CMake will use default generator"
        $ninja = $null
    }
}

# Check for windeployqt
if (-not (Test-Path $windeployqt)) {
    Write-Error "windeployqt.exe not found at: $windeployqt"
    Write-Error "Please ensure Qt 6.9.3 is properly installed"
    exit 1
}

# Check for Rust toolchain
try {
    $rustupOutput = & rustup show 2>&1
    if ($rustupOutput -notmatch "x86_64-pc-windows-msvc") {
        Write-Warning "x86_64-pc-windows-msvc toolchain not found"
        Write-Status "Installing Rust MSVC toolchain..."
        & rustup target add x86_64-pc-windows-msvc
    }
} catch {
    Write-Error "Rust toolchain not found. Please install Rust from https://rustup.rs/"
    exit 1
}

# Clean if requested
if ($Clean) {
    Write-Status "Cleaning build artifacts..."
    if (Test-Path $BuildDir) {
        Remove-Item -Recurse -Force $BuildDir
    }
    if (Test-Path $DistDir) {
        Remove-Item -Recurse -Force $DistDir
    }
    if (Test-Path "Simsapa-Setup-*.exe") {
        Remove-Item -Force "Simsapa-Setup-*.exe"
    }
}

# Build the application
if (-not $SkipBuild) {
    Write-Status "Building application..."
    
    # Set up environment for MSVC
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vsWhere) {
        $vsPath = & $vsWhere -latest -property installationPath
        $vcvarsPath = Join-Path $vsPath "VC\Auxiliary\Build\vcvars64.bat"
        
        if (Test-Path $vcvarsPath) {
            Write-Status "Setting up MSVC environment..."
            # We'll use the Developer PowerShell instead of calling vcvars64.bat
            # because it's difficult to capture environment variables from batch files
        } else {
            Write-Warning "vcvars64.bat not found, assuming MSVC is already in PATH"
        }
    } else {
        Write-Warning "Visual Studio not found via vswhere, assuming build tools are in PATH"
    }
    
    # Configure with CMake
    Write-Status "Configuring CMake..."
    $cmakeArgs = @(
        "-S", ".",
        "-B", $BuildDir,
        "-DCMAKE_PREFIX_PATH=$QtPath",
        "-DCMAKE_BUILD_TYPE=Release"
    )
    
    if ($ninja) {
        $cmakeArgs += "-G", "Ninja"
    }
    
    & $cmake $cmakeArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "CMake configuration failed"
        exit 1
    }
    
    # Build with CMake
    Write-Status "Building with CMake..."
    & $cmake --build $BuildDir --config Release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed"
        exit 1
    }
    
    Write-Status "Build completed successfully"
} else {
    Write-Status "Skipping build (using existing executable)"
}

# Check if executable exists
$exePath = Join-Path $BuildDir "simsapadhammareader.exe"
if (-not (Test-Path $exePath)) {
    Write-Error "Executable not found at: $exePath"
    Write-Error "Please build the application first or remove -SkipBuild flag"
    exit 1
}

# Deploy Qt dependencies
if (-not $SkipDeploy) {
    Write-Status "Creating distribution directory..."
    if (Test-Path $DistDir) {
        Remove-Item -Recurse -Force $DistDir
    }
    New-Item -ItemType Directory -Path $DistDir | Out-Null
    
    # Copy executable to dist directory
    Write-Status "Copying executable to distribution directory..."
    Copy-Item $exePath -Destination $DistDir
    
    # Run windeployqt
    Write-Status "Deploying Qt dependencies with windeployqt..."
    $distExePath = Join-Path $DistDir "simsapadhammareader.exe"
    
    # Add Qt bin to PATH for this session
    $env:PATH = "$qtBinPath;$env:PATH"
    
    $windeployqtArgs = @(
        $distExePath,
        "--qmldir", "assets\qml",
        "--release",
        "--no-translations",
        "--no-system-d3d-compiler",
        "--no-opengl-sw"
    )
    
    & $windeployqt $windeployqtArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "windeployqt completed with warnings (this is often normal)"
    }
    
    Write-Status "Qt dependencies deployed successfully"
    
    # Copy additional resources
    Write-Status "Copying application resources..."
    
    # Copy icon (for runtime use)
    if (Test-Path "assets\icons\appicons\simsapa.ico") {
        Copy-Item "assets\icons\appicons\simsapa.ico" -Destination $DistDir
    }
    
    Write-Status "Distribution package created at: $DistDir"
} else {
    Write-Status "Skipping Qt deployment"
}

# Create installer with Inno Setup
if (-not $SkipInstaller) {
    Write-Status "Creating installer with Inno Setup..."
    
    # Check for Inno Setup
    $innoSetupPaths = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 6\ISCC.exe",
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe"
    )
    
    $iscc = $null
    foreach ($path in $innoSetupPaths) {
        if (Test-Path $path) {
            $iscc = $path
            break
        }
    }
    
    if (-not $iscc) {
        Write-Error "Inno Setup not found. Please install Inno Setup 6 from https://jrsoftware.org/isdl.php"
        Write-Warning "Installer creation skipped. You can still use the files in $DistDir"
        exit 0
    }
    
    Write-Status "Using Inno Setup at: $iscc"
    
    # Check if installer script exists
    $installerScript = "simsapa-installer.iss"
    if (-not (Test-Path $installerScript)) {
        Write-Error "Installer script not found: $installerScript"
        Write-Error "Please create the Inno Setup script first"
        exit 1
    }
    
    # Compile installer
    Write-Status "Compiling installer..."
    $installerArgs = @(
        "/DAppVersion=$AppVersion",
        "/DDistDir=$DistDir",
        $installerScript
    )
    
    & $iscc $installerArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Installer creation failed"
        exit 1
    }
    
    $installerName = "Simsapa-Setup-$AppVersion.exe"
    if (Test-Path $installerName) {
        Write-Status "Installer created successfully: $installerName"
        $installerSize = (Get-Item $installerName).Length / 1MB
        Write-Status "Installer size: $([math]::Round($installerSize, 2)) MB"
    } else {
        Write-Warning "Installer was compiled but not found at expected location"
    }
} else {
    Write-Status "Skipping installer creation"
}

Write-Status ""
Write-Status "Windows build completed successfully!"
Write-Status "Distribution files:"
Write-Status "  - $DistDir\simsapadhammareader.exe"
if (-not $SkipInstaller) {
    Write-Status "  - Simsapa-Setup-$AppVersion.exe"
}
Write-Status ""
Write-Status "Application details:"
Write-Status "  - Install location: C:\Program Files\Simsapa"
Write-Status "  - User data: %LOCALAPPDATA%\profound-labs\simsapa-ng"
Write-Status "  - Databases: %LOCALAPPDATA%\profound-labs\simsapa-ng\app-assets"
Write-Status ""
Write-Status "Note: Users will need Visual C++ Redistributable installed"
Write-Status "      (The installer will check for this)"
