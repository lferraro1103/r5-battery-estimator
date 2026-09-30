; R5 Battery Estimator V2 — per-user Windows installer.
; Build after `cargo build --release --bin r5-battery-estimator-v2` and staging
; outputs\R5BatteryEstimatorV2\.

#define AppName "R5 Battery Estimator"
#define AppVersion "2.0.0"
#define AppPublisher "Leaan Ferraro"
#define AppExeName "R5BatteryEstimatorV2.exe"

[Setup]
AppId={{A6EB84CC-7C55-4CCD-8B83-84921A968E2E}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL=https://github.com/lferraro1103/r5-battery-estimator
AppSupportURL=https://github.com/lferraro1103/r5-battery-estimator/issues
DefaultDirName={localappdata}\Programs\R5 Battery Estimator
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\outputs\installer
OutputBaseFilename=R5BatteryEstimatorV2-Setup
SetupIconFile=..\src-tauri\icons\icon.ico
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Files]
Source: "..\outputs\R5BatteryEstimatorV2\R5BatteryEstimatorV2.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\outputs\R5BatteryEstimatorV2\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\outputs\R5BatteryEstimatorV2\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\outputs\R5BatteryEstimatorV2\Assets\shark-battery.png"; DestDir: "{app}\Assets"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; GroupDescription: "Accesos directos adicionales:"; Flags: unchecked

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Abrir {#AppName}"; Flags: nowait postinstall skipifsilent
