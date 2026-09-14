param([string]$BundlePath = '')

$ErrorActionPreference = 'Stop'
$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$assetRoot = Join-Path $repositoryRoot 'Widget/Assets/GameStyles/valorant/killconfirm'
$audioRoot = Join-Path $repositoryRoot 'SourceAssets/GameStyles/valorant/soundpacks'
$servicePath = Join-Path $repositoryRoot 'Widget/Services/Catalog/Games/ValorantPackService.cs'
$service = Get-Content -Raw -LiteralPath $servicePath
$editor = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Pages/Main/Packs/Creation/MainPage.PackCreation.ValorantIcon.cs')

$expected = @(
    '00009_prime', '00010_glitchpop',
    '00011_singularity_v1', '00012_singularity_v2', '00013_singularity_v3',
    '00014_gaia_s_vengeance', '00015_gaia_s_vengeance_v1',
    '00016_gaia_s_vengeance_v2', '00017_gaia_s_vengeance_v3',
    '00018_bubblegum_deathwish', '00019_bubblegum_deathwish_v1',
    '00020_bubblegum_deathwish_v2', '00021_bubblegum_deathwish_v3',
    '00022_champions_2021',
    '00023_prelude_to_chaos_v1', '00024_prelude_to_chaos_v2',
    '00025_prelude_to_chaos_v3',
    '00026_primordium', '00027_primordium_v1', '00028_primordium_v2',
    '00029_primordium_v3', '00030_radiant_crisis_001',
    '00031_rgx_11z_pro', '00032_rgx_11z_pro_v1',
    '00033_rgx_11z_pro_v2', '00034_rgx_11z_pro_v3'
)

$nativeSupportRoot = Join-Path $assetRoot '_native/shared/textures'
foreach ($texture in @(
    'Base_Badge_Dissolve.png', 'Base_FrameBG.png', 'Base_FrameDissolve.png',
    'Base_headshot.png', 'Base_RingBG.png', 'BaseT1_FX.png', 'BaseT2_FX.png',
    'BaseT3_FX.png', 'FB_HeroFlame.png', 'FB_Large_Sparks.png',
    'FB_X_Sparks.png', 'T_Mask_Ramp_TopDown.png', 'UI_Hud_Killbanner_VignetteFlat.png'
)) {
    if (-not (Test-Path -LiteralPath (Join-Path $nativeSupportRoot $texture) -PathType Leaf)) {
        throw "Missing current-renderer support texture: $texture"
    }
}

