# Windows Build Guide for Simsapa

This guide explains how to build the Simsapa Dhamma Reader on Windows.

## Prerequisites

### Required Software

1. **Qt 6.9.3 for MSVC**
   - Install Qt 6.9.3 with MSVC 2022 64-bit component
   - Default installation path: `C:\Qt\6.9.3\msvc2022_64`
   - Qt installer also includes CMake and Ninja

2. **Visual Studio 2022 or Build Tools**
   - Install Visual Studio 2022 Community Edition, or
   - Install Visual Studio Build Tools 2022
   - Required components: Desktop development with C++

3. **Rust Toolchain**
   - Install from https://rustup.rs/
   - Ensure `x86_64-pc-windows-msvc` toolchain is installed:
     ```powershell
     rustup target add x86_64-pc-windows-msvc
     ```

4. **Inno Setup 6** (for installer creation)
   - Download from https://jrsoftware.org/isdl.php
   - Default installation path: `C:\Program Files (x86)\Inno Setup 6`

## Build Process

### Quick Start

#### Option 1: PowerShell with Execution Policy Bypass (Recommended)

If you get an error about script execution being disabled, run:

```powershell
powershell -ExecutionPolicy Bypass -File build-windows.ps1
```

This bypasses the execution policy for this single script without changing system settings.

#### Option 2: Enable Scripts for Current User

Alternatively, you can enable script execution for your user account:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

Then run the script normally:

```powershell
.\build-windows.ps1
```

#### Option 3: Use Developer PowerShell for VS 2022 (RECOMMENDED)

The **Developer PowerShell for VS 2022** is the best option because:
- ✅ Execution policies already configured
- ✅ C++ compiler (MSVC) already in PATH
- ✅ All build tools ready to use
- ✅ No environment setup needed

**How to use:**
1. Open Start Menu
2. Search for "Developer PowerShell for VS 2022"
3. Navigate to project directory: `cd C:\path\to\simsapa-ng`
4. Run: `.\build-windows.ps1`

**Why this is better:** Regular PowerShell requires the script to configure the MSVC environment, which can sometimes fail. Developer PowerShell already has everything configured.

---

The build script will:
1. Build the application with CMake
2. Deploy Qt dependencies with windeployqt
3. Create an installer with Inno Setup

### Build Options

All these options use the `-ExecutionPolicy Bypass` method:

```powershell
# Show help
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -Help

# Clean build
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -Clean

# Build with custom Qt path
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -QtPath "C:\Qt\6.9.3\msvc2022_64"

# Skip installer creation (just build and deploy)
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -SkipInstaller

# Use existing build (only deploy and create installer)
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -SkipBuild
```

Or if you've set the execution policy, use the shorter form:

```powershell
.\build-windows.ps1 -Clean
.\build-windows.ps1 -QtPath "C:\Qt\6.9.3\msvc2022_64"
```

### Using Makefile (if Make is available)

```bash
# Build Windows installer
make windows

# Clean and rebuild
make windows-rebuild

# Clean only
make windows-clean
```

## Build Output

After a successful build, you will find:

- **Distribution folder:** `dist\simsapadhammareader.exe` with all Qt dependencies
- **Installer:** `Simsapa-Setup-{version}.exe` in the project root

## Installer Features

The Inno Setup installer includes:

1. **Application Installation**
   - Installs to `C:\Program Files\Simsapa` by default
   - Creates Start Menu shortcuts
   - Optional desktop icon

2. **Visual C++ Redistributable Check**
   - Warns if VC++ Redistributable is not installed
   - Provides download link if needed

3. **Clean Uninstall**
   - Removes application files
   - Optional: Delete user data and downloaded databases
   - User data location: `%LOCALAPPDATA%\profound-labs\simsapa-ng`

## Troubleshooting

### PowerShell Script Execution Disabled

**Problem:** Error message: "cannot be loaded because running scripts is disabled on this system"

**Solution 1 (Best):** Use **Developer PowerShell for VS 2022**
- Open Start Menu → Search "Developer PowerShell for VS 2022"
- Navigate to project directory
- Run: `.\build-windows.ps1`

**Solution 2:** Run with execution policy bypass:
```powershell
powershell -ExecutionPolicy Bypass -File build-windows.ps1
```

**Solution 3:** Enable scripts for current user:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

**Why this happens:** Windows blocks unsigned PowerShell scripts by default for security.

---

### CMake Cannot Find Build Program (Ninja) or C++ Compiler

**Problem:** 
```
CMake Error: CMake was unable to find a build program corresponding to "Ninja"
CMake Error: CMAKE_CXX_COMPILER not set, after EnableLanguage
```

**Cause:** The MSVC (Visual C++) compiler environment is not configured in your PowerShell session.

**Solution 1 (Recommended):** Use **Developer PowerShell for VS 2022**
- This PowerShell variant has the C++ compiler already configured
- Open Start Menu → Search "Developer PowerShell for VS 2022"
- Navigate to project and run: `.\build-windows.ps1`

