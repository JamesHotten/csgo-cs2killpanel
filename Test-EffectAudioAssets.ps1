param(
    [switch]$SkipRustTests
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$errors = [System.Collections.Generic.List[string]]::new()
$checks = 0

function Test-RequiredFile {
    param([string]$RelativePath)

    $script:checks++
    $fullPath = Join-Path $repoRoot $RelativePath
    if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        $script:errors.Add("Missing file: $RelativePath")
    }
}

function Test-PngFile {
    param([System.IO.FileInfo]$File)

    $script:checks++
    if ($File.Length -lt 8) {
        $script:errors.Add("Invalid PNG (too small): $($File.FullName)")
        return
    }

    $signature = [System.IO.File]::ReadAllBytes($File.FullName)[0..7]
    if ([System.BitConverter]::ToString($signature) -ne '89-50-4E-47-0D-0A-1A-0A') {
        $script:errors.Add("Invalid PNG signature: $($File.FullName)")
    }
}

# XAML event binding and generated-code compilation are also checked by the Release build.
Get-ChildItem (Join-Path $repoRoot 'Widget') -Recurse -Filter '*.xaml' | ForEach-Object {
    $checks++
    try {
        [xml](Get-Content -LiteralPath $_.FullName -Raw) | Out-Null
    }
    catch {
        $errors.Add("Invalid XAML: $($_.FullName): $($_.Exception.Message)")
    }
}

# Shared control-panel styles must live at application scope because both the
# standalone settings page and the Game Bar widget instantiate these controls.
$appXamlPath = Join-Path $repoRoot 'Widget\App.xaml'
$appXaml = Get-Content -LiteralPath $appXamlPath -Raw
foreach ($resourceKey in @(
    'CompactSettingsComboBoxStyle',
    'CompactChoiceCardStyle',
    'CompactCircleIconStyle',
    'CompactChoiceLabelStyle'
)) {
    $checks++
    if ($appXaml -notmatch ('x:Key="' + [regex]::Escape($resourceKey) + '"')) {
        $errors.Add("Missing application XAML resource: $resourceKey")
    }
}

# Legacy/remastered CrossFire animation sheets.
$legacyKeys = @(
    '1killre', '2killre', '3killre', '4killre', '5killre', '6killre',
    'headshot_silver', 'goldheadshot', 'knife_kill', 'firstkill', 'last_kill'
)
foreach ($key in $legacyKeys) {
    $manifestPath = Join-Path $repoRoot "Widget\Assets\KillConfirmSheets\$key.json"
    Test-RequiredFile "Widget\Assets\KillConfirmSheets\$key.json"
    if (Test-Path -LiteralPath $manifestPath) {
        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        $sheetFrames = 0
        foreach ($sheet in $manifest.sheets) {
            Test-RequiredFile "Widget\Assets\KillConfirmSheets\$($sheet.file)"
            $sheetFrames += [int]$sheet.frames
        }
        $checks++
        if ($sheetFrames -ne [int]$manifest.frames) {
            $errors.Add("Frame count mismatch: $key ($sheetFrames != $($manifest.frames))")
        }
    }
}

# CrossFire code-rendered main icons and optional overlay layers.
$cfFiles = @(
    'Original\badge_multi1.png', 'Original\badge_multi2.png', 'Original\badge_multi3.png',
    'Original\badge_multi4.png', 'Original\badge_multi5.png', 'Original\badge_multi6.png',
    'Original\badge_headshot.png', 'Original\badge_headshot_gold.png', 'Original\badge_Assist.png',
    'Knife\badge_knife.png', 'Knife\badge_knife_1.png', 'Knife\badge_knife_2.png', 'Knife\badge_knife_3.png',
    'FirstLast\FIRSTKILL.png', 'FirstLast\LASTKILL.png',
    'CommonFx\multi2_fx.png', 'CommonFx\multi3_fx.png', 'CommonFx\multi4_fx.png',
    'CommonFx\multi5_fx.png', 'CommonFx\multi6_fx.png',
    'EliteUpgrade\KillMark_Upgrade1.png', 'EliteUpgrade\KillMark_Upgrade2.png',
    'EliteUpgrade\KillMark_Upgrade3.png'
)
foreach ($file in $cfFiles) {
    Test-RequiredFile "Widget\Assets\KillConfirmCode\$file"
}
foreach ($className in @('Assault', 'Elite', 'Scout', 'Sniper', 'Knife')) {
    foreach ($level in 1..3) {
        Test-RequiredFile "Widget\Assets\KillConfirmCode\WeaponBadge\badge_${className}${level}.png"
    }
}
foreach ($pack in @('Original', 'Vip', 'AngelicBeast', 'Anniversary10', 'Anniversary15', 'CFPL', 'Rankmach2019_1', 'Rankmach2019_2')) {
    foreach ($tier in 1..6) {
        Test-RequiredFile "Widget\Assets\KillConfirmCode\$pack\badge_multi$tier.png"
    }
    Test-RequiredFile "Widget\Assets\KillConfirmCode\$pack\badge_headshot.png"
    Test-RequiredFile "Widget\Assets\KillConfirmCode\$pack\badge_headshot_gold.png"
}

