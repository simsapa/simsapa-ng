# Build Fix Summary - CMake/Ninja Path Issues

## What Was Fixed

The build script couldn't find CMake and Ninja because they weren't in the PATH. I've updated `build-windows.ps1` to:

1. **Automatically locate Qt Tools** - Correctly calculates the Tools directory from Qt installation path
2. **Add tools to PATH** - Adds CMake and Ninja directories to PATH before running CMake
3. **Import full MSVC environment** - Properly imports all Visual Studio environment variables
4. **Explicit Ninja path** - Tells CMake exactly where Ninja is located using `-DCMAKE_MAKE_PROGRAM`
5. **Better error messages** - Shows what was found and what's missing

## Changes Made

### File: `build-windows.ps1`

**Path Resolution:**
```powershell
# OLD (incorrect):
$qtToolsDir = Split-Path $QtPath | Join-Path -ChildPath "Tools"

# NEW (correct):
$qtRootDir = Split-Path (Split-Path $QtPath -Parent) -Parent  # C:\Qt
$qtToolsDir = Join-Path $qtRootDir "Tools"                     # C:\Qt\Tools
```

**PATH Setup:**
- Adds `C:\Qt\Tools\CMake_64\bin` to PATH
- Adds `C:\Qt\Tools\Ninja` to PATH
- These additions happen before CMake is run

**CMake Generator Selection:**
```powershell
# If Ninja found, use it with explicit path:
$cmakeArgs += "-G", "Ninja"
$cmakeArgs += "-DCMAKE_MAKE_PROGRAM=$ninja"  # Points to C:\Qt\Tools\Ninja\ninja.exe
```

**MSVC Environment:**
- Imports ALL environment variables from vcvars64.bat (not just selected ones)
- Uses temp file to reliably capture environment
- Verifies compiler (cl.exe) is available after import

### New File: `build-windows.bat`

Simple batch wrapper that:
- Finds and runs vcvars64.bat automatically
- Calls PowerShell build script
- Can be double-clicked or run from Command Prompt

## How to Test

### Option 1: Developer PowerShell (Recommended)

```powershell
cd C:\Users\Sumedharama\prods\simsapa-ng-project\simsapa-ng
.\build-windows.ps1
```

**Expected output:**
```
[INFO] Building Windows installer for Simsapa...
[INFO]   App Name: Simsapa
[INFO]   App Version: 0.1.8
[INFO]   Qt Path: C:\Qt\6.9.3\msvc2022_64
...
[INFO] ✓ Added Qt CMake to PATH: C:\Qt\Tools\CMake_64\bin
[INFO] ✓ Added Qt Ninja to PATH: C:\Qt\Tools\Ninja
[INFO] ✓ Found CMake: C:\Qt\Tools\CMake_64\bin\cmake.exe
[INFO] ✓ Found Ninja: C:\Qt\Tools\Ninja\ninja.exe
[INFO] ✓ Found windeployqt: C:\Qt\6.9.3\msvc2022_64\bin\windeployqt.exe
...
[INFO] Building application...
[INFO] Found Visual Studio at: C:\Program Files\Microsoft Visual Studio\2022\Community
[INFO] Setting up MSVC environment...
[INFO] MSVC environment configured
[INFO] ✓ C++ compiler (cl.exe) found in PATH
[INFO] Configuring CMake...
[INFO] Using Ninja build system
-- The CXX compiler identification is MSVC ...
-- Configuring done
-- Generating done
```

### Option 2: Verbose Mode (For Debugging)

```powershell
.\build-windows.ps1 -Verbose
```

This shows additional debug information about path resolution.

### Option 3: Batch File

```cmd
build-windows.bat
```

Or just double-click `build-windows.bat` in Windows Explorer.

## What Should Happen Now

1. **Tools Found:**
   - CMake: `C:\Qt\Tools\CMake_64\bin\cmake.exe`
   - Ninja: `C:\Qt\Tools\Ninja\ninja.exe`
   - Qt bin: `C:\Qt\6.9.3\msvc2022_64\bin\windeployqt.exe`

2. **Environment Configured:**
   - MSVC compiler (cl.exe) available
   - All Visual Studio build tools in PATH

3. **CMake Succeeds:**
   - Finds Ninja build program
   - Finds C++ compiler
   - Configures successfully

4. **Build Completes:**
   - Compiles Rust code
   - Compiles C++ code
   - Links executable
   - Deploys Qt dependencies
   - Creates installer

## If It Still Fails

### Enable Verbose Output

```powershell
.\build-windows.ps1 -Verbose
```

Look for these lines:
```
Debug: Qt Path: C:\Qt\6.9.3\msvc2022_64
Debug: Qt Root Dir: C:\Qt
Debug: Qt Tools Dir: C:\Qt\Tools
Debug: Expected CMake: C:\Qt\Tools\CMake_64\bin\cmake.exe
Debug: Expected Ninja: C:\Qt\Tools\Ninja\ninja.exe
```

### Check Qt Installation

Verify these paths exist:
```powershell
Test-Path C:\Qt\Tools\CMake_64\bin\cmake.exe
Test-Path C:\Qt\Tools\Ninja\ninja.exe
Test-Path C:\Qt\6.9.3\msvc2022_64\bin\qmake.exe
```

All should return `True`.

### Use Custom Qt Path

If Qt is installed elsewhere:
```powershell
.\build-windows.ps1 -QtPath "D:\Qt\6.9.3\msvc2022_64"
```

### Check MSVC Compiler

After running the script with `-Verbose`, verify:
```powershell
Get-Command cl.exe
```

Should show the path to the Visual C++ compiler.

## Common Issues

### "Qt Tools directory not found"

**Problem:** Qt Tools not installed or in different location

**Solution:** 
1. Check if `C:\Qt\Tools` exists
2. If not, reinstall Qt and ensure "Developer and Designer Tools" are selected
3. Or specify custom Qt path with `-QtPath`

### "C++ compiler not found in PATH"

**Problem:** Visual Studio environment not configured

**Solution:**
1. Use Developer PowerShell for VS 2022 (easiest)
2. Or run from Developer Command Prompt
3. Or manually import VS environment first:
   ```powershell
   & "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1"
   ```

### Build succeeds but installer fails

**Problem:** Inno Setup not installed

**Solution:**
1. Install from https://jrsoftware.org/isdl.php
2. Or skip installer: `.\build-windows.ps1 -SkipInstaller`

## Next Steps

1. **Try the build** using Developer PowerShell for VS 2022
2. **Check output** for the ✓ marks showing tools were found
3. **Report results** - let me know if you see any errors

The changes are comprehensive and should resolve the CMake/Ninja path issues completely!
