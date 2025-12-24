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

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\simsapa.ico"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\simsapa.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
var
  UninstallUserDataPage: TInputOptionWizardPage;
  DeleteUserDataCheckbox: TNewCheckBox;

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

// Initialize the wizard with custom pages
procedure InitializeWizard;
var
  WarningPage: TOutputMsgWizardPage;
begin
  // Add a warning page if VC++ Redistributable is needed
  if VCRedistNeedsInstall then
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

// Initialize the uninstall wizard with custom page
procedure InitializeUninstallProgressForm();
var
  UninstallConfirmPage: TNewNotebookPage;
  PageText: TNewStaticText;
  PathLabel: TLabel;
  UserDataDir: String;
begin
  if not UninstallSilent then
  begin
    // Create a custom page in the uninstall wizard
    UninstallConfirmPage := TNewNotebookPage.Create(UninstallProgressForm);
    UninstallConfirmPage.Notebook := UninstallProgressForm.InnerNotebook;
    UninstallConfirmPage.Parent := UninstallProgressForm.InnerNotebook;
    UninstallConfirmPage.Align := alClient;

    // Title text
    PageText := TNewStaticText.Create(UninstallProgressForm);
    PageText.Parent := UninstallConfirmPage;
    PageText.Top := ScaleY(16);
    PageText.Left := ScaleX(0);
    PageText.Width := UninstallConfirmPage.Width;
    PageText.AutoSize := False;
    PageText.ShowAccelChar := False;
    PageText.Font.Style := [fsBold];
    PageText.Caption := 'Remove Downloaded Data and User Settings?';

    // Description text
    PageText := TNewStaticText.Create(UninstallProgressForm);
    PageText.Parent := UninstallConfirmPage;
    PageText.Top := ScaleY(48);
    PageText.Left := ScaleX(0);
    PageText.Width := UninstallConfirmPage.Width - ScaleX(16);
    PageText.Height := ScaleY(60);
    PageText.AutoSize := False;
    PageText.ShowAccelChar := False;
    PageText.Caption := 
      'Simsapa stores downloaded language databases, user settings, and annotations in your user data folder.' + #13#10#13#10 +
      'Would you like to remove this data as well?';

    // Path label
    UserDataDir := GetUserDataDir;
    PathLabel := TLabel.Create(UninstallProgressForm);
    PathLabel.Parent := UninstallConfirmPage;
    PathLabel.Top := ScaleY(118);
    PathLabel.Left := ScaleX(0);
    PathLabel.Width := UninstallConfirmPage.Width - ScaleX(16);
    PathLabel.AutoSize := False;
    PathLabel.Caption := 'Location: ' + UserDataDir;
    PathLabel.Font.Color := clGrayText;

    // Checkbox for deletion
    DeleteUserDataCheckbox := TNewCheckBox.Create(UninstallProgressForm);
    DeleteUserDataCheckbox.Parent := UninstallConfirmPage;
    DeleteUserDataCheckbox.Top := ScaleY(148);
    DeleteUserDataCheckbox.Left := ScaleX(0);
    DeleteUserDataCheckbox.Width := UninstallConfirmPage.Width - ScaleX(16);
    DeleteUserDataCheckbox.Height := ScaleY(20);
    DeleteUserDataCheckbox.Caption := 'Yes, delete all downloaded databases, settings, and user data';
    DeleteUserDataCheckbox.Checked := True;  // Enabled by default

    // Set this as the initial page
    UninstallProgressForm.InnerNotebook.ActivePage := UninstallConfirmPage;
    
    // Update button labels for the first page
    UninstallProgressForm.StatusLabel.Caption := 'Click Next to continue, or Cancel to exit.';
  end;
end;

// Handle the Next button click in uninstall wizard
function UninstallNeedRestart(): Boolean;
begin
  Result := False;
end;

// Custom uninstall process to remove user data if requested
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  UserDataDir: String;
  ShouldDelete: Boolean;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    UserDataDir := GetUserDataDir;
    ShouldDelete := False;
    
    // Determine if we should delete user data
    if UninstallSilent then
    begin
      // During silent uninstall, don't delete user data by default
      ShouldDelete := False;
    end
    else
    begin
      // During interactive uninstall, check if user opted to delete
      if Assigned(DeleteUserDataCheckbox) and DeleteUserDataCheckbox.Checked then
      begin
        ShouldDelete := True;
      end;
    end;
    
    // Perform deletion if requested
    if ShouldDelete and DirExists(UserDataDir) then
    begin
      // Delete the user data directory
      if DelTree(UserDataDir, True, True, True) then
      begin
        if not UninstallSilent then
          MsgBox('User data has been successfully deleted from:' + #13#10#13#10 + UserDataDir, mbInformation, MB_OK);
      end
      else
      begin
        if not UninstallSilent then
          MsgBox('Could not delete all user data. Some files may still remain at:' + #13#10 +
                 UserDataDir + #13#10#13#10 +
                 'You may need to delete them manually.',
                 mbError, MB_OK);
      end;
    end;
  end;
end;
