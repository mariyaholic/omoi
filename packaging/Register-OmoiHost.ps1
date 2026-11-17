# Explicit per-user registration. Never edits Edge profiles or extension policy.
[CmdletBinding()]
param(
    [ValidateSet('Install','Remove')][string]$Action='Install',
    [string]$ExtensionId,
    [string]$CatalogueHome=(Join-Path $env:LOCALAPPDATA 'omoi'),
    [ValidateSet('development','release')][string]$Channel='development',
    [string]$RegistryRoot='HKCU:\Software\Microsoft\Edge\NativeMessagingHosts'
)
$ErrorActionPreference='Stop'
if ($RegistryRoot -ne 'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts' -and $RegistryRoot -notmatch '^HKCU:\\Software\\OmoiTests\\[a-zA-Z0-9-]+$') { throw 'Unsupported registry root' }
$InstallRoot=[IO.Path]::GetFullPath($PSScriptRoot)
$name=if($Channel -eq 'development'){'org.omoi.capture_dev'}else{'org.omoi.capture'}
$key=Join-Path $RegistryRoot $name
$manifest=Join-Path $InstallRoot "$name.json"
$marker=Join-Path $InstallRoot "$name.owner.json"
if ($Action -eq 'Remove') {
    if(Test-Path $key){
        $registered=(Get-Item $key).GetValue('')
        if($registered -ne $manifest){throw 'Registration belongs to another installation; nothing removed'}
        if(!(Test-Path $marker)){throw 'Ownership marker missing; nothing removed'}
        $owned=Get-Content -Raw $marker | ConvertFrom-Json
        if($owned.name -ne $name -or $owned.manifest -ne $manifest){throw 'Ownership marker mismatch; nothing removed'}
        Remove-Item -LiteralPath $key
    }
    # Keep host configuration and all catalogue/settings data. Reinstall is reversible.
    Write-Output 'Owned host registration removed; catalogue and settings preserved.'
    exit 0
}
if($ExtensionId -notmatch '^[a-p]{32}$'){throw 'Supply the exact 32-letter Edge extension ID; no wildcard IDs are accepted'}
if(![IO.Path]::IsPathRooted($CatalogueHome)){throw 'CatalogueHome must be absolute'}
$CatalogueHome=[IO.Path]::GetFullPath($CatalogueHome)
$hostExe=Join-Path $InstallRoot 'omoi-native.exe'
if(!(Test-Path -LiteralPath $hostExe -PathType Leaf)){throw 'omoi-native.exe is missing. Repair the application package first.'}
if(Test-Path $key){
    $prior=(Get-Item $key).GetValue('')
    if($prior -ne $manifest){
        # Repair a moved installation only with a readable Omoi-owned marker and
        # the same home/channel. An unrelated or unverifiable key is never replaced.
        $priorMarker=Join-Path (Split-Path $prior) "$name.owner.json"
        if(!(Test-Path $priorMarker)){throw 'Existing host ownership cannot be verified. Remove it from its original installer first.'}
        $owned=Get-Content -Raw $priorMarker | ConvertFrom-Json
        if($owned.name -ne $name -or $owned.manifest -ne $prior -or $owned.home -ne $CatalogueHome){throw 'Existing host belongs to a different catalogue or installer; nothing changed'}
    }
}
# Per-installation host config binds the process to one explicit catalogue.
# Switching catalogue requires a separate install directory and explicit registration.
$config=Join-Path $InstallRoot 'omoi-host.json'
if(Test-Path $config){$old=Get-Content -Raw $config | ConvertFrom-Json;if($old.home -ne $CatalogueHome){throw 'This installation is bound to a different catalogue. Use a separate install directory.'}}
$origin="chrome-extension://$ExtensionId/"
function Write-AtomicJson($path,$value){
    $temp="$path.$([guid]::NewGuid().ToString('N')).tmp"
    try{[IO.File]::WriteAllText($temp,($value|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false));Move-Item -LiteralPath $temp -Destination $path -Force}finally{if(Test-Path $temp){Remove-Item -LiteralPath $temp}}
}
Write-AtomicJson $config @{home=$CatalogueHome;allowed_origins=@($origin)}
Write-AtomicJson $manifest @{name=$name;description='Omoi local playback capture';path=$hostExe;type='stdio';allowed_origins=@($origin)}
Write-AtomicJson $marker @{name=$name;manifest=$manifest;home=$CatalogueHome}
New-Item -Path $key -Force | Out-Null
Set-Item -Path $key -Value $manifest
Write-Output 'Native host registered for the exact approved extension and catalogue. Approve site access in Edge, then Connect.'
