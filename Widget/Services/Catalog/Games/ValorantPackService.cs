using System;
using System.Collections.Generic;
using System.Linq;

namespace KillConfirmGameBar.Services
{
    internal sealed class ValorantPackInfo
    {
        public string Key { get; set; }
        public string Folder { get; set; }
        public string DisplayName { get; set; }
        public string ChineseDisplayName { get; set; }

        /// <summary>
        /// Emblem texture file name inside the native theme's textures folder.
        /// </summary>
        public string EmblemFile { get; set; }

        /// <summary>
        /// Whether the native audio shipped with this visual pack is available
        /// in the core application.
        /// </summary>
        public bool HasBuiltInAudio { get; set; }
        public bool IsExternal { get; set; }
        public string AssociationId { get; set; }
        public string FolderPath { get; set; }
        public ValorantVisualProfileInfo Profile { get; set; }
    }

    internal sealed class ValorantVisualProfileInfo
    {
        public string Accent { get; set; }
        public string Emblem { get; set; }
        public string Frame { get; set; }
        public string Bar { get; set; }
        public string BarHover { get; set; }
        public string Ring { get; set; }
        public string FrameDissolve { get; set; }
        public string BadgeDissolve { get; set; }
        public string Blade { get; set; }
        public string SpecialFrame { get; set; }
        public double HeadshotX { get; set; }
        public double HeadshotY { get; set; }
        public double SliceSize { get; set; }
        public bool LegacyRendering { get; set; }
    }

    internal static class ValorantPackService
    {
        public const string DefaultKey = "valorant_00000_base";

