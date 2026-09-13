using System;
using KillConfirmGameBar.Services;
using Windows.UI;

namespace KillConfirmGameBar.Controls
{
    public sealed partial class KillConfirmAnimation
    {
        private const double ValorantDemoVfxScale = 0.8075;
        private const double ValorantDemoFrameCssHeight = 116.0;

        private static ValorantDemoProfile GetValorantDemoProfile(string packKey)
        {
            ValorantVisualProfileInfo external = ValorantPackService.Find(packKey)?.Profile;
            if (external != null)
            {
                if (external.LegacyRendering)
                {
                    return GetLegacyValorantDemoProfile(packKey);
                }

                return new ValorantDemoProfile(
                    "external",
                    external.Accent,
                    external.Emblem,
                    external.Frame,
                    external.Bar,
                    external.BarHover)
                {
                    Ring = external.Ring,
                    FrameDissolve = external.FrameDissolve,
                    BadgeDissolve = external.BadgeDissolve,
                    Blade = external.Blade,
                    SpecialFrame = external.SpecialFrame,
                    HeadshotX = external.HeadshotX,
                    HeadshotY = external.HeadshotY,
                    SliceSize = external.SliceSize > 0 ? external.SliceSize : 147.0
                };
            }

            string id = ExtractValorantDemoId(packKey);
            switch (id)
            {
                case "00000":
                    return new ValorantDemoProfile(
                        id,
                        "#57F2D1",
                        "Base_Emblem.png",
                        null,
                        "Base_KillPip_Up.png",
                        "Base_KillPip_Hover.png");
                case "00010":
                    return new ValorantDemoProfile(id, "#68F5FF", "Cyberpunk_Emblem.png", "Cyberpunk_FrameBG.png", "Cyberpunk_KillPip_Hover.png", "Cyberpunk_KillPip_Up.png") { Ring = "Cyberpunk_RingBG.png", FrameDissolve = "Cyberpunk_FrameDissolve.png", BadgeDissolve = "Cyberpunk_BadgeDissolve.png", SliceSize = 172, HeadshotY = -10 };
                case "00011":
                    return EdgeProfile(id, "#F67A44", "V1");
                case "00012":
                    return EdgeProfile(id, "#D6B644", "V2");
                case "00013":
                    return EdgeProfile(id, "#436CBF", "V3");
                case "00014":
                    return AshenProfile(id, "#C01B1F", string.Empty);
                case "00015":
                    return AshenProfile(id, "#0871F7", "_v1");
                case "00016":
                    return AshenProfile(id, "#257133", "_v2");
                case "00017":
                    return AshenProfile(id, "#CB2C00", "_v3");
                case "00018":
                    return HazardProfile(id, "#AF00A3", "Standard", string.Empty, 0, 3.25);
                case "00019":
                    return HazardProfile(id, "#FFC359", "Yellow", "_v1", -1, -16.5);
                case "00020":
                    return HazardProfile(id, "#932B00", "Red", "_v2", 0.6, -4);
                case "00021":
                    return HazardProfile(id, "#06A600", "Green", "_v3", 8, 4);
                case "00022":
                    return new ValorantDemoProfile(id, "#C5B174", "Esports_Emblem.png", null, "Esports_KillPip_Up.png", "EsportsKillPip_Hover.png") { Ring = "Dragon_RingBG.png", SliceSize = 134 };
                case "00023":
                    return DemonStoneProfile(id, "#F35D45", "v1");
                case "00024":
                    return DemonStoneProfile(id, "#01BA01", "v2");
                case "00025":
                    return DemonStoneProfile(id, "#0D7DF5", "v3");
                case "00026":
                    return HellfireProfile(id, "#FE6D41", string.Empty);
                case "00027":
                    return HellfireProfile(id, "#84EAB6", "V1");
                case "00028":
                    return HellfireProfile(id, "#70C9F2", "V2");
                case "00029":
                    return HellfireProfile(id, "#F0D854", "V3");
                case "00030":
                    return new ValorantDemoProfile(id, "#FFC10F", "ComicBook_Emblem.png", "Dragon_FrameBG.png", "ComicBook_KillPip_Up.png", "ComicBook_KillPip_Hover.png") { Ring = "Dragon_RingBG.png", SliceSize = 135, HeadshotY = -19 };
                case "00031":
                    return AfterglowProfile(id, "#A4FF96", string.Empty);
                case "00032":
                    return AfterglowProfile(id, "#A73437", "v1");
                case "00033":
                    return AfterglowProfile(id, "#184DD4", "v2");
                case "00034":
                    return AfterglowProfile(id, "#CE842B", "v3");
                default:
                    // Unreal stores Prime's PrimaryColor as the linear value
                    // (1.0, 0.5, 0.0). FModel's #FFBC00 is its sRGB preview,
                    // not the multiplier passed to the particle widgets.
                    return new ValorantDemoProfile("00009", "#FF8000", "HypeBeast_Emblem.png", "HypeBeast__FrameBG.png", "HypeBeast_KillPip_Up.png", "HypeBeast_KillPip_Hover.png") { Ring = "HypeBeast_RingBG.png", FrameDissolve = "HypeBeast_FrameDissolve.png", BadgeDissolve = "HypeBeast_Emblem_Dissolve.png", SliceSize = 145, HeadshotY = -17 };
            }
        }

