# Windows Build - Quick Start

## TL;DR - Just Build It!

### ⭐ RECOMMENDED: Developer PowerShell for VS 2022

**This is the ONLY reliably working method:**

1. Open **Start Menu**
2. Search for: **Developer PowerShell for VS 2022**
3. Navigate to project:
   ```powershell
   cd C:\Users\Sumedharama\prods\simsapa-ng-project\simsapa-ng
   ```
4. Run:
   ```powershell
   .\build-windows.ps1
   ```

**Why this is required:** The build needs Windows SDK tools (rc.exe, mt.exe) which are ONLY available in Developer PowerShell. Regular PowerShell will fail with "rc.exe not found" errors.

---

### ❌ Methods That DON'T Work Reliably

These methods will likely fail with "rc.exe not found" errors:

- ❌ Double-clicking `build-windows.bat`
- ❌ Running from regular PowerShell
- ❌ Running from Command Prompt

**The problem:** These don't have the full Visual Studio environment configured.

---

### Alternative: Import VS Environment First (Advanced)

If you must use regular PowerShell:

```powershell
# Step 1: Import Visual Studio environment
& "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1" -Arch amd64 -HostArch amd64

# Step 2: Run build
.\build-windows.ps1
```

But using Developer PowerShell directly is much simpler!

---

That's it! This will build the app, deploy dependencies, and create an installer.

---

## Understanding the Different Methods

### Method 1: Developer PowerShell for VS 2022 (RECOMMENDED)

**Pros:**
- ✅ C++ compiler already configured
- ✅ Build tools in PATH
- ✅ Scripts allowed by default
- ✅ Just run `.\build-windows.ps1`

**How to find it:**
- Open Start Menu → Search "Developer PowerShell for VS 2022"

### Method 2: Regular PowerShell with Execution Policy Bypass

**When to use:** If you don't have Visual Studio installed or prefer regular PowerShell

**Command:**
```powershell
powershell -ExecutionPolicy Bypass -File build-windows.ps1
```

**Note:** The script will try to configure MSVC environment automatically, but may need Developer PowerShell if it fails.

### Method 3: Enable Scripts Permanently

**One-time setup:**
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

Then run normally:
```powershell
.\build-windows.ps1
```

### Method 4: Use Makefile

If you have Make installed (e.g., via Git Bash):
```bash
make windows
```

---

## What You Need Installed

1. ✅ Qt 6.9.3 (MSVC 2022 64-bit) - Should be at `C:\Qt\6.9.3\msvc2022_64`
2. ✅ Visual Studio 2022 or Build Tools (with C++ support)
3. ✅ Rust with `x86_64-pc-windows-msvc` toolchain
4. ✅ Inno Setup 6 (for installer creation)

### Quick Check

```powershell
# Check if Qt is installed
Test-Path C:\Qt\6.9.3\msvc2022_64

# Check if Rust is installed
rustup show

# Check if Inno Setup is installed
Test-Path "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

---

## Common Build Options

```powershell
# Clean build (remove old files first)
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -Clean

# Skip installer creation (just build the app)
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -SkipInstaller

# Custom Qt path
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -QtPath "D:\Qt\6.9.3\msvc2022_64"

# See all options
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -Help
```

---

## What Gets Created

After build completes:

- **`dist\simsapadhammareader.exe`** - Application with Qt dependencies
- **`Simsapa-Setup-0.1.8.exe`** - Windows installer

---

## First Time? Install Prerequisites

### 1. Install Qt 6.9.3

Download Qt Online Installer: https://www.qt.io/download-qt-installer

During installation:
- Select version: 6.9.3
- Select component: MSVC 2022 64-bit
- Install to: `C:\Qt` (default)

### 2. Install Visual Studio Build Tools

Download: https://visualstudio.microsoft.com/downloads/

Options:
- **VS 2022 Community** (full IDE), or
- **Build Tools for VS 2022** (compiler only)

During installation:
- Select: "Desktop development with C++"

### 3. Install Rust

Download: https://rustup.rs/

Run installer, then:
```powershell
rustup target add x86_64-pc-windows-msvc
```

### 4. Install Inno Setup (for installer)

Download: https://jrsoftware.org/isdl.php

Install to default location: `C:\Program Files (x86)\Inno Setup 6`

---

## Troubleshooting

### "Scripts disabled" Error

**Error:**
```
build-windows.ps1 cannot be loaded because running scripts is disabled
```

**Solution 1 (Best):** Use Developer PowerShell for VS 2022 instead

**Solution 2:** Use the full command with `-ExecutionPolicy Bypass`:
```powershell
powershell -ExecutionPolicy Bypass -File build-windows.ps1
```

---

### "CMake was unable to find a build program" or "CMAKE_CXX_COMPILER not set"

**Error:**
```
CMake Error: CMake was unable to find a build program corresponding to "Ninja"
CMake Error: CMAKE_CXX_COMPILER not set
```

**Cause:** MSVC environment (C++ compiler) is not configured.

**Solution (Recommended):** Use **Developer PowerShell for VS 2022**:
1. Open Start Menu
2. Search for "Developer PowerShell for VS 2022"
3. Navigate to project: `cd C:\Users\YourName\prods\simsapa-ng-project\simsapa-ng`
4. Run: `.\build-windows.ps1`

**Alternative Solution:** Import Visual Studio environment manually:
```powershell
# Run this first to set up the C++ compiler
& "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1"

# Then run the build
.\build-windows.ps1
```

---

### "Qt not found" Error

**Solution:** Specify Qt path:
```powershell
.\build-windows.ps1 -QtPath "C:\Qt\6.9.3\msvc2022_64"
```

Or if using regular PowerShell:
```powershell
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -QtPath "C:\Qt\6.9.3\msvc2022_64"
```

---

### "Ninja not found" but Visual Studio is installed

**Solution:** Let CMake use Visual Studio instead of Ninja. The script will automatically detect this and use the MSVC generator.

No action needed - just run from Developer PowerShell for VS 2022.

---

### Build Fails

1. **First, try Developer PowerShell for VS 2022** (most common fix)

2. **Try clean build:**
   ```powershell
   .\build-windows.ps1 -Clean
   ```

3. **Check Visual Studio installation:**
   - Open Visual Studio Installer
   - Verify "Desktop development with C++" is installed

4. **Check prerequisites** are all installed (see section below)

---

## Testing the Build

### Test the Distribution Folder

```powershell
.\dist\simsapadhammareader.exe
```

### Test the Installer

```powershell
.\Simsapa-Setup-0.1.8.exe
```

After installation:
- Check Start Menu for "Simsapa" shortcut
- Test downloading a language database
- Check user data at: `%LOCALAPPDATA%\profound-labs\simsapa-ng`

---

## Need More Help?

See the full guide: [WINDOWS_BUILD_GUIDE.md](WINDOWS_BUILD_GUIDE.md)
