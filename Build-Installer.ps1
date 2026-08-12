param(
    [string]$Configuration = "Release",
    [string]$Platform = "x64",
    [string]$MsBuildPath = "",
    [string]$VcInstallPath = "",
    [string]$InnoCompilerPath = "",
    [string]$InstallerSigningPfxPath = "",
    [string]$InstallerSigningPfxPassword = "",
    [switch]$DisableSigning
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceRoot = Split-Path -Parent $Root
$ManifestPath = Join-Path $Root "Package\Package.appxmanifest"
$InstallerScript = Join-Path $Root "Installer\KillConfirmGameBar.iss"

if (-not (Test-Path $ManifestPath)) {
    throw "Package.appxmanifest was not found at $ManifestPath"
}

if (-not (Test-Path $InstallerScript)) {
    throw "Installer script was not found at $InstallerScript"
}

[xml]$Manifest = Get-Content $ManifestPath
$Version = $Manifest.Package.Identity.Version
if (-not $Version) {
    throw "Could not read package version from $ManifestPath"
}

$TransferRoot = Join-Path $WorkspaceRoot ("KillConfirmGameBar_Transfer_{0}" -f $Version)

$buildTransferArgs = @{
    Configuration = $Configuration
    Platform = $Platform
    MsBuildPath = $MsBuildPath
}
if ($VcInstallPath) {
    $buildTransferArgs.VcInstallPath = $VcInstallPath
}
if ($DisableSigning) {
    $buildTransferArgs.DisableSigning = $true
}

& (Join-Path $Root "Build-TransferPackage.ps1") @buildTransferArgs

if (-not (Test-Path $TransferRoot)) {
    throw "Expected transfer folder was not produced: $TransferRoot"
}

if (-not $InnoCompilerPath) {
    $Inno = Get-Command iscc -ErrorAction SilentlyContinue
    if ($Inno) {
        $InnoCompilerPath = $Inno.Source
    }
    else {
        $Candidates = @(
            (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
            (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
            (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe")
        )
        $InnoCompilerPath = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    }
}

if (-not $InnoCompilerPath -or -not (Test-Path $InnoCompilerPath)) {
    throw "Inno Setup compiler was not found. Install Inno Setup 6, then run this script again."
}

New-Item -ItemType Directory -Force -Path (Join-Path $Root "Output") | Out-Null

$innoArgs = @(
    ("/DMyAppVersion={0}" -f $Version),
    ("/DTransferRoot={0}" -f $TransferRoot)
)
$innoArgs += $InstallerScript

& $InnoCompilerPath @innoArgs

if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed with exit code $LASTEXITCODE"
}

$SetupPath = Join-Path $Root ("Output\KillConfirmGameBar_Setup_{0}.exe" -f $Version)
if (-not (Test-Path $SetupPath)) {
    throw "Expected installer was not produced: $SetupPath"
}

if (-not $DisableSigning) {
    if (-not $InstallerSigningPfxPath) {
        $InstallerSigningPfxPath = Join-Path $Root "Widget\KillConfirmGameBar_TemporaryKey.pfx"
    }
    if (-not $InstallerSigningPfxPassword) {
        $InstallerSigningPfxPassword = $env:KILLCONFIRM_SIGNING_PASSWORD
    }
    if (-not $InstallerSigningPfxPassword -and $InstallerSigningPfxPath -like "*KillConfirmGameBar_TemporaryKey.pfx") {
        $InstallerSigningPfxPassword = "test"
    }
    if (-not (Test-Path -LiteralPath $InstallerSigningPfxPath -PathType Leaf)) {
        throw "Installer signing PFX was not found: $InstallerSigningPfxPath"
    }

    $SignTool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter "signtool.exe" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $SignTool) {
        throw "signtool.exe was not found in the Windows SDK."
    }

    $signArgs = @("sign", "/fd", "SHA256", "/f", $InstallerSigningPfxPath)
    if ($InstallerSigningPfxPassword) {
        $signArgs += @("/p", $InstallerSigningPfxPassword)
    }
    $signArgs += @("/d", "Kill Confirm Overlay $Version", $SetupPath)
    & $SignTool.FullName @signArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Installer signing failed with exit code $LASTEXITCODE"
    }
}

Write-Host ""
Write-Host ("Installer: {0}" -f $SetupPath)