        private static ValorantDemoProfile GetLegacyValorantDemoProfile(string packKey)
        {
            string id = ExtractValorantDemoId(packKey);
            ValorantDemoProfile profile;
            switch (id)
            {
                case "00010": profile = new ValorantDemoProfile(id, "#2697f5", "killicon_valorant_glitchpop_emblem.png", "killicon_valorant_glitchpop_frame.png", "killicon_valorant_glitchpop_bar.png", "killicon_valorant_glitchpop_bar.png") { HeadshotY = -16, HeroFlame = false }; break;
                case "00011": profile = LegacySimple(id, "#df7e49", "singularity_v1", -10, 0.9, 0.8, false); break;
                case "00012": profile = LegacySimple(id, "#dcc971", "singularity_v2", -10, 0.9, 0.8, false); break;
                case "00013": profile = LegacySimple(id, "#7e9edc", "singularity_v3", -10, 0.9, 0.8, false); break;
                case "00014": profile = LegacyGaia(id, "#f9545e", "gaia"); break;
                case "00015": profile = LegacyGaia(id, "#287ef3", "gaia_v1"); break;
                case "00016": profile = LegacyGaia(id, "#27b748", "gaia_v2"); break;
                case "00017": profile = LegacyGaia(id, "#f77124", "gaia_v3"); break;
                case "00018": profile = LegacyBubblegum(id, "#c94fb9", "bubblegum_deathwish", "bubblegum_deathwish", -12); break;
                case "00019": profile = LegacyBubblegum(id, "#c98e4c", "bubblegum_deathwish_v3", "bubblegum_deathwish_v1", -12); break;
                case "00020": profile = LegacyBubblegum(id, "#9d332f", "bubblegum_deathwish_v2", "bubblegum_deathwish_v2", -12); break;
                case "00021": profile = LegacyBubblegum(id, "#6eb037", "bubblegum_deathwish_v1", "bubblegum_deathwish_v3", -12); break;
                case "00022": profile = LegacySimple(id, "#947046", "champions_2021", -12, 0.6, 0.8); break;
                case "00023": profile = LegacyChaos(id, "#f46e57", "prelude_to_chaos_v1"); break;
                case "00024": profile = LegacyChaos(id, "#10c110", "prelude_to_chaos_v2"); break;
                case "00025": profile = LegacyChaos(id, "#1168c1", "prelude_to_chaos_v3"); break;
                case "00026": profile = LegacyPrimordium(id, "#8f3e31", "primordium"); break;
                case "00027": profile = LegacyPrimordium(id, "#387a51", "primordium_v1"); break;
                case "00028": profile = LegacyPrimordium(id, "#316884", "primordium_v2"); break;
                case "00029": profile = LegacyPrimordium(id, "#8d6f43", "primordium_v3"); break;
                case "00030": profile = LegacySimple(id, "#73c0c4", "radiant_crisis_001", -12, 0.5, 0.8); profile.HaloRadius = 25; break;
                case "00031": profile = LegacyRgx(id, "#c1f341", "rgx_11z_pro"); break;
                case "00032": profile = LegacyRgx(id, "#f3414a", "rgx_11z_pro_v1"); break;
                case "00033": profile = LegacyRgx(id, "#41baf3", "rgx_11z_pro_v2"); break;
                case "00034": profile = LegacyRgx(id, "#f3a741", "rgx_11z_pro_v3"); break;
                default: profile = new ValorantDemoProfile("00009", "#908ccd", "killicon_valorant_prime_emblem.png", "killicon_valorant_prime_frame.png", "killicon_valorant_bar.png", "killicon_valorant_bar.png") { HeadshotY = -16, HeroFlame = false }; break;
            }
            profile.LegacyRendering = true;
            return profile;
        }

