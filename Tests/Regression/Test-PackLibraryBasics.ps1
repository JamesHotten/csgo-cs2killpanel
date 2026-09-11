$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$navigation = Get-Content -Raw (Join-Path $root 'Widget/Services/Catalog/PackLibraryNavigation.cs')
$selectors = Get-Content -Raw (Join-Path $root 'Widget/Pages/KillConfirmWidget/Packs/KillConfirmWidgetPage.PackSelectors.cs')
$settings = Get-Content -Raw (Join-Path $root 'Widget/Pages/KillConfirmWidget/Packs/KillConfirmWidgetPage.PackSettings.cs')
$library = Get-Content -Raw (Join-Path $root 'Widget/Pages/Main/Packs/MainPage.PackLibrary.cs')
[xml]$xaml = Get-Content -Raw (Join-Path $root 'Widget/Pages/Main/MainPage.xaml')

foreach ($needle in @(
    'https://pan.quark.cn/s/f93adc47c434?pwd=JEcL',
    'https://pan.quark.cn/s/070d14fa9438?pwd=YwFG',
    'https://pan.quark.cn/s/52c6d57d73e9?pwd=cgCV',
    'https://pan.quark.cn/s/9467261e2bd5?pwd=czBG',
    'PendingPackLibraryNavigation',
    'ToUnixTimeSeconds() - created) > 120')) {
    if (-not $navigation.Contains($needle)) { throw "Missing navigation contract: $needle" }
}
if ($selectors -notmatch 'CreateAddMorePackItem\(true\)' -or $selectors -notmatch 'CreateAddMorePackItem\(false\)') {
    throw 'Widget selectors do not expose both pack-library actions.'
}
if ($settings -notmatch 'OpenPackLibraryAsync\(true, e\)' -or $settings -notmatch 'OpenPackLibraryAsync\(false, e\)') {
    throw 'Widget pack-library actions are not routed.'
}
if ($library -notmatch 'ApplyPendingPackLibraryNavigation' -or $library -notmatch 'Launcher\.LaunchUriAsync') {
    throw 'Settings pack-library navigation/download handler is incomplete.'
}
$ns = New-Object Xml.XmlNamespaceManager($xaml.NameTable)
$ns.AddNamespace('ui','http://schemas.microsoft.com/winfx/2006/xaml/presentation')
$ns.AddNamespace('x','http://schemas.microsoft.com/winfx/2006/xaml')
foreach ($name in @('DownloadVoicePackButton','DownloadIconPackButton')) {
    if (-not $xaml.SelectSingleNode("//*[@x:Name='$name']", $ns)) { throw "Missing $name" }
}
'PASS: four download links, expiring one-shot game/tab navigation, widget entry points and settings download controls.'