# Battlefield/PUBG/Delta Force renderer file maps.
$styleFiles = @{
    'battlefield1' = @(
        'killicon_battlefield1_default.png', 'killicon_battlefield1_headshot.png',
        'killicon_battlefield1_crit.png', 'killicon_battlefield1_destroyvehicle.png',
        'killicon_battlefield1_explosion.png'
    )
    'battlefield5' = @(
        'killicon_battlefield5_default.png', 'killicon_battlefield5_headshot.png',
        'killicon_battlefield5_assist.png', 'killicon_battlefield5_destroyvehicle.png'
    )
    'battlefield4' = @('killicon_battlefield1_default.png', 'killicon_battlefield1_headshot.png')
    'battlefield2042' = @(
        'Assist.png', 'AssistSprite.png', 'HeadshotSkull.png', 'HeadshotSkullSprite.png',
        'NormalSkull.png', 'NormalSkullSprite.png', 'HitmarkerKillArm.png', 'SmoothCircle.png',
        'Glitch0.png', 'Glitch1.png', 'Glitch2.png', 'Glitch3.png', 'Glitch4.png'
    )
    'pubg' = @(
        'killicon_scrolling_default.png', 'killicon_scrolling_headshot.png',
        'killicon_scrolling_crit.png', 'killicon_scrolling_assist.png',
        'killicon_scrolling_destroyvehicle.png', 'killicon_scrolling_explosion.png'
    )
    'deltaforce' = @(
        'killicon_df_default.png', 'killicon_df_headshot.png', 'killicon_df_capture.png',
        'killicon_df_destroyvehicle.png', 'killicon_scrolling_assist.png'
    )
}
foreach ($style in $styleFiles.Keys) {
    foreach ($file in $styleFiles[$style]) {
        Test-RequiredFile "Widget\Assets\GameStyles\$style\killconfirm\textures\$file"
    }
}

# Every Valorant visual manifest must have all textures and a matching service voice pack.
$valorantRoot = Join-Path $repoRoot 'Widget\Assets\GameStyles\valorant\killconfirm'
$valorantManifests = Get-ChildItem $valorantRoot -Directory | ForEach-Object {
    Get-Item (Join-Path $_.FullName 'manifest.json')
}
$checks++
if ($valorantManifests.Count -lt 26) {
    $errors.Add("Unexpected Valorant manifest count: $($valorantManifests.Count)")
}
foreach ($manifestFile in $valorantManifests) {
    $manifest = Get-Content -LiteralPath $manifestFile.FullName -Raw | ConvertFrom-Json
    $folder = $manifest.folder
    $checks++
    if ($manifestFile.Directory.Name -ne $folder) {
        $errors.Add("Valorant folder mismatch: $($manifestFile.Directory.Name) != $folder")
    }
    foreach ($texture in $manifest.textures) {
        Test-RequiredFile "Widget\Assets\GameStyles\valorant\killconfirm\$folder\textures\$texture"
    }
    foreach ($audioName in @('1.wav', '2.wav', '3.wav', '4.wav', '5.wav', 'headshot.wav', 'sound.lua')) {
        Test-RequiredFile "KillConfirmService\sounds\valorant_$folder\$audioName"
    }
}

# Check all shipped PNGs, including files reached through renderer fallbacks.
Get-ChildItem (Join-Path $repoRoot 'Widget\Assets') -Recurse -File |
    Where-Object { $_.Extension -ieq '.png' } |
    ForEach-Object { Test-PngFile $_ }

# Presentation regressions: zero-money Delta rounds are outcomes, and Battlefield V
# must keep the service event order even while the preceding kill icon loads async.
$deltaSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\Controls\KillConfirmAnimation.DeltaForce.cs') -Raw
$checks++
if ($deltaSource -notmatch 'eventKind,\s*moneyReward\);') {
    $errors.Add('Delta Force feed labels are not receiving the resolved money reward.')
}
$checks++
if ($deltaSource -notmatch 'moneyReward > 0[\s\S]*"回合胜利" : "回合失败"') {
    $errors.Add('Delta Force zero-money round events are not labelled as round outcomes.')
}

$battlefield5Source = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\Controls\KillConfirmAnimation.Battlefield5.cs') -Raw
$battlefield5ModelsSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\Controls\KillConfirmAnimation.Battlefield5.Models.cs') -Raw
$checks++
if ($battlefield5Source -notmatch 'QueueBattlefield5PendingEvent\(pendingEvent\)[\s\S]*LoadBattlefieldIconAsync') {
    $errors.Add('Battlefield V does not reserve the kill event order before async icon loading.')
}
$checks++
if ($battlefield5Source -notmatch 'PendingEvents\[0\]\.IsReady') {
    $errors.Add('Battlefield V pending events can overtake an earlier icon that is still loading.')
}
$checks++
if ($battlefield5Source -notmatch 'PendingEvents\.Count >= Battlefield5MaxPendingEvents[\s\S]*return false;') {
    $errors.Add('Battlefield V queue overflow can discard the ordered head event.')
}
$checks++
if ($battlefield5ModelsSource -notmatch 'public bool IsReady \{ get; set; \}') {
    $errors.Add('Battlefield V pending event readiness state is missing.')
}

