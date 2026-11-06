; Compile from the repository root: ISCC /DBinDir=... packaging\omoi.iss
#ifndef BinDir
  #define BinDir "..\target\release"
#endif
[Setup]
AppId={{FA25299C-2530-48CB-A2C3-4C86047E87F1}
AppName=Omoi
AppVersion=0.2.0
DefaultDirName={localappdata}\Programs\Omoi
DefaultGroupName=Omoi
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputBaseFilename=Omoi-Setup-unsigned
OutputDir=..\dist
UninstallDisplayIcon={app}\Omoi.ico
CloseApplications=yes
RestartApplications=no
Compression=lzma2
SolidCompression=yes
[Files]
Source: "{#BinDir}\omoi.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\omoiw.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\omoi-desktop.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\omoi-native.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\WebView2Loader.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icon.ico"; DestName: "Omoi.ico"; DestDir: "{app}"
Source: "Register-OmoiHost.ps1"; DestDir: "{app}"
Source: "Remove-OmoiTask.ps1"; DestDir: "{app}"
Source: "..\docs\SETUP.md"; DestName: "SETUP.md"; DestDir: "{app}"
Source: "..\userscripts\omoi.user.js"; DestDir: "{app}"
Source: "..\extension\*"; DestDir: "{app}\extension"; Flags: recursesubdirs
[Icons]
Name: "{group}\Omoi"; Filename: "{app}\omoi-desktop.exe"; IconFilename: "{app}\Omoi.ico"
Name: "{group}\Setup instructions"; Filename: "{app}\SETUP.md"
[Run]
Filename: "{app}\omoi-desktop.exe"; Description: "Open Omoi"; Flags: postinstall nowait skipifsilent
[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File ""{app}\Register-OmoiHost.ps1"" -Action Remove"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveOwnedOmoiDevHost"
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File ""{app}\Remove-OmoiTask.ps1"""; Flags: runhidden waituntilterminated; RunOnceId: "RemoveOwnedOmoiTask"
[Code]
var HostPage: TInputQueryWizardPage;
procedure InitializeWizard;
begin
  HostPage := CreateInputQueryPage(wpSelectDir, 'Optional Edge companion', 'Register the local native host', 'For a development install, enter the exact ID shown for the unpacked Omoi companion in Edge. Leave blank for history-only use. This does not install an extension or grant browser permissions.');
  HostPage.Add('Development extension ID:', False);
end;
function NextButtonClick(CurPageID: Integer): Boolean;
var Id: String; I: Integer;
begin
  Result := True;
  if CurPageID = HostPage.ID then begin
    Id := HostPage.Values[0];
    if Id = '' then exit;
    Result := Length(Id) = 32;
    for I := 1 to Length(Id) do
      if (Id[I] < 'a') or (Id[I] > 'p') then Result := False;
    if not Result then MsgBox('Enter the exact 32-letter extension ID, or leave blank.', mbError, MB_OK);
  end;
end;
procedure CurStepChanged(CurStep: TSetupStep);
var Code: Integer; Params: String;
begin
  if (CurStep = ssPostInstall) and (HostPage.Values[0] <> '') then begin
    Params := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + ExpandConstant('{app}\Register-OmoiHost.ps1') + '" -ExtensionId ' + HostPage.Values[0];
    if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Params, '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
      MsgBox('Host registration needs repair. Open SETUP.md in the install folder. History remains available.', mbError, MB_OK);
  end;
end;
