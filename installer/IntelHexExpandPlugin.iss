#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "IntelHexExpandPlugin"
#define MyAppPublisher "mogwai-dev"
#define MyAppURL "https://github.com/mogwai-dev/IntelHexExpandPlugin"

[Setup]
AppId={{B353EA7D-1167-4EAA-BFB0-1B3026993EF0}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\{#MyAppName}
DisableDirPage=yes
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=IntelHexExpandPlugin-x64-setup
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
CloseApplications=yes
RestartApplications=no
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={userappdata}\WinMerge\MergePlugins\IntelHexExpand.dll
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription=WinMerge plugin for expanding Intel HEX files
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
WizardStyle=modern

[Files]
Source: "..\dist\IntelHexExpand.dll"; DestDir: "{userappdata}\WinMerge\MergePlugins"; Flags: ignoreversion
