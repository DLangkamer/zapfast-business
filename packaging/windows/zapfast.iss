; Windows installer built from a release binary with Inno Setup 6.3 or newer:
;
;   iscc /DVersion=0.1.0 /DArch=x86_64 /DBinary=...\zapfast-business.exe ^
;        /DOutputDir=dist packaging\windows\zapfast.iss
;
; Arch matches the Rust target: x86_64 or aarch64. Installation uses the
; current user's Programs folder and does not need administrator rights.
; Updates close a running copy before replacing it.

#ifndef Version
  #error Version must be defined on the ISCC command line
#endif
#ifndef Arch
  #error Arch must be defined on the ISCC command line (x86_64 or aarch64)
#endif
#ifndef NumericVersion
  #define NumericVersion Version
#endif
#ifndef Binary
  #error Binary must be defined on the ISCC command line
#endif
#ifndef OutputDir
  #error OutputDir must be defined on the ISCC command line
#endif
#if Arch == "aarch64"
  #define InnoArch "arm64"
#else
  #define InnoArch "x64compatible"
#endif

#define AppName "ZapFast Business"
#define AppExeName "zapfast-business.exe"

[Setup]
; Never change: this is how Windows tells an update from a new program.
AppId={{B52DF836-982D-48F8-91B0-FE4D9E58B7F2}
AppName={#AppName}
AppVersion={#Version}
AppVerName={#AppName} {#Version}
AppPublisher=DLangkamer (fork of Carmine Paolino's ZapFast)
AppPublisherURL=https://zapfast.rocks
AppSupportURL=https://github.com/DLangkamer/zapfast-business
AppUpdatesURL=https://github.com/DLangkamer/zapfast-business
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed={#InnoArch}
ArchitecturesInstallIn64BitMode={#InnoArch}
MinVersion=10.0
LicenseFile=..\..\LICENSE
OutputDir={#OutputDir}
OutputBaseFilename=zapfast-business-v{#Version}-{#Arch}-pc-windows-msvc-setup
SetupIconFile=zapfast.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
CloseApplicationsFilter=zapfast-business.exe
UninstallDisplayIcon={app}\{#AppExeName}
VersionInfoVersion={#NumericVersion}.0

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "{#Binary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\BUSINESS.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\THIRD-PARTY-NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\fonts\Inter-LICENSE.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\..\assets\fonts\NotoColorEmoji-LICENSE.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\..\assets\icons\LICENSE.txt"; DestDir: "{app}\licenses"; DestName: "Lucide-LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; AppUserModelID: "io.github.DLangkamer.ZapFastBusiness"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon; AppUserModelID: "io.github.DLangkamer.ZapFastBusiness"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\io.github.DLangkamer.ZapFastBusiness"; ValueType: string; ValueName: "DisplayName"; ValueData: "ZapFast Business"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "ZapFast Business"; Flags: dontcreatekey uninsdeletevalue

[Code]
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if FileExists(ExpandConstant('{app}\zapfast.exe')) or
     FileExists(ExpandConstant('{app}\fastsapp.exe')) or
     FileExists(ExpandConstant('{app}\fastwhatsapp.exe')) then
    Result := 'Choose a separate folder for ZapFast Business. This folder contains another client.';
end;
