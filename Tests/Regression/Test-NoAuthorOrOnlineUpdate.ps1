$ErrorActionPreference = 'Stop'
$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$files = Get-ChildItem -LiteralPath $repositoryRoot -Recurse -File -Include *.cs,*.xaml,*.rs,*.appxmanifest |
    Where-Object { $_.FullName -notmatch '[\\/](?:bin|obj)[\\/]' }
$content = ($files | ForEach-Object { Get-Content -Raw -LiteralPath $_.FullName }) -join "`n"

foreach ($forbidden in @(
    'CheckForUpdatesAsync', 'UpdateOverlayView', 'UpdateRequested=',
    'CreditsCommunityPanel', '作者与致谢', 'Author & credits',
    'github.com/eachkinji', 'space.bilibili.com/18017622'
)) {
    if ($content -match [regex]::Escape($forbidden)) {
        throw "Author/update UI or online checking remains: $forbidden"
    }
}

'PASS: widget contains no author page or online update-check entry point.'
