; Simsapa Dhamma Reader - Inno Setup Installer Script
; This script creates a Windows installer for the Simsapa application

#ifndef AppVersion
  #define AppVersion "0.1.8"
#endif

#ifndef DistDir
  #define DistDir ".\dist"
#endif

#define AppName "Simsapa"
#define AppPublisher "Profound Labs"
#define AppURL "https://simsapa.github.io/"
#define AppExeName "simsapadhammareader.exe"
#define AppId "{{B8F8C8A0-9F8E-4E5A-8C9D-1E2F3A4B5C6D}"

[Setup]
; NOTE: The value of AppId uniquely identifies this application.
; Do not use the same AppId value in installers for other applications.
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
AllowNoIcons=yes
; Uncomment the following line if you have a LICENSE file
;LicenseFile=LICENSE
; Require administrator rights (the default). Setup requests elevation (UAC) at
; startup when not already elevated, and runs straight through when launched via
; "Run as administrator" — no "Select Setup Install Mode" dialog is shown. Both
; Standard and Portable installs run elevated; the installed app itself never
; needs administrator rights at runtime (data lives in the user profile for
; Standard, or the sibling data folder for Portable).
PrivilegesRequired=admin
; Portable installs register no uninstaller (no Add/Remove Programs entry);
; Standard installs remain fully uninstallable. ShouldRunStandard is a [Code]
; Boolean function (= not IsPortable), evaluated after the mode page.
Uninstallable=ShouldRunStandard
OutputDir=.
OutputBaseFilename=Simsapa-Setup-{#AppVersion}
SetupIconFile=assets\icons\appicons\simsapa.ico
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Uninstaller should remove user data (app databases downloaded to user folder)
UninstallDisplayIcon={app}\{#AppExeName}
UninstallFilesDir={app}\uninst

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
; Standard-only: Portable mode creates its launcher in the parent folder and
; would otherwise show a desktop-icon checkbox that does nothing.
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked; Check: ShouldRunStandard

[Files]
; Main executable
Source: "{#DistDir}\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

; Qt libraries and dependencies (deployed by windeployqt)
Source: "{#DistDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "{#AppExeName}"

; Application icon
Source: "assets\icons\appicons\simsapa.ico"; DestDir: "{app}"; Flags: ignoreversion

; Visual C++ Redistributable (bundled for offline installation)
; Download from: https://aka.ms/vs/17/release/vc_redist.x64.exe
; Place in redist\ folder before building installer
Source: "redist\vc_redist.x64.exe"; DestDir: "{tmp}"; Flags: ignoreversion deleteafterinstall; Check: VCRedistNeedsInstall

[Icons]
; Standard-only icons. Portable mode creates its launcher in the parent folder
; (see [Code]) and registers no uninstaller, so these are gated to Standard.
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\simsapa.ico"; Check: ShouldRunStandard
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"; Check: ShouldRunStandard
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\simsapa.ico"; Tasks: desktopicon; Check: ShouldRunStandard

[Run]
; Install Visual C++ Redistributable silently if needed (runs before app launch)
Filename: "{tmp}\vc_redist.x64.exe"; Parameters: "/install /quiet /norestart"; StatusMsg: "Installing Visual C++ Redistributable..."; Flags: waituntilterminated; Check: VCRedistNeedsInstall
; Launch application after installation
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
var
  ShouldDeleteUserData: Boolean;
  // Install-mode selection page (Standard vs Portable), shown after Welcome.
  // Built from custom controls so the option titles can be shown in bold.
  ModePage: TWizardPage;
  StandardRadio: TNewRadioButton;
  PortableRadio: TNewRadioButton;
  // True when the user chose Portable install on ModePage.
  IsPortable: Boolean;
  // Launcher-type page (Portable only): .lnk shortcut vs .cmd relative launcher.
  LauncherPage: TInputOptionWizardPage;
  // True when the user chose the .cmd launcher; False for the .lnk shortcut.
  LauncherIsCmd: Boolean;

// Helper used by [Files]/[Icons]/[Run] Check: parameters to gate entries on the
// chosen install mode. Returns True only for a Portable install.
function ShouldRunPortable: Boolean;
begin
  Result := IsPortable;
end;

// Inverse of ShouldRunPortable, for entries that apply only to Standard install.
function ShouldRunStandard: Boolean;
begin
  Result := not IsPortable;
end;

// The portable data folder: a sibling of the install ({app}) folder, named by
// appending 'Data' to the install folder name (e.g. ...\Simsapa -> ...\SimsapaData).
function GetPortableDataDir: String;
var
  AppDir: String;
begin
  AppDir := ExpandConstant('{app}');
  Result := ExtractFileDir(AppDir) + '\' + ExtractFileName(AppDir) + 'Data';
end;

// Check if Visual C++ Redistributable is installed
function VCRedistNeedsInstall: Boolean;
var
  Version: String;
begin
  // Check for Visual C++ 2015-2022 Redistributable (x64)
  // The registry key varies by version, but we check for the unified runtime
  if RegQueryStringValue(HKLM64, 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64', 'Version', Version) then
    Result := False
  else if RegQueryStringValue(HKLM, 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64', 'Version', Version) then
    Result := False
  else
    Result := True;
end;

// Check if VC++ Redistributable installer is bundled
function VCRedistBundled: Boolean;
begin
  Result := FileExists(ExpandConstant('{src}\redist\vc_redist.x64.exe'));
end;

// Add one install-mode option to ModePage: a bold radio-button title followed
// by a wrapped, normal-weight description. Advances NextTop past the block so
// the next option stacks below it.
procedure AddModeOption(var Radio: TNewRadioButton; var NextTop: Integer; Title, Desc: String);
var
  DescLabel: TNewStaticText;
begin
  Radio := TNewRadioButton.Create(ModePage);
  Radio.Parent := ModePage.Surface;
  Radio.Caption := Title;
  Radio.Font.Style := [fsBold];
  Radio.Left := 0;
  Radio.Top := NextTop;
  Radio.Width := ModePage.SurfaceWidth;
  Radio.Height := ScaleY(17);

  DescLabel := TNewStaticText.Create(ModePage);
  DescLabel.Parent := ModePage.Surface;
  DescLabel.AutoSize := True;
  DescLabel.WordWrap := True;
  DescLabel.Left := ScaleX(16);
  DescLabel.Top := Radio.Top + Radio.Height + ScaleY(2);
  DescLabel.Width := ModePage.SurfaceWidth - ScaleX(16);
  // Caption set last so the wrapped height is computed against the final Width.
  DescLabel.Caption := Desc;

  NextTop := DescLabel.Top + DescLabel.Height + ScaleY(16);
end;

// Initialize the wizard with custom pages
procedure InitializeWizard;
var
  WarningPage: TOutputMsgWizardPage;
  NextTop: Integer;
begin
  // Install-type selection page, shown early (after Welcome, before the
  // directory page). Built from custom controls so the option titles are bold.
  // Standard is ALWAYS the default so users who do not expect portable mode are
  // not surprised; the existing behavior is unchanged unless the user explicitly
  // opts into Portable.
  ModePage := CreateCustomPage(wpWelcome,
    'Installation Type',
    'Choose how Simsapa should be installed.');

  NextTop := ScaleY(8);
  AddModeOption(StandardRadio, NextTop,
    'Standard Install (recommended)',
    'Installs to Program Files for all users (requires administrator rights to ' +
    'install). Databases and settings are stored in your user profile. The app ' +
    'does not need administrator rights once installed.');
  AddModeOption(PortableRadio, NextTop,
    'Portable Install',
    'Installs into any folder you choose (e.g. your Desktop or a USB drive) and ' +
    'keeps all data in a folder next to the app, so it can travel with you. No ' +
    'administrator rights required - choose "Install for me only" if prompted at ' +
    'startup.');

  // Standard is the unconditional default.
  StandardRadio.Checked := True;
  IsPortable := False;

  // Launcher-type page (shown after the directory page, Portable only via
  // ShouldSkipPage). The launcher is placed in the parent of the install
  // folder (e.g. the Desktop or USB root) and starts the installed exe.
  LauncherPage := CreateInputOptionPage(wpSelectDir,
    'Portable Launcher',
    'Choose how to create the launcher next to your install folder.',
    'The launcher is placed in the parent folder (e.g. your Desktop or the USB ' +
    'drive root) and starts Simsapa. Select a type, then click Next.',
    True {Exclusive radio buttons}, False {not a list box});
  LauncherPage.Add(
    'Shortcut (.lnk) - simplest, uses the app icon. May stop working if a USB ' +
    'drive is given a different drive letter on another computer.');
  LauncherPage.Add(
    'Batch launcher (.cmd) - recommended for USB drives. Resolves the app ' +
    'relative to its own location, so it keeps working when the drive letter ' +
    'changes on another computer.');
  LauncherPage.SelectedValueIndex := 0;
  LauncherIsCmd := False;

  // Add a warning page if VC++ Redistributable is needed but NOT bundled
  // If bundled, it will be installed silently - no warning needed
  // Placed after ModePage so the wizard order is Welcome -> Mode -> (VC warning).
  if VCRedistNeedsInstall and not VCRedistBundled then
  begin
    WarningPage := CreateOutputMsgPage(ModePage.ID,
      'Visual C++ Redistributable Required',
      'Additional runtime required',
      'This application requires the Microsoft Visual C++ Redistributable to run.' + #13#10#13#10 +
      'If you encounter errors starting the application after installation, ' +
      'please download and install the Visual C++ Redistributable from:' + #13#10#13#10 +
      'https://aka.ms/vs/17/release/vc_redist.x64.exe' + #13#10#13#10 +
      'Click Next to continue with the installation.');
  end;
end;

// Capture the install-mode choice when leaving the mode page.
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if CurPageID = ModePage.ID then
    IsPortable := PortableRadio.Checked;
  if CurPageID = LauncherPage.ID then
    LauncherIsCmd := (LauncherPage.SelectedValueIndex = 1);
end;

// Decide which wizard pages to skip per mode:
// - The launcher-type page only applies to Portable installs.
// - In Portable mode the "Select Start Menu Folder" page is meaningless (no
//   group icons are created, and the launcher goes in the parent folder), so
//   skip it to avoid confusing the user.
function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := False;
  if PageID = LauncherPage.ID then
    Result := not IsPortable;
  if PageID = wpSelectProgramGroup then
    Result := IsPortable;
end;

// When entering the directory page, suggest a sensible default per mode:
// Portable defaults to a user-writable Desktop folder (no admin needed);
// Standard keeps the Program Files default.
procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpSelectDir then
  begin
    if IsPortable then
      WizardForm.DirEdit.Text := ExpandConstant('{userdesktop}\Simsapa')
    else
      WizardForm.DirEdit.Text := ExpandConstant('{autopf}\Simsapa');
  end;
end;

// After files are installed, for Portable mode: create the sibling data folder
// (reusing it as-is if it already exists, preserving any downloaded databases)
// and write config.txt next to the exe pointing SIMSAPA_DIR at that folder via
// a relative, unquoted, forward-slash path (e.g. SIMSAPA_DIR=../SimsapaData).
procedure CurStepChanged(CurStep: TSetupStep);
var
  AppDir: String;
  DataDir: String;
  ParentDir: String;
  BaseName: String;
  ConfigPath: String;
  ConfigContent: String;
  CmdContent: String;
  LinkResult: String;
begin
  if (CurStep = ssPostInstall) and IsPortable then
  begin
    AppDir := ExpandConstant('{app}');
    ParentDir := ExtractFileDir(AppDir);
    BaseName := ExtractFileName(AppDir);

    DataDir := GetPortableDataDir;
    // Reuse an existing data folder as-is; only create it when absent.
    if not DirExists(DataDir) then
      CreateDir(DataDir);

    // Relative path must match the sibling folder name and use forward slashes
    // (Rust accepts '/' on Windows; dotenvy treats '\' as an escape char).
    ConfigContent := 'SIMSAPA_DIR=../' + BaseName + 'Data';
    ConfigPath := AppDir + '\config.txt';
    SaveStringToFile(ConfigPath, ConfigContent + #13#10, False);

    // Create the launcher in the parent folder. Neither launcher relies on a
    // "Start in"/CWD value: the app locates config.txt via the exe's own
    // directory, so the launcher only needs to start the exe.
    if LauncherIsCmd then
    begin
      // %~dp0 expands to the launcher's own drive+path (with trailing '\'),
      // so the exe is resolved relative to the .cmd's location -> survives
      // USB drive-letter changes. The install subfolder name is BaseName.
      CmdContent :=
        '@echo off' + #13#10 +
        'start "" "%~dp0' + BaseName + '\{#AppExeName}"' + #13#10;
      SaveStringToFile(ParentDir + '\' + BaseName + '.cmd', CmdContent, False);
    end
    else
    begin
      // Standard Windows shortcut pointing at the installed exe, using the app
      // icon. Working directory left empty (config discovery is exe-relative).
      LinkResult := CreateShellLink(
        ParentDir + '\' + BaseName + '.lnk',
        '{#AppName}',
        AppDir + '\{#AppExeName}',
        '', '',
        AppDir + '\simsapa.ico',
        0, SW_SHOWNORMAL);
    end;
  end;
end;

// Get the user data directory where app databases are stored
// Uses app_dirs2 crate convention: AppInfo{name: "simsapa-ng", author: "profound-labs"}
// From backend/src/lib.rs:45: APP_INFO: AppInfo = AppInfo{name: "simsapa-ng", author: "profound-labs"}
// From backend/src/lib.rs:274: get_app_root(AppDataType::UserData, &APP_INFO)
// On Windows, app_dirs2 creates: %LOCALAPPDATA%\{author}\{name}
// Result: %LOCALAPPDATA%\profound-labs\simsapa-ng
// This directory contains:
//   - app-assets/ (appdata.sqlite3, dictionaries, downloaded language databases)
//   - logs/ (application logs)
function GetUserDataDir: String;
begin
  Result := ExpandConstant('{localappdata}\profound-labs\simsapa-ng');
end;

// Called before uninstall begins - ask user about deleting user data
function InitializeUninstall(): Boolean;
var
  UserDataDir: String;
  MsgResult: Integer;
begin
  Result := True;
  ShouldDeleteUserData := False;
  
  if not UninstallSilent then
  begin
    UserDataDir := GetUserDataDir;
    
    if DirExists(UserDataDir) then
    begin
      MsgResult := MsgBox(
        'Simsapa stores downloaded language databases, user settings, and annotations in your user data folder.' + #13#10#13#10 +
        'Location: ' + UserDataDir + #13#10#13#10 +
        'Do you want to delete this data as well?' + #13#10#13#10 +
        'Click Yes to remove the Simsapa app and all user data.' + #13#10 +
        'Click No to remove the Simsapa app but keep user data (you can reinstall the app later without re-downloading).' + #13#10 +
        'Click Cancel to exit and abort uninstallation.',
        mbConfirmation, MB_YESNOCANCEL);
      
      case MsgResult of
        IDYES: ShouldDeleteUserData := True;
        IDNO: ShouldDeleteUserData := False;
        IDCANCEL: Result := False;  // Abort uninstall
      end;
    end;
  end;
end;

// Custom uninstall process to remove user data if requested
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  UserDataDir: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Perform deletion if user requested it
    if ShouldDeleteUserData then
    begin
      UserDataDir := GetUserDataDir;
      if DirExists(UserDataDir) then
      begin
        if DelTree(UserDataDir, True, True, True) then
        begin
          if not UninstallSilent then
            MsgBox('User data has been successfully deleted.', mbInformation, MB_OK);
        end
        else
        begin
          if not UninstallSilent then
            MsgBox('Could not delete all user data. Some files may still remain at:' + #13#10#13#10 +
                   UserDataDir + #13#10#13#10 +
                   'You may need to delete them manually.',
                   mbError, MB_OK);
        end;
      end;
    end;
  end;
end;
