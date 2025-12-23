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
  UserDataDirPage: TInputDirWizardPage;
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

// Initialize the uninstall wizard
procedure InitializeUninstallProgressForm();
var
  PageText: TNewStaticText;
  PagePanel: TPanel;
  BevelTop: TBevel;
begin
  if not UninstallSilent then
  begin
    // Create a custom page for uninstall options
    PagePanel := TPanel.Create(UninstallProgressForm);
    PagePanel.Parent := UninstallProgressForm;
    PagePanel.Left := 0;
    PagePanel.Top := UninstallProgressForm.OuterNotebook.Top + UninstallProgressForm.OuterNotebook.Height + ScaleY(20);
    PagePanel.Width := UninstallProgressForm.ClientWidth;
    PagePanel.Height := ScaleY(80);
    PagePanel.Anchors := [akLeft, akTop, akRight];
    PagePanel.BevelOuter := bvNone;

    BevelTop := TBevel.Create(UninstallProgressForm);
    BevelTop.Parent := PagePanel;
    BevelTop.Left := 0;
    BevelTop.Top := 0;
    BevelTop.Width := PagePanel.Width;
    BevelTop.Height := ScaleY(2);
    BevelTop.Anchors := [akLeft, akTop, akRight];
    BevelTop.Shape := bsTopLine;

    PageText := TNewStaticText.Create(UninstallProgressForm);
    PageText.Parent := PagePanel;
    PageText.Left := ScaleX(10);
    PageText.Top := ScaleY(15);
    PageText.Width := PagePanel.Width - ScaleX(20);
    PageText.Height := ScaleY(30);
    PageText.Anchors := [akLeft, akTop, akRight];
    PageText.Caption := 'User data and downloaded databases:';
    PageText.WordWrap := False;

    DeleteUserDataCheckbox := TNewCheckBox.Create(UninstallProgressForm);
    DeleteUserDataCheckbox.Parent := PagePanel;
    DeleteUserDataCheckbox.Left := ScaleX(10);
    DeleteUserDataCheckbox.Top := PageText.Top + PageText.Height + ScaleY(5);
    DeleteUserDataCheckbox.Width := PagePanel.Width - ScaleX(20);
    DeleteUserDataCheckbox.Height := ScaleY(25);
    DeleteUserDataCheckbox.Anchors := [akLeft, akTop, akRight];
    DeleteUserDataCheckbox.Caption := 'Delete user data and downloaded databases at: ' + GetUserDataDir();
    DeleteUserDataCheckbox.Checked := False;
    DeleteUserDataCheckbox.WordWrap := True;

    // Adjust form height to accommodate the new panel
    UninstallProgressForm.ClientHeight := UninstallProgressForm.ClientHeight + PagePanel.Height;
  end;
end;

// Custom uninstall process to remove user data if requested
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  UserDataDir: String;
  ResultCode: Integer;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Check if user wants to delete their data
    if Assigned(DeleteUserDataCheckbox) and DeleteUserDataCheckbox.Checked then
    begin
      UserDataDir := GetUserDataDir;
      if DirExists(UserDataDir) then
      begin
        if MsgBox('This will permanently delete all your downloaded databases and user data at:' + #13#10#13#10 +
                  UserDataDir + #13#10#13#10 +
                  'Are you sure you want to continue?',
                  mbConfirmation, MB_YESNO) = IDYES then
        begin
          // Delete the user data directory
          if DelTree(UserDataDir, True, True, True) then
          begin
            MsgBox('User data has been successfully deleted.', mbInformation, MB_OK);
          end
          else
          begin
            MsgBox('Could not delete all user data. Some files may still remain at:' + #13#10 +
                   UserDataDir + #13#10#13#10 +
                   'You may need to delete them manually.',
                   mbError, MB_OK);
          end;
        end;
      end
      else
      begin
        MsgBox('User data directory not found. Nothing to delete.', mbInformation, MB_OK);
      end;
    end;
  end;
end;