        private static ValorantDemoProfile LegacySimple(string id, string color, string name, double headshotY, double emblemScale, double frameScale, bool heroFlame = true)
        {
            return new ValorantDemoProfile(id, color, $"killicon_valorant_{name}_emblem.png", "killicon_valorant_base_frame.png", $"killicon_valorant_{name}_bar.png", $"killicon_valorant_{name}_bar.png")
            { HeadshotY = headshotY, HeroFlame = heroFlame, EmblemScale = emblemScale, FrameWidthScale = frameScale };
        }

        private static ValorantDemoProfile LegacyGaia(string id, string color, string name)
        {
            return new ValorantDemoProfile(id, color, $"killicon_valorant_{name}_emblem.png", $"killicon_valorant_{name}_frame.png", $"killicon_valorant_{name}_bar.png", $"killicon_valorant_{name}_bar.png")
            { HeadshotX = -2, HeadshotY = -20, EmblemScale = 0.9, BarRadiusOffset = 4, IsGaia = true };
        }

        private static ValorantDemoProfile LegacyBubblegum(string id, string color, string emblem, string bar, double headshotY)
        {
            return new ValorantDemoProfile(id, color, $"killicon_valorant_{emblem}_emblem.png", "killicon_valorant_bubblegum_deathwish_frame.png", $"killicon_valorant_{bar}_bar.png", $"killicon_valorant_{bar}_bar.png")
            { Blade = "killicon_valorant_bubblegum_deathwish_blade.png", HeadshotY = headshotY, EmblemScale = 0.55 };
        }

        private static ValorantDemoProfile LegacyChaos(string id, string color, string name)
        {
            ValorantDemoProfile profile = LegacySimple(id, color, name, -12, 0.5, 0.8);
            profile.BarRadiusOffset = 9;
            return profile;
        }

        private static ValorantDemoProfile LegacyPrimordium(string id, string color, string name)
        {
            return new ValorantDemoProfile(id, color, $"killicon_valorant_{name}_emblem.png", "killicon_valorant_primordium_frame.png", $"killicon_valorant_{name}_bar.png", $"killicon_valorant_{name}_bar.png")
            { HeadshotY = -14, EmblemScale = 0.4, HaloRadius = 25 };
        }

        private static ValorantDemoProfile LegacyRgx(string id, string color, string name)
        {
            return new ValorantDemoProfile(id, color, $"killicon_valorant_{name}_emblem.png", $"killicon_valorant_{name}_frame.png", $"killicon_valorant_{name}_bar.png", $"killicon_valorant_{name}_bar.png")
            { HeadshotY = -12, EmblemScale = 0.35, FrameWidthScale = 0.8, HaloRadius = 25 };
        }

        private static ValorantDemoProfile EdgeProfile(string id, string color, string variant)
        {
            return new ValorantDemoProfile(id, color, $"Edge_Emblem{variant}.png", "Dragon_FrameBG.png", $"Edge_KillPip_Up{variant}.png", $"Edge_KillPip_Hover{variant}.png") { Ring = "FantasySovereign_RingBG.png", FrameDissolve = "Dragon_FrameDissolve.png", BadgeDissolve = "Cyberpunk_BadgeDissolve.png", SliceSize = 140, HeadshotY = -10 };
        }

        private static ValorantDemoProfile AshenProfile(string id, string color, string suffix)
        {
            return new ValorantDemoProfile(id, color, $"Ashen_Emblem{suffix}.png", "Ashen_FrameBG.png", $"Ashen_KillPip{suffix}_Up.png", $"Ashen_KillPip{suffix}_Hover.png") { Ring = "Dragon_RingBG.png", HeadshotX = -2, HeadshotY = -20 };
        }

