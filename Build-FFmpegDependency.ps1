#Requires -Version 7.0
param([Parameter(Mandatory)][string]$Destination)
$ErrorActionPreference = 'Stop'
$assetName = 'ffmpeg-n9.0-latest-win64-lgpl-9.0.zip'
$releaseRoot = 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest'
$assetUrl = "$releaseRoot/$assetName"
$checksumsUrl = "$releaseRoot/checksums.sha256"
$cacheRoot = Join-Path $env:LOCALAPPDATA 'KillConfirmBuildCache/ffmpeg-n9.0-lgpl'
$archive = Join-Path $cacheRoot 'ffmpeg-lgpl.zip'
$checksumCache = Join-Path $cacheRoot 'ffmpeg-lgpl.sha256'
New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null

$archiveSha256 = if (Test-Path -LiteralPath $checksumCache) {
    (Get-Content -LiteralPath $checksumCache -Raw).Trim().ToUpperInvariant()
} else {
    ''
}
$cachedArchiveIsValid =
    $archiveSha256 -match '^[0-9A-F]{64}$' -and
    (Test-Path -LiteralPath $archive) -and
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -eq $archiveSha256

if (-not $cachedArchiveIsValid) {
    $checksumDownload = "$checksumCache.download"
    Remove-Item -LiteralPath $checksumDownload -Force -ErrorAction SilentlyContinue
    Invoke-WebRequest -Uri $checksumsUrl -Headers @{ 'User-Agent' = 'KillConfirmGameBar-Build' } -OutFile $checksumDownload
    $checksumLine = Get-Content -LiteralPath $checksumDownload |
        Where-Object { $_ -match "^([0-9a-fA-F]{64})\s+\*?$([regex]::Escape($assetName))$" } |
        Select-Object -First 1
    if (-not $checksumLine) {
        Remove-Item -LiteralPath $checksumDownload -Force
        throw "FFmpeg checksum manifest does not contain $assetName."
    }
    $archiveSha256 = ([regex]::Match($checksumLine, '^[0-9a-fA-F]{64}').Value).ToUpperInvariant()

    $download = "$archive.download"
    Remove-Item -LiteralPath $download -Force -ErrorAction SilentlyContinue
    Invoke-WebRequest -Uri $assetUrl -Headers @{ 'User-Agent' = 'KillConfirmGameBar-Build' } -OutFile $download
    if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ne $archiveSha256) {
        Remove-Item -LiteralPath $download -Force
        throw 'FFmpeg archive checksum mismatch.'
    }
    Move-Item -LiteralPath $download -Destination $archive -Force
    [IO.File]::WriteAllText($checksumCache, $archiveSha256)
    Remove-Item -LiteralPath $checksumDownload -Force
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $archiveSha256) { throw 'FFmpeg archive checksum mismatch.' }
$extract = Join-Path $cacheRoot 'extract'
if (-not (Test-Path -LiteralPath (Join-Path $extract 'ffmpeg.exe'))) {
    $resolvedExtract = [IO.Path]::GetFullPath($extract)
    $resolvedCache = [IO.Path]::GetFullPath($cacheRoot).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolvedExtract.StartsWith($resolvedCache, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe FFmpeg cache path.' }
    if (Test-Path -LiteralPath $resolvedExtract) { Remove-Item -LiteralPath $resolvedExtract -Recurse -Force }
    Expand-Archive -LiteralPath $archive -DestinationPath $extract
    $root = Get-ChildItem -LiteralPath $extract -Directory | Select-Object -First 1
    Copy-Item -LiteralPath (Join-Path $root.FullName 'bin/ffmpeg.exe') -Destination $extract -Force
    Copy-Item -LiteralPath (Join-Path $root.FullName 'LICENSE.txt') -Destination $extract -Force
}
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
Copy-Item -LiteralPath (Join-Path $extract 'ffmpeg.exe'),(Join-Path $extract 'LICENSE.txt') -Destination $Destination -Force
$lines = @(
    'FFmpeg n9.0 LGPLv3 build by BtbN (invoked as a separate process for video import)'
    'Build and corresponding source information: https://github.com/BtbN/FFmpeg-Builds'
    "Binary archive: $assetName"
    "Binary archive SHA-256: $archiveSha256"
)
[IO.File]::WriteAllLines((Join-Path $Destination 'SOURCE.txt'), $lines)
