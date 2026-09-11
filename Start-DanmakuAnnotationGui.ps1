#Requires -Version 7.0
[CmdletBinding()]
param(
    [int]$Port = 8765,
    [switch]$NoBrowser
)

$ErrorActionPreference = 'Stop'
$organizedLauncher = Join-Path $PSScriptRoot 'Tools/Danmaku/Start-DanmakuAnnotationGui.ps1'
& $organizedLauncher @PSBoundParameters
exit $LASTEXITCODE