        private static ValorantDemoProfile HazardProfile(string id, string color, string emblem, string suffix, double headshotX, double headshotY)
        {
            return new ValorantDemoProfile(id, color, $"Hazard_Emblem_{emblem}.png", "hazard_blank.png", $"Hazard_KillPip_Up{suffix}.png", $"Hazard_KillPip_Hover{suffix}.png") { Ring = "hazard_blank_ring.png", SpecialFrame = "Hazard_Frame_BG.png", Blade = "Hazard_Frame_Blade.png", SliceSize = 170, HeadshotX = headshotX, HeadshotY = headshotY };
        }

        private static ValorantDemoProfile DemonStoneProfile(string id, string color, string variant)
        {
            return new ValorantDemoProfile(id, color, $"Demonstone_Emblem_{variant}.png", "Demonstone_FrameBG.png", $"Demonstone_KillPip_Up_{variant}.png", $"Demonstone_KillPip_Hover_{variant}.png") { FrameDissolve = "Demonstone_FrameDissolve.png", SliceSize = 170, HeadshotY = -18 };
        }

        private static ValorantDemoProfile HellfireProfile(string id, string color, string variant)
        {
            string suffix = string.IsNullOrEmpty(variant) ? string.Empty : "_" + variant;
            return new ValorantDemoProfile(id, color, $"Hellfire_Emblem{suffix}.png", "HellFire_Frame.png", $"HellFire_KillPip_Up{suffix}.png", $"HellFire_KillPip_Hover{suffix}.png") { SliceSize = 152 };
        }

        private static ValorantDemoProfile AfterglowProfile(string id, string color, string variant)
        {
            string emblem = string.IsNullOrEmpty(variant) ? "Afterglow_Emblem.png" : $"Afterglow_Emblem_{variant}.png";
            string up = string.IsNullOrEmpty(variant) ? "Afterglow_KillPip_Up.png" : $"Afterglow_KillPip__{variant}_Up.png";
            string hover = string.IsNullOrEmpty(variant) ? "Afterglow_KillPip_Hover.png" : $"Afterglow_KillPip_{variant}_Hover.png";
            return new ValorantDemoProfile(id, color, emblem, "Afterglow_FrameBG.png", up, hover) { Ring = "Afterglow_RingBG.png", FrameDissolve = "Afterglow_FrameDissolve.png", BadgeDissolve = "Afterglow_Badge_Dissolve.png", SliceSize = 147, HeadshotX = 0.85, HeadshotY = -21 };
        }

        private static string ExtractValorantDemoId(string packKey)
        {
            if (string.IsNullOrWhiteSpace(packKey))
            {
                return "00009";
            }

            int marker = packKey.IndexOf("valorant_", StringComparison.OrdinalIgnoreCase);
            int start = marker >= 0 ? marker + "valorant_".Length : 0;
            return packKey.Length >= start + 5 ? packKey.Substring(start, 5) : "00009";
        }

        private sealed class ValorantDemoProfile
        {
            public ValorantDemoProfile(string id, string accentHex, string emblem, string frame, string bar, string barHover)
            {
                Id = id;
                Accent = ParseValorantColor(accentHex);
                Emblem = emblem;
                Frame = frame;
                Bar = bar;
                BarHover = barHover;
            }

            public string Id { get; }
            public Color Accent { get; }
            public string Emblem { get; }
            public string Frame { get; }
            public string Bar { get; }
            public string BarHover { get; }
            public string Ring { get; set; }
            public string FrameDissolve { get; set; }
            public string BadgeDissolve { get; set; }
            public string Blade { get; set; }
            public string SpecialFrame { get; set; }
            public double HeadshotX { get; set; }
            public double HeadshotY { get; set; }
            public double SliceSize { get; set; } = 147.0;
            public bool LegacyRendering { get; set; }
            public bool HeroFlame { get; set; } = true;
            public bool IsGaia { get; set; }
            public double EmblemScale { get; set; } = 1.0;
            public double FrameWidthScale { get; set; } = 1.0;
            public double BarRadiusOffset { get; set; }
            public double BaseParticleYOffset { get; set; } = 45.0;
            public double BaseParticleScale { get; set; } = 1.0;
            public double LargeSparksScale { get; set; } = 1.0;
            public double HaloRadius { get; set; } = 30.0;
        }

        private static Color ParseValorantColor(string hex)
        {
            return Color.FromArgb(
                255,
                Convert.ToByte(hex.Substring(1, 2), 16),
                Convert.ToByte(hex.Substring(3, 2), 16),
                Convert.ToByte(hex.Substring(5, 2), 16));
        }
    }
}