foreach ($folder in $expected) {
    $packRoot = Join-Path $assetRoot $folder
    $manifestPath = Join-Path $packRoot 'manifest.json'
    $textureRoot = Join-Path $packRoot 'textures'
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Missing restored Valorant icon pack manifest: $folder"
    }
    $manifest = (Get-Content -Raw -LiteralPath $manifestPath).TrimStart([char]0xFEFF) | ConvertFrom-Json
    if ($manifest.folder -ne $folder -or -not $manifest.textures.Count) {
        throw "Invalid restored Valorant icon pack manifest: $folder"
    }
    foreach ($texture in $manifest.textures) {
        if (-not (Test-Path -LiteralPath (Join-Path $textureRoot $texture) -PathType Leaf)) {
            throw "Missing restored Valorant texture: $folder/$texture"
        }
    }
    if ($service -notmatch [regex]::Escape("LegacyPack(`"$folder`"")) {
        throw "Restored Valorant icon pack is not registered: $folder"
    }
    $voiceRoot = Join-Path $audioRoot "valorant_$folder"
    $voiceManifestPath = Join-Path $voiceRoot 'manifest.json'
    if (-not (Test-Path -LiteralPath $voiceManifestPath -PathType Leaf)) {
        throw "Missing restored Valorant voice pack manifest: valorant_$folder"
    }
    $voiceManifest = Get-Content -Raw -LiteralPath $voiceManifestPath | ConvertFrom-Json
    foreach ($voice in @('1.wav', '2.wav', '3.wav', '4.wav', '5.wav')) {
        if (-not (Test-Path -LiteralPath (Join-Path $voiceRoot $voice) -PathType Leaf)) {
            throw "Missing restored Valorant voice: valorant_$folder/$voice"
        }
    }
}

$registered = [regex]::Matches($service, 'LegacyPack\("00(?:0(?:09)|0[1-3][0-9])[^\"]*"').Count
if ($registered -ne $expected.Count) {
    throw "Expected $($expected.Count) restored Valorant icon packs; found $registered registrations."
}
if ($service -notmatch 'HasBuiltInAudio\s*=\s*true') {
    throw 'Restored Valorant packs must expose their restored built-in audio.'
}
$renderSource = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Controls/Animations/Valorant/ValorantAnimation.Render.cs')
$resourceSource = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Controls/Animations/Valorant/ValorantAnimation.Resources.cs')
$legacySource = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Controls/Animations/Valorant/ValorantAnimation.Legacy.cs')
$profilesSource = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Controls/Animations/Valorant/ValorantAnimation.Profiles.cs')
$packSyncSource = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Pages/KillConfirmWidget/Packs/KillConfirmWidgetPage.PackSettings.Valorant.cs')
$catalogStorageSource = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Services/Catalog/PackCatalogService.Storage.cs')
if ($renderSource -notmatch 'LegacyRendering' -or $renderSource -notmatch 'DrawLegacyValorantKillFrame') {
    throw 'Legacy Valorant packs are not routed to the restored renderer.'
}
if ($renderSource -notmatch 'DrawNativeValorantFrame') {
    throw 'Current Valorant pack variants are not routed to the current renderer.'
}
foreach ($required in @('killicon_valorant_headshot.png', 'killicon_valorant_particle_base_t1.png',
    'killicon_valorant_particle_large_sparks.png', 'killicon_valorant_particle_x_sparks.png')) {
    if ($resourceSource -notmatch [regex]::Escape($required)) { throw "Legacy texture loader missing: $required" }
}
if ($legacySource -notmatch 'LegacyValorantBarAngles' -or $legacySource -notmatch 'GetLegacyValorantLifeOpacity') {
    throw 'Restored Valorant renderer is incomplete.'
}
function Get-MethodBody([string]$source, [string]$name) {
    $match = [regex]::Match(
        $source,
        '(?ms)^        private static [^\r\n]+\b' + [regex]::Escape($name) + '\([^)]*\)\s*\{.*?^        \}')
    if (-not $match.Success) { throw "Method not found: $name" }
    return $match.Value
}

$legacyGaiaProfile = Get-MethodBody $profilesSource 'LegacyGaia'
$legacyRgxProfile = Get-MethodBody $profilesSource 'LegacyRgx'
if ($legacyGaiaProfile -notmatch '\$"killicon_valorant_\{name\}_frame\.png"') {
    throw 'Gaia legacy variants must load their own frame texture.'
}
if ($legacyRgxProfile -notmatch '"killicon_valorant_rgx_11z_pro_frame\.png"') {
    throw 'RGX legacy variants must share the frame texture that actually exists in every variant folder.'
}
if ($packSyncSource -notmatch 'SelectedValorantIconMatchesAssociation') {
    throw 'Valorant voice-pack synchronization can still bounce a selected current-renderer variant back to legacy.'
}
if ($catalogStorageSource -notmatch 'mustSave\s*\|=\s*MergeMissingBuiltIns') {
    throw 'New built-in Valorant variants added to an existing catalog are not persisted.'
}

$rgxAudioRoot = Join-Path $audioRoot 'valorant_00031_rgx_11z_pro'
$rgxAudioManifest = Get-Content -Raw -LiteralPath (Join-Path $rgxAudioRoot 'manifest.json') | ConvertFrom-Json
if ($rgxAudioManifest.audio.slots.headshot -ne 'headshot.wav' -or
    -not (Test-Path -LiteralPath (Join-Path $rgxAudioRoot 'headshot.wav') -PathType Leaf)) {
    throw 'RGX 00031 headshot audio has not been restored.'
}

if (-not ('KillConfirmGameBar.Services.LegacyValorantPackRegression' -as [type])) {
    $stubs = @'
namespace KillConfirmGameBar.Services
{
    internal enum UiLanguage { SimplifiedChinese, English }
    internal static class LocalizationManager
    {
        public static UiLanguage Current => UiLanguage.English;
        public static string Text(string key)
        {
            if (key == "ValorantLegacyRendererSuffix") return " (Legacy renderer)";
            if (key == "ValorantCurrentRendererSuffix") return " (Current renderer)";
            return key;
        }
    }
    internal static class ValorantExternalAssetService
    {
        public static System.Collections.Generic.IReadOnlyList<ValorantPackInfo> DiscoverExternalPacks()
            => new ValorantPackInfo[0];
        public static string GetExternalEmblemUri(ValorantPackInfo pack) => null;
    }
    public static class LegacyValorantPackRegression
    {
        public static void Verify(string assetRoot, int expectedLegacyCount)
        {
            if (ValorantPackService.All.Count != expectedLegacyCount * 2 + 1)
                throw new System.Exception("Unexpected built-in Valorant pack count.");
            foreach (ValorantPackInfo legacy in ValorantPackService.All)
            {
                if (legacy.Key == ValorantPackService.DefaultKey
                    || legacy.RendererVariant != "legacy") continue;
                ValorantPackInfo current = ValorantPackService.Find(legacy.Key + "_current_renderer");
                if (legacy.IsExternal || !legacy.HasBuiltInAudio || legacy.Profile == null
                    || !legacy.Profile.LegacyRendering || current == null)
                    throw new System.Exception("Legacy renderer variant is invalid: " + legacy.Key);
                if (current.IsExternal || current.HasBuiltInAudio || current.Profile == null
                    || current.Profile.LegacyRendering || current.RendererVariant != "current"
                    || current.Folder != legacy.Folder || current.AssociationId != legacy.AssociationId)
                    throw new System.Exception("Current renderer variant is invalid: " + legacy.Key);
                if (!ValorantPackService.GetDisplayName(legacy.Key).Contains("Legacy renderer")
                    || !ValorantPackService.GetDisplayName(current.Key).Contains("Current renderer"))
                    throw new System.Exception("Renderer label is missing: " + legacy.Key);
                if (ValorantPackService.GetVoiceDisplayName(legacy.Key).Contains("renderer"))
                    throw new System.Exception("Renderer label leaked into voice pack: " + legacy.Key);
                string textureRoot = System.IO.Path.Combine(assetRoot, legacy.Folder, "textures");
                foreach (string file in new[] { legacy.Profile.Emblem, legacy.Profile.Bar,
                    legacy.Profile.BarHover, legacy.Profile.Frame, legacy.Profile.Ring, legacy.Profile.Blade })
                {
                    if (!string.IsNullOrWhiteSpace(file)
                        && !System.IO.File.Exists(System.IO.Path.Combine(textureRoot, file)))
                        throw new System.Exception("Registered texture is missing: " + legacy.Key + "/" + file);
                }
            }
        }
    }
}
'@
    Add-Type -TypeDefinition ($service + $stubs)
}
[KillConfirmGameBar.Services.LegacyValorantPackRegression]::Verify($assetRoot, $expected.Count)

foreach ($requiredEditorBehavior in @(
    'ValorantPackService.Find(item.Key)',
    'CreateValorantBuiltInCopyManifest',
    'builtIn.Folder',
    'profile.GetNamedString("emblem"'
)) {
    if ($editor -notmatch [regex]::Escape($requiredEditorBehavior)) {
        throw "Built-in Valorant editor does not preserve the selected legacy pack: $requiredEditorBehavior"
    }
}

if ($BundlePath) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $bundle = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $BundlePath).Path)
    $memory = $null
    $package = $null
    try {
        $main = $bundle.Entries | Where-Object { $_.FullName -like '*.msix' -and $_.FullName -notlike '*language-*' } |
            Sort-Object Length -Descending | Select-Object -First 1
        if (-not $main) { throw 'Bundle contains no main MSIX.' }
        $memory = [IO.MemoryStream]::new()
        $stream = $main.Open()
        try { $stream.CopyTo($memory) } finally { $stream.Dispose() }
        $memory.Position = 0
        $package = [IO.Compression.ZipArchive]::new($memory, [IO.Compression.ZipArchiveMode]::Read)
        $entries = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($entry in $package.Entries) { [void]$entries.Add([Uri]::UnescapeDataString($entry.FullName)) }
        foreach ($folder in $expected) {
            $manifest = (Get-Content -Raw -LiteralPath (Join-Path $assetRoot "$folder/manifest.json")).TrimStart([char]0xFEFF) | ConvertFrom-Json
            foreach ($texture in $manifest.textures) {
                $path = "Assets/GameStyles/valorant/killconfirm/$folder/textures/$texture"
                if (-not $entries.Contains($path)) { throw "Packaged Valorant texture missing: $path" }
            }
            $sourceVoiceRoot = Join-Path $audioRoot "valorant_$folder"
            foreach ($voiceFile in Get-ChildItem -LiteralPath $sourceVoiceRoot -File) {
                $voice = $voiceFile.Name
                $voicePath = "KillConfirmService/sounds/valorant_$folder/$voice"
                if (-not $entries.Contains($voicePath)) { throw "Packaged Valorant voice missing: $voicePath" }
            }
        }
    }
    finally {
        if ($package) { $package.Dispose() }
        if ($memory) { $memory.Dispose() }
        $bundle.Dispose()
    }
}

"PASS: all $($expected.Count) restored Valorant packs expose labeled legacy/current renderer variants, share complete assets, and keep one audio pack each."
