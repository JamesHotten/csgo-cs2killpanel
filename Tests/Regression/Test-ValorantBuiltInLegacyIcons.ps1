param([string]$BundlePath = '')

$ErrorActionPreference = 'Stop'
$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$assetRoot = Join-Path $repositoryRoot 'Widget/Assets/GameStyles/valorant/killconfirm'
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
}

$registered = [regex]::Matches($service, 'LegacyPack\("00(?:0(?:09)|0[1-3][0-9])[^\"]*"').Count
if ($registered -ne $expected.Count) {
    throw "Expected $($expected.Count) restored Valorant icon packs; found $registered registrations."
}
if ($service -notmatch 'HasBuiltInAudio\s*=\s*false') {
    throw 'Restored Valorant visual packs must not claim missing built-in audio.'
}

if (-not ('KillConfirmGameBar.Services.LegacyValorantPackRegression' -as [type])) {
    $stubs = @'
namespace KillConfirmGameBar.Services
{
    internal enum UiLanguage { SimplifiedChinese, English }
    internal static class LocalizationManager
    {
        public static UiLanguage Current => UiLanguage.English;
        public static string Text(string key) => key;
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
            if (ValorantPackService.All.Count != expectedLegacyCount + 1)
                throw new System.Exception("Unexpected built-in Valorant pack count.");
            foreach (ValorantPackInfo pack in ValorantPackService.All)
            {
                if (pack.Key == ValorantPackService.DefaultKey) continue;
                if (pack.IsExternal || pack.HasBuiltInAudio || pack.Profile == null)
                    throw new System.Exception("Legacy pack flags changed: " + pack.Key);
                string textureRoot = System.IO.Path.Combine(assetRoot, pack.Folder, "textures");
                foreach (string file in new[] { pack.Profile.Emblem, pack.Profile.Bar,
                    pack.Profile.BarHover, pack.Profile.Frame, pack.Profile.Ring, pack.Profile.Blade })
                {
                    if (!string.IsNullOrWhiteSpace(file)
                        && !System.IO.File.Exists(System.IO.Path.Combine(textureRoot, file)))
                        throw new System.Exception("Registered texture is missing: " + pack.Key + "/" + file);
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
        }
    }
    finally {
        if ($package) { $package.Dispose() }
        if ($memory) { $memory.Dispose() }
        $bundle.Dispose()
    }
}

"PASS: all $($expected.Count) restored Valorant icon packs are complete, registered and visual-only."