# CS2 assists contribute to the scoreboard but do not award player cash. Keep the
# UI boundary defensive so test/custom events cannot inject a false assist reward.
$animationPageSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\KillConfirmWidgetPage.Animation.cs') -Raw
$checks++
if ($animationPageSource -notmatch 'killEvent\.IsAssist[\s\S]*GetBattlefieldEventKind\(killEvent\), "assist"[\s\S]*return 0;') {
    $errors.Add('Assist events are not clamped to zero CS2 cash at the presentation boundary.')
}
$checks++
if ([regex]::Matches($animationPageSource, 'GetDisplayedMoneyReward\(killEvent\)').Count -ne 6) {
    $errors.Add('Not every reward-capable game style uses the assist-safe money reward.')
}

# Selectively ported upstream robustness fixes must remain compatible with the
# existing WebSocket/Legacy implementation and per-style pack model.
$handlerSource = Get-Content -LiteralPath (Join-Path $repoRoot 'KillConfirmService\src\util\handler.rs') -Raw
$parsePosition = $handlerSource.IndexOf('let parsed = match parse_gsi_frame(&body)')
$acceptedPostPosition = $handlerSource.IndexOf('let posts = app_state.gsi_posts.fetch_add')
$checks++
if ($parsePosition -lt 0 -or $acceptedPostPosition -le $parsePosition) {
    $errors.Add('Rejected GSI payloads are being counted as accepted posts.')
}

$eventStreamSource = Get-Content -LiteralPath (Join-Path $repoRoot 'KillConfirmService\src\util\event_stream.rs') -Raw
$checks++
if ([regex]::Matches($eventStreamSource, 'else if previous_active \{[\s\S]{0,300}assist_audio_setting_active[\s\S]{0,80}store\(false').Count -lt 2) {
    $errors.Add('Inactive CF/shared modes can leave assist-audio settings active.')
}

$loggingSource = Get-Content -LiteralPath (Join-Path $repoRoot 'KillConfirmService\src\util\logging.rs') -Raw
$checks++
if ($loggingSource -notmatch 'MAX_SERVICE_LOG_BYTES[\s\S]*rotate_if_needed\(&log_path\)[\s\S]*with_extension\("log\.old"\)') {
    $errors.Add('Bounded service.log rotation is missing.')
}

$packPersistenceSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\KillConfirmWidgetPage.PackSettings.Persistence.cs') -Raw
$checks++
if ($packPersistenceSource -notmatch 'GetPackSettingKey\(legacySettingKey, style\)' -or
    $packPersistenceSource -notmatch 'GameStyleService\.GetStyleForPackKey\(value\) == style') {
    $errors.Add('Icon/voice pack settings are not isolated by game style.')
}

$servicePageSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\KillConfirmWidgetPage.Service.cs') -Raw
$checks++
if ($servicePageSource -notmatch 'AudioOutputDevice' -and
    $servicePageSource -notmatch 'SyncAudioDeviceAsync') {
    $errors.Add('The selected audio device is not restored after service reconnect.')
}

$csConfigSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Widget\KillConfirmWidgetPage.CsConfig.cs') -Raw
$checks++
if ([regex]::Matches($csConfigSource, 'await TryAutoDetectCsFolderAsync\(\);').Count -lt 2) {
    $errors.Add('CS2 CFG auto-detection is not used when saved folder access is unavailable.')
}

# A deferred MSIX update can stage the new package while leaving the current
# user registered to the previous version. The portable installer must detect
# that state and activate the staged package without deleting application data.
$transferBuildSource = Get-Content -LiteralPath (Join-Path $repoRoot 'Build-TransferPackage.ps1') -Raw
$checks++
if ($transferBuildSource -notmatch 'function Ensure-OverlayPackageVersion') {
    $errors.Add('The portable installer does not validate the active overlay version after staging an update.')
}
$checks++
if ($transferBuildSource -notmatch 'RegisterByFamilyName[\s\S]*Ensure-OverlayPackageVersion -Identity \$msixIdentity') {
    $errors.Add('The portable installer cannot activate a staged overlay update while preserving application data.')
}

if (-not $SkipRustTests) {
    & cargo test --manifest-path (Join-Path $repoRoot 'KillConfirmService\Cargo.toml') `
        every_builtin_audio_route_points_to_an_existing_decodable_file --quiet
    if ($LASTEXITCODE -ne 0) {
        $errors.Add('Built-in audio routing/decoding test failed.')
    }
}

if ($errors.Count -gt 0) {
    $errors | ForEach-Object { Write-Error $_ }
    throw "Effect/audio asset audit failed with $($errors.Count) error(s)."
}

Write-Host "Effect/audio asset audit passed: $checks checks, $($valorantManifests.Count) Valorant packs, $($legacyKeys.Count) legacy animation sets."
