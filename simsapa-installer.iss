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
; Uncomment the following line to run in non-administrative install mode
; (install for current user only.)
;PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
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
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

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
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\simsapa.ico"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\simsapa.ico"; Tasks: desktopicon

[Run]
; Install Visual C++ Redistributable silently if needed (runs before app launch)
Filename: "{tmp}\vc_redist.x64.exe"; Parameters: "/install /quiet /norestart"; StatusMsg: "Installing Visual C++ Redistributable..."; Flags: waituntilterminated; Check: VCRedistNeedsInstall
; Launch application after installation
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
var
  ShouldDeleteUserData: Boolean;

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

// Initialize the wizard with custom pages
procedure InitializeWizard;
var
  WarningPage: TOutputMsgWizardPage;
begin
  // Add a warning page if VC++ Redistributable is needed but NOT bundled
  // If bundled, it will be installed silently - no warning needed
  if VCRedistNeedsInstall and not VCRedistBundled then
  begin
    WarningPage := CreateOutputMsgPage(wpWelcome,
      'Visual C++ Redistributable Required',
      'Additional runtime required',
      'This application requires the Microsoft Visual C++ Redistributable to run.' + #13#10#13#10 +
      'If you encounter errors starting the application after installation, ' +
      'please download and install the Visual C++ Redistributable from:' + #13#10#13#10 +
      'https://aka.ms/vs/17/release/vc_redist.x64.exe' + #13#10#13#10 +
      'Click Next to continue with the installation.');
  end;
end;

// Get the user data directory where app databases are stored
// Uses app_dirs2 crate convention: AppInfo{name: "simsapa-ng", author: "profound-labs"}
// From backend/src/lib.rs:45: APP_INFO: AppInfo = AppInfo{name: "simsapa-ng", author: "profound-labs"}
// From backend/src/lib.rs:274: get_app_root(AppDataType::UserData, &APP_INFO)
// On Windows, app_dirs2 creates: %LOCALAPPDATA%\{author}\{name}
// Result: %LOCALAPPDATA%\profound-labs\simsapa-ng
// This directory contains:
//   - userdata.sqlite3 (user's personal database)
//   - app-assets/ (downloaded language databases)
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