        // The public keys remain stable for saved settings. Their folders now point
        // directly at the replacement tree built from the cooked VALORANT exports.
        private static readonly ValorantPackInfo[] BuiltInPacks =
        {
            Pack("00000_base", "_native/themes/Base", "Base", "Base_Emblem.png", hasBuiltInAudio: true),
            LegacyPack("00009_prime", "Prime", "killicon_valorant_prime_emblem.png", "killicon_valorant_bar.png", "killicon_valorant_prime_frame.png", "killicon_valorant_base_ring.png", "#FF8000", sliceSize: 145, headshotY: -17),
            LegacyPack("00010_glitchpop", "Glitchpop", "killicon_valorant_glitchpop_emblem.png", "killicon_valorant_glitchpop_bar.png", "killicon_valorant_glitchpop_frame.png", "killicon_valorant_glitchpop_ring.png", "#68F5FF", sliceSize: 172, headshotY: -10),
            LegacyPack("00011_singularity_v1", "Singularity V1", "killicon_valorant_singularity_v1_emblem.png", "killicon_valorant_singularity_v1_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#F67A44", sliceSize: 140, headshotY: -10),
            LegacyPack("00012_singularity_v2", "Singularity V2", "killicon_valorant_singularity_v2_emblem.png", "killicon_valorant_singularity_v2_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#D6B644", sliceSize: 140, headshotY: -10),
            LegacyPack("00013_singularity_v3", "Singularity V3", "killicon_valorant_singularity_v3_emblem.png", "killicon_valorant_singularity_v3_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#436CBF", sliceSize: 140, headshotY: -10),
            LegacyPack("00014_gaia_s_vengeance", "Gaia's Vengeance", "killicon_valorant_gaia_emblem.png", "killicon_valorant_gaia_bar.png", "killicon_valorant_gaia_frame.png", "killicon_valorant_base_ring.png", "#C01B1F", headshotX: -2, headshotY: -20),
            LegacyPack("00015_gaia_s_vengeance_v1", "Gaia's Vengeance V1", "killicon_valorant_gaia_v1_emblem.png", "killicon_valorant_gaia_v1_bar.png", "killicon_valorant_gaia_v1_frame.png", "killicon_valorant_base_ring.png", "#0871F7", headshotX: -2, headshotY: -20),
            LegacyPack("00016_gaia_s_vengeance_v2", "Gaia's Vengeance V2", "killicon_valorant_gaia_v2_emblem.png", "killicon_valorant_gaia_v2_bar.png", "killicon_valorant_gaia_v2_frame.png", "killicon_valorant_base_ring.png", "#257133", headshotX: -2, headshotY: -20),
            LegacyPack("00017_gaia_s_vengeance_v3", "Gaia's Vengeance V3", "killicon_valorant_gaia_v3_emblem.png", "killicon_valorant_gaia_v3_bar.png", "killicon_valorant_gaia_v3_frame.png", "killicon_valorant_base_ring.png", "#CB2C00", headshotX: -2, headshotY: -20),
            LegacyPack("00018_bubblegum_deathwish", "Bubblegum Deathwish", "killicon_valorant_bubblegum_deathwish_emblem.png", "killicon_valorant_bubblegum_deathwish_bar.png", "killicon_valorant_bubblegum_deathwish_frame.png", "killicon_valorant_bubblegum_deathwish_ring.png", "#AF00A3", "killicon_valorant_bubblegum_deathwish_blade.png", 170, 0, 3.25),
            LegacyPack("00019_bubblegum_deathwish_v1", "Bubblegum Deathwish V1", "killicon_valorant_bubblegum_deathwish_v3_emblem.png", "killicon_valorant_bubblegum_deathwish_v1_bar.png", "killicon_valorant_bubblegum_deathwish_frame.png", "killicon_valorant_bubblegum_deathwish_ring.png", "#FFC359", "killicon_valorant_bubblegum_deathwish_blade.png", 170, -1, -16.5),
            LegacyPack("00020_bubblegum_deathwish_v2", "Bubblegum Deathwish V2", "killicon_valorant_bubblegum_deathwish_v2_emblem.png", "killicon_valorant_bubblegum_deathwish_v2_bar.png", "killicon_valorant_bubblegum_deathwish_frame.png", "killicon_valorant_bubblegum_deathwish_ring.png", "#932B00", "killicon_valorant_bubblegum_deathwish_blade.png", 170, 0.6, -4),
            LegacyPack("00021_bubblegum_deathwish_v3", "Bubblegum Deathwish V3", "killicon_valorant_bubblegum_deathwish_v1_emblem.png", "killicon_valorant_bubblegum_deathwish_v3_bar.png", "killicon_valorant_bubblegum_deathwish_frame.png", "killicon_valorant_bubblegum_deathwish_ring.png", "#06A600", "killicon_valorant_bubblegum_deathwish_blade.png", 170, 8, 4),
            LegacyPack("00022_champions_2021", "Champions 2021", "killicon_valorant_champions_2021_emblem.png", "killicon_valorant_champions_2021_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#C5B174", sliceSize: 134),
            LegacyPack("00023_prelude_to_chaos_v1", "Prelude to Chaos V1", "killicon_valorant_prelude_to_chaos_v1_emblem.png", "killicon_valorant_prelude_to_chaos_v1_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#F35D45", sliceSize: 170, headshotY: -18),
            LegacyPack("00024_prelude_to_chaos_v2", "Prelude to Chaos V2", "killicon_valorant_prelude_to_chaos_v2_emblem.png", "killicon_valorant_prelude_to_chaos_v2_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#01BA01", sliceSize: 170, headshotY: -18),
            LegacyPack("00025_prelude_to_chaos_v3", "Prelude to Chaos V3", "killicon_valorant_prelude_to_chaos_v3_emblem.png", "killicon_valorant_prelude_to_chaos_v3_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#0D7DF5", sliceSize: 170, headshotY: -18),
            LegacyPack("00026_primordium", "Primordium", "killicon_valorant_primordium_emblem.png", "killicon_valorant_primordium_bar.png", "killicon_valorant_primordium_frame.png", "killicon_valorant_base_ring.png", "#FE6D41", sliceSize: 152),
            LegacyPack("00027_primordium_v1", "Primordium V1", "killicon_valorant_primordium_v1_emblem.png", "killicon_valorant_primordium_v1_bar.png", "killicon_valorant_primordium_frame.png", "killicon_valorant_base_ring.png", "#84EAB6", sliceSize: 152),
            LegacyPack("00028_primordium_v2", "Primordium V2", "killicon_valorant_primordium_v2_emblem.png", "killicon_valorant_primordium_v2_bar.png", "killicon_valorant_primordium_frame.png", "killicon_valorant_base_ring.png", "#70C9F2", sliceSize: 152),
            LegacyPack("00029_primordium_v3", "Primordium V3", "killicon_valorant_primordium_v3_emblem.png", "killicon_valorant_primordium_v3_bar.png", "killicon_valorant_primordium_frame.png", "killicon_valorant_base_ring.png", "#F0D854", sliceSize: 152),
            LegacyPack("00030_radiant_crisis_001", "Radiant Crisis 001", "killicon_valorant_radiant_crisis_001_emblem.png", "killicon_valorant_radiant_crisis_001_bar.png", "killicon_valorant_base_frame.png", "killicon_valorant_base_ring.png", "#FFC10F", sliceSize: 135, headshotY: -19),
            LegacyPack("00031_rgx_11z_pro", "RGX 11z Pro", "killicon_valorant_rgx_11z_pro_emblem.png", "killicon_valorant_rgx_11z_pro_bar.png", "killicon_valorant_rgx_11z_pro_frame.png", "killicon_valorant_rgx_11z_pro_ring.png", "#A4FF96", sliceSize: 147, headshotX: 0.85, headshotY: -21),
            LegacyPack("00032_rgx_11z_pro_v1", "RGX 11z Pro V1", "killicon_valorant_rgx_11z_pro_v1_emblem.png", "killicon_valorant_rgx_11z_pro_v1_bar.png", "killicon_valorant_rgx_11z_pro_frame.png", "killicon_valorant_rgx_11z_pro_ring.png", "#A73437", sliceSize: 147, headshotX: 0.85, headshotY: -21),
            LegacyPack("00033_rgx_11z_pro_v2", "RGX 11z Pro V2", "killicon_valorant_rgx_11z_pro_v2_emblem.png", "killicon_valorant_rgx_11z_pro_v2_bar.png", "killicon_valorant_rgx_11z_pro_frame.png", "killicon_valorant_rgx_11z_pro_ring.png", "#184DD4", sliceSize: 147, headshotX: 0.85, headshotY: -21),
            LegacyPack("00034_rgx_11z_pro_v3", "RGX 11z Pro V3", "killicon_valorant_rgx_11z_pro_v3_emblem.png", "killicon_valorant_rgx_11z_pro_v3_bar.png", "killicon_valorant_rgx_11z_pro_frame.png", "killicon_valorant_rgx_11z_pro_ring.png", "#CE842B", sliceSize: 147, headshotX: 0.85, headshotY: -21)
        };

