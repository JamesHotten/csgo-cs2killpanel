#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))

$pythonWrappers = @(
    'Build-CrossfirePacks.py',
    'Build-CrossfireExternalPacks.py'
)
foreach ($name in $pythonWrappers) {
    $path = Join-Path $root $name
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Missing legacy tool wrapper: $name"
    }
    $help = & python $path --help 2>&1
    if ($LASTEXITCODE -ne 0 -or ($help -join "`n") -notmatch 'usage:') {
        throw "Legacy Python tool wrapper is not executable: $name"
    }
}

$powershellWrapper = Join-Path $root 'Start-DanmakuAnnotationGui.ps1'
if (-not (Test-Path -LiteralPath $powershellWrapper)) {
    throw 'Missing legacy Danmaku GUI wrapper'
}
$wrapperText = Get-Content -Raw -LiteralPath $powershellWrapper
if ($wrapperText -notmatch 'Tools[\\/]Danmaku[\\/]Start-DanmakuAnnotationGui\.ps1') {
    throw 'Legacy Danmaku GUI wrapper does not forward to the organized tool path'
}

'PASS: legacy root tool paths remain executable after Tools/ reorganization.'