**Solution 2:** Manually import Visual Studio environment:
```powershell
# Import the VS environment (adjust path if needed)
& "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1"

# Then run the build
.\build-windows.ps1
```

**Solution 3:** The script will now automatically try to import the MSVC environment. If this fails, use Solution 1.

**Why this happens:** CMake needs the Visual C++ compiler to build C++ code. The compiler is only available when the Visual Studio environment is properly configured.

---

### CMake Configuration Fails

**Problem:** CMake can't find Qt installation

**Solution:**
```powershell
powershell -ExecutionPolicy Bypass -File build-windows.ps1 -QtPath "C:\path\to\Qt\6.9.3\msvc2022_64"
```

### Build Fails with Linking Errors

**Problem:** MSVC runtime library mismatch

**Solution:** The CMakeLists.txt is configured to use `MultiThreadedDLL` runtime. Ensure you're building in Release mode and using the x86_64-pc-windows-msvc Rust toolchain.

### windeployqt Warnings

**Problem:** windeployqt shows warnings about missing files

**Solution:** This is often normal. The script continues and creates a working package. Test the executable in the `dist` folder.

### Inno Setup Not Found

**Problem:** Installer creation fails because ISCC.exe is not found

**Solution:** 
1. Install Inno Setup 6 from https://jrsoftware.org/isdl.php
2. Or skip installer creation with `-SkipInstaller` flag

## Manual Build Steps

If you prefer to build manually:

### 1. Configure with CMake

```powershell
cmake -S . -B .\build\simsapadhammareader `
  -DCMAKE_PREFIX_PATH=C:\Qt\6.9.3\msvc2022_64 `
  -DCMAKE_BUILD_TYPE=Release `
  -G Ninja
```

### 2. Build with CMake

```powershell
cmake --build .\build\simsapadhammareader --config Release
```

### 3. Deploy Qt Dependencies

```powershell
# Create distribution directory
New-Item -ItemType Directory -Path .\dist -Force

# Copy executable
Copy-Item .\build\simsapadhammareader\simsapadhammareader.exe -Destination .\dist\

# Run windeployqt
C:\Qt\6.9.3\msvc2022_64\bin\windeployqt.exe `
  .\dist\simsapadhammareader.exe `
  --qmldir assets\qml `
  --release
```

### 4. Create Installer

```powershell
# Using Inno Setup compiler
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" `
  /DAppVersion=0.1.8 `
  /DDistDir=.\dist `
  simsapa-installer.iss
```

## Environment Setup (Developer PowerShell)

For the best experience, use **Developer PowerShell for VS 2022**:

1. Open Start Menu
2. Search for "Developer PowerShell for VS 2022"
3. Navigate to project directory
4. Run build script

This ensures all MSVC tools are in PATH.

## Testing the Build

### Test the Distribution Folder

```powershell
# Run the deployed executable
.\dist\simsapadhammareader.exe
```

### Test the Installer

```powershell
# Run the installer
.\Simsapa-Setup-0.1.8.exe
```

After installation:
- Check Start Menu for Simsapa shortcut
- Verify the application launches
- Test downloading app databases
- Test uninstaller (should offer to delete user data)

## Customization

### Modify Version Number

Edit `bridges\Cargo.toml`:
```toml
[package]
version = "0.1.8"  # Change this
```

The build script automatically reads this version.

### Customize Installer

Edit `simsapa-installer.iss` to customize:
- Application name and publisher
- Installation directory
- Shortcuts and icons
- Uninstaller behavior

### Application Icon

The application uses:
- `assets\icons\appicons\simsapa.ico` - Windows icon
- `assets\icons\appicons\simsapa.rc` - Resource file with version info

## Continuous Integration

For automated builds (e.g., GitHub Actions), ensure:

1. Qt is installed and `CMAKE_PREFIX_PATH` is set
2. MSVC environment is initialized
3. Rust toolchain is available
4. All paths are absolute or properly escaped

Example environment variables:
```powershell
$env:CMAKE_PREFIX_PATH = "C:\Qt\6.9.3\msvc2022_64"
$env:PATH = "C:\Qt\6.9.3\msvc2022_64\bin;$env:PATH"
```

## Additional Notes

- Build time: ~5-15 minutes depending on system (first build takes longer due to Rust compilation)
- Distribution size: ~150-200 MB (includes Qt WebEngine)
- Installer size: ~80-100 MB (compressed)
- User data location: `%LOCALAPPDATA%\profound-labs\simsapa-ng` (databases downloaded at runtime)
- App data structure:
  - `userdata.sqlite3` - User's database (bookmarks, notes, etc.)
  - `app-assets/` - Downloaded language databases
  - `logs/` - Application logs

## Support

For build issues:
1. Check this guide's troubleshooting section
2. Verify all prerequisites are installed
3. Try a clean rebuild: `.\build-windows.ps1 -Clean`
4. Check the Qt and Rust versions match requirements