        private static readonly object PacksLock = new object();
        private static IReadOnlyList<ValorantPackInfo> _packs = BuiltInPacks;

        public static IReadOnlyList<ValorantPackInfo> All => _packs;

        public static void RefreshExternalPacks()
        {
            IReadOnlyList<ValorantPackInfo> discovered = ValorantExternalAssetService.DiscoverExternalPacks();
            var builtInKeys = new HashSet<string>(
                BuiltInPacks.Select(pack => pack.Key),
                StringComparer.OrdinalIgnoreCase);
            ValorantPackInfo[] combined = BuiltInPacks
                .Concat(discovered.Where(pack => !builtInKeys.Contains(pack.Key)))
                .ToArray();
            lock (PacksLock)
            {
                _packs = combined;
            }
        }

        public static bool IsValorantPackKey(string key)
        {
            return !string.IsNullOrWhiteSpace(key)
                && key.Trim().StartsWith("valorant_", StringComparison.OrdinalIgnoreCase);
        }

        public static ValorantPackInfo Find(string key)
        {
            return All.FirstOrDefault(pack =>
                string.Equals(pack.Key, key, StringComparison.OrdinalIgnoreCase));
        }

        public static string GetFolder(string key)
        {
            return Find(key)?.Folder;
        }

        public static string GetDisplayName(string key)
        {
            ValorantPackInfo pack = Find(key);
            if (pack == null)
            {
                return key;
            }

            string localized = LocalizationManager.Text(pack.Key);
            if (!string.Equals(localized, pack.Key, StringComparison.Ordinal))
            {
                return localized;
            }

            return pack.IsExternal
                && LocalizationManager.Current == UiLanguage.SimplifiedChinese
                && !string.IsNullOrWhiteSpace(pack.ChineseDisplayName)
                ? pack.ChineseDisplayName
                : pack.DisplayName;
        }

        public static string GetEmblemFile(string key)
        {
            return Find(key)?.EmblemFile;
        }

        public static string GetEmblemUri(string key)
        {
            ValorantPackInfo pack = Find(key);
            string externalUri = ValorantExternalAssetService.GetExternalEmblemUri(pack);
            if (!string.IsNullOrWhiteSpace(externalUri))
            {
                return externalUri;
            }

            return pack == null || string.IsNullOrWhiteSpace(pack.EmblemFile)
                ? null
                : $"ms-appx:///Assets/GameStyles/valorant/killconfirm/{pack.Folder}/textures/{pack.EmblemFile}";
        }

        public static int GetDisplayOrder(string key)
        {
            IReadOnlyList<ValorantPackInfo> packs = All;
            int index = -1;
            for (int candidate = 0; candidate < packs.Count; candidate++)
            {
                if (string.Equals(packs[candidate].Key, key, StringComparison.OrdinalIgnoreCase))
                {
                    index = candidate;
                    break;
                }
            }
            return index < 0 ? int.MaxValue : index;
        }

        private static ValorantPackInfo Pack(
            string keySuffix,
            string folder,
            string displayName,
            string emblemFile,
            bool hasBuiltInAudio = true)
        {
            return new ValorantPackInfo
            {
                Key = "valorant_" + keySuffix,
                Folder = folder,
                DisplayName = displayName,
                EmblemFile = emblemFile,
                HasBuiltInAudio = hasBuiltInAudio,
                AssociationId = "valorant:base"
            };
        }

        private static ValorantPackInfo LegacyPack(
            string folder,
            string displayName,
            string emblem,
            string bar,
            string frame,
            string ring,
            string accent,
            string blade = null,
            double sliceSize = 147,
            double headshotX = 0,
            double headshotY = -20)
        {
            return new ValorantPackInfo
            {
                Key = "valorant_" + folder,
                Folder = folder,
                DisplayName = displayName,
                EmblemFile = emblem,
                HasBuiltInAudio = true,
                IsExternal = false,
                AssociationId = "valorant:legacy:" + folder,
                Profile = new ValorantVisualProfileInfo
                {
                    Accent = accent,
                    Emblem = emblem,
                    Frame = frame,
                    Bar = bar,
                    BarHover = bar,
                    Ring = ring,
                    Blade = blade,
                    HeadshotX = headshotX,
                    HeadshotY = headshotY,
                    SliceSize = sliceSize
                    ,LegacyRendering = true
                }
            };
        }
    }
}
