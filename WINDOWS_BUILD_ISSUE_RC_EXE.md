# Windows Build Issue: rc.exe Not Found

## The Error

```
RC Pass 1: command "rc /fo ..." failed (exit code 0) with the following output:
no such file or directory
ninja: build stopped: subcommand failed.
```

## What This Means

CMake found the C++ compiler but cannot find the Windows SDK resource compiler (`rc.exe`) and manifest tool (`mt.exe`). These tools are part of the Windows SDK and are **only available in the PATH when using Developer PowerShell**.

## Why Regular PowerShell Doesn't Work

Regular PowerShell sessions don't have the Visual Studio build environment configured. Even though the build script tries to import the environment from `vcvars64.bat`, this doesn't work perfectly because:

1. PowerShell environment import from CMD batch files is unreliable
2. Some environment variables may not be captured correctly
3. The Windows SDK paths may not be fully configured

## The Solution: Use Developer PowerShell for VS 2022

### ✅ RECOMMENDED METHOD

This is the **only** reliable way to build on Windows:

1. **Open Start Menu**
2. **Search for:** `Developer PowerShell for VS 2022`
3. **Navigate to project:**
   ```powershell
   cd C:\Users\Sumedharama\prods\simsapa-ng-project\simsapa-ng
   ```
4. **Run build script:**
   ```powershell
   .\build-windows.ps1
   ```

### Why This Works

Developer PowerShell for VS 2022:
- ✅ Pre-configures all Visual Studio environment variables
- ✅ Adds Windows SDK bin directories to PATH
- ✅ Sets up compiler, linker, and resource compiler paths
- ✅ Ensures rc.exe, mt.exe, and all build tools are available
- ✅ No manual environment setup needed

## Alternative Methods (Not Recommended)

### Option A: Import VS Environment Manually

This **might** work but is less reliable:

```powershell
# In regular PowerShell, run this first:
& "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1" -Arch amd64 -HostArch amd64

# Then run the build:
.\build-windows.ps1
```

### Option B: Use the VS Environment Wrapper Script

```powershell
powershell -ExecutionPolicy Bypass -File build-windows-with-vsenv.ps1
```

This script tries to automatically import the VS environment, but may still fail.

### Option C: Use Visual Studio Developer Command Prompt

1. Open Start Menu
2. Search for: `Developer Command Prompt for VS 2022`
3. Navigate to project directory
4. Run: `powershell -ExecutionPolicy Bypass -File build-windows.ps1`

## Verification

In Developer PowerShell for VS 2022, you should see:

```powershell
# Check if environment is loaded:
$env:VSCMD_VER
# Should output: 17.x.x

# Check if tools are available:
Get-Command rc.exe
Get-Command mt.exe
Get-Command cl.exe
# All should show paths in Program Files\Microsoft Visual Studio\...
```

## What You'll See When It Works

```
[INFO] Building Windows installer for Simsapa...
[INFO] [OK] Added Qt CMake to PATH: C:\Qt\Tools\CMake_64\bin
[INFO] [OK] Added Qt Ninja to PATH: C:\Qt\Tools\Ninja
[INFO] [OK] Found CMake: C:\Qt\Tools\CMake_64\bin\cmake.exe
[INFO] [OK] Found Ninja: C:\Qt\Tools\Ninja\ninja.exe
[INFO] [OK] Running in Visual Studio Developer environment (version 17.x.x)
[INFO] [OK] C++ compiler (cl.exe) found
[INFO] [OK] Linker (link.exe) found
[INFO] [OK] Resource compiler (rc.exe) found
[INFO] Configuring CMake...
-- The CXX compiler identification is MSVC 19.44.35222.0
-- Detecting CXX compiler ABI info
-- Detecting CXX compiler ABI info - done
-- Configuring done
-- Generating done
```

## Key Takeaway

**Do NOT use regular PowerShell for building C++ projects on Windows.**

Always use one of these instead:
1. ✅ Developer PowerShell for VS 2022 (best)
2. ✅ Developer Command Prompt for VS 2022
3. ✅ Regular PowerShell/CMD **after** importing VS environment

The build script has been updated to detect this and show a clear error message.
