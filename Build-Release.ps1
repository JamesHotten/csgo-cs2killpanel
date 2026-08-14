param(
    [string]$Configuration = "Release",
    [string]$Platform = "x64",
    [string]$MsBuildPath = "",
    [string]$VcInstallPath = "",
    [switch]$DisableSigning,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspaceRoot = Split-Path -Parent $root
$manifestPath = Join-Path $root "Package\Package.appxmanifest"
[xml]$manifest = Get-Content -LiteralPath $manifestPath
$version = $manifest.Package.Identity.Version

$buildArguments = @{
    Configuration = $Configuration
    Platform = $Platform
    MsBuildPath = $MsBuildPath
}
if ($VcInstallPath) {
    $buildArguments.VcInstallPath = $VcInstallPath
}
if ($DisableSigning) {
    $buildArguments.DisableSigning = $true
}

if (-not $SkipBuild) {
    & (Join-Path $root "Build-TransferPackage.ps1") @buildArguments
}

$transferZip = Join-Path $workspaceRoot ("KillConfirmGameBar_Transfer_{0}.zip" -f $version)
$packageRoot = Join-Path $workspaceRoot ("KillConfirmGameBar_Transfer_{0}\OverlayPackage" -f $version)
$msix = Get-ChildItem -LiteralPath $packageRoot -Filter "*.msix" -Recurse |
    Where-Object { $_.Name -like "*$version*" -and $_.Name -like "*$Platform*" } |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
$certificate = Get-ChildItem -LiteralPath $packageRoot -Filter "*.cer" -Recurse |
    Where-Object { $_.DirectoryName -eq $msix.DirectoryName } |
    Select-Object -First 1

if (-not (Test-Path -LiteralPath $transferZip)) {
    throw "Transfer ZIP was not produced: $transferZip"
}
if (-not $msix) {
    throw "MSIX package was not produced under $packageRoot"
}
if (-not $DisableSigning -and -not $certificate) {
    throw "Test certificate was not produced beside $($msix.FullName)"
}

$assets = @(
    Get-Item -LiteralPath $transferZip
    $msix
)
if ($certificate) {
    $assets += $certificate
}

$checksumPath = Join-Path $workspaceRoot ("KillConfirmGameBar_{0}_SHA256SUMS.txt" -f $version)
$checksumLines = foreach ($asset in $assets) {
    $hash = Get-FileHash -LiteralPath $asset.FullName -Algorithm SHA256
    "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), $asset.Name
}
$checksumLines | Set-Content -LiteralPath $checksumPath -Encoding ASCII
$assets += Get-Item -LiteralPath $checksumPath

Write-Host "Release assets generated without installing the package:"
foreach ($asset in $assets) {
    Write-Host ("RELEASE_ASSET={0}" -f $asset.FullName)
}
