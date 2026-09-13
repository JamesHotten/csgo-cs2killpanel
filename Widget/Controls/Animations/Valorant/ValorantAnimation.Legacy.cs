using System;
using System.Numerics;
using Microsoft.Graphics.Canvas;
using Windows.Foundation;
using Windows.UI;

namespace KillConfirmGameBar.Controls
{
    public sealed partial class KillConfirmAnimation
    {
        private const double LegacyValorantDurationMs = 2600.0;
        private const double LegacyValorantFadeStartMs = 2380.0;
        private const double LegacyValorantFivePlusShowMs = 300.0;
        private const double LegacyValorantFrameCssWidth = 116.0;
        private const double LegacyValorantEmblemCssSize = 104.0;
        private const double LegacyValorantBladeCssSize = 70.0;
        private const double LegacyValorantHeadshotCssSize = 19.0;
        private static readonly Color LegacyValorantFlashColor = Color.FromArgb(255, 255, 42, 54);

        private static readonly int[][] LegacyValorantBarAngles =
        {
            new[] { 0 },
            new[] { -90, 90 },
            new[] { 0, -120, 120 },
            new[] { 0, -90, 90, 180 },
            new[] { 0, -72, 72, -144, 144 },
            new[] { 0, -60, 60, -120, 120, 180 }
        };

        private void DrawLegacyValorantKillFrame(CanvasDrawingSession ds, int frame, ValorantKillAsset asset)
        {
            double elapsedMs = frame * (1000.0 / FrameSequenceFps);
            double opacity = GetLegacyValorantLifeOpacity(asset.KillCount, elapsedMs);
            if (opacity <= 0)
            {
                return;
            }

            double cx = ValorantFrameWidth / 2.0;
            double cy = ValorantFrameHeight / 2.0;
            ValorantDemoProfile profile = asset.DemoProfile;

            DrawLegacyValorantParticle(ds, asset.BaseParticle, 49, 25, elapsedMs, cx, cy,
                112 * profile.BaseParticleScale, 112 * profile.BaseParticleScale,
                0, 206.0 / 256.0, 0, profile.BaseParticleYOffset, opacity, true, false, asset.Accent);
            DrawValorantHalo(ds, cx, cy, asset.Accent, profile.HaloRadius, elapsedMs, opacity);
            DrawLegacyValorantBars(ds, asset, cx, cy, elapsedMs, opacity);

            DrawCenteredImageWithShadowAt(ds, asset.Frame, cx, cy,
                LegacyValorantFrameCssWidth * profile.FrameWidthScale * ValorantDemoVfxScale,
                ValorantDemoFrameCssHeight * ValorantDemoVfxScale, 1, opacity,
                Colors.Black, 12, 0, 0, 0.62, asset.Brightness, asset.Contrast);

            if (asset.Blade != null)
            {
                double bladeSpin = ResolveLegacyValorantBladeRotation(profile, asset.SpinDirection, elapsedMs);
                DrawRotatedCenteredImageWithShadowAt(ds, asset.Blade, cx, cy,
                    LegacyValorantBladeCssSize * ValorantDemoVfxScale,
                    LegacyValorantBladeCssSize * ValorantDemoVfxScale,
                    1, bladeSpin, opacity, Colors.Black, 12, 0, 0, 0.62,
                    asset.Brightness, asset.Contrast);
            }

            double emblemY = cy + GetLegacyValorantEmblemYOffset(elapsedMs) * ValorantDemoVfxScale;
            double emblemSize = LegacyValorantEmblemCssSize * profile.EmblemScale * ValorantDemoVfxScale;
            DrawCenteredImageWithShadowAt(ds, asset.Emblem, cx, emblemY, emblemSize, emblemSize,
                1, opacity, Colors.Black, 12, 0, 0, 0.62, asset.Brightness, asset.Contrast);
            double flashOpacity = GetLegacyValorantFlashOpacity(elapsedMs) * opacity;
            if (flashOpacity > 0)
            {
                DrawCenteredFlashImageAt(ds, asset.Emblem, cx, cy, emblemSize, emblemSize,
                    1, flashOpacity, LegacyValorantFlashColor, asset.Brightness, asset.Contrast);
            }

            if (asset.IsHeadshot)
            {
                double headshotScale = Lerp(1.8, 1.0,
                    LegacyCubicBezierEase(Clamp01(elapsedMs / 250.0), 0.22, 0.9, 0.28, 1));
                Color headshotGlow = elapsedMs <= 250.0
                    ? LerpColor(Colors.White, LegacyValorantFlashColor, Clamp01(elapsedMs / 250.0))
                    : LegacyValorantFlashColor;
                DrawCenteredImageWithShadowAt(ds, asset.Headshot,
                    cx + profile.HeadshotX * ValorantDemoVfxScale,
                    cy + profile.HeadshotY * ValorantDemoVfxScale,
                    LegacyValorantHeadshotCssSize * ValorantDemoVfxScale,
                    LegacyValorantHeadshotCssSize * ValorantDemoVfxScale,
                    headshotScale, opacity, headshotGlow, 8, 0, 0, 0.9);
            }

            if (asset.HeroFlame != null && profile.HeroFlame)
            {
                DrawLegacyValorantParticle(ds, asset.HeroFlame, 20, 29, elapsedMs, cx, cy,
                    96, 108, 0.4934375, 0.49079242, 0, -30, 0.7 * opacity,
                    false, false, asset.Accent);
            }
            if (asset.KillCount >= 5)
            {
                DrawLegacyValorantParticle(ds, asset.LargeSparks, 52, 25, elapsedMs, cx, cy,
                    105 * profile.LargeSparksScale, 105 * profile.LargeSparksScale,
                    0.49052733, 0.5185547, 0, 2, 0.82 * opacity, false, true, asset.Accent);
            }
            if (asset.IsHeadshot)
            {
                DrawLegacyValorantParticle(ds, asset.XSparks, 29, 25, elapsedMs, cx, cy,
                    56, 170, 0.48015872, 0.56722003, 0, -8, 0.9 * opacity,
                    false, false, LegacyValorantFlashColor);
            }
        }

        private void DrawLegacyValorantBars(CanvasDrawingSession ds, ValorantKillAsset asset,
            double cx, double cy, double elapsedMs, double opacity)
        {
            if (asset.Bar == null)
            {
                return;
            }

            ValorantDemoProfile profile = asset.DemoProfile;
            int[] angles = LegacyValorantBarAngles[Math.Max(0, Math.Min(5, asset.KillCount - 1))];
            double spinDuration = asset.KillCount >= 5 ? 1000.0 : 700.0;
            double spin = (asset.KillCount >= 5 ? 360.0 : 180.0)
                * LegacyCubicBezierEase(Clamp01((elapsedMs - 750.0) / spinDuration), 0.22, 0.9, 0.28, 1);
            double baseDistance = (36.0 + profile.BarRadiusOffset) * ValorantDemoVfxScale;
            double distance = GetLegacyValorantBarDistance(elapsedMs, baseDistance);
            double scale = GetLegacyValorantBarScale(elapsedMs);
            foreach (int angle in angles)
            {
                double radians = (angle + spin) * Math.PI / 180.0;
                double x = cx + Math.Sin(radians) * distance;
                double y = cy - Math.Cos(radians) * distance;
                DrawRotatedCenteredImageWithShadowAt(ds, asset.Bar, x, y,
                    32 * ValorantDemoVfxScale, 32 * ValorantDemoVfxScale,
                    scale, angle + spin, opacity, asset.Accent, 5, 0, 0, 0.75,
                    asset.Brightness, asset.Contrast);
            }
        }

        private void DrawLegacyValorantParticle(CanvasDrawingSession ds, CanvasBitmap image,
            int frameCount, int intervalMs, double elapsedMs, double cx, double cy,
            double width, double height, double anchorX, double anchorY,
            double offsetX, double offsetY, double opacity, bool mirrored, bool additive, Color tint)
        {
            if (image == null || opacity <= 0)
            {
                return;
            }

            CanvasBlend previousBlend = ds.Blend;
            if (additive)
            {
                ds.Blend = CanvasBlend.Add;
            }
            int spriteFrame = intervalMs <= 0 ? 0 : (int)Math.Floor(elapsedMs / intervalMs);
            spriteFrame = Math.Max(0, Math.Min(frameCount - 1, spriteFrame));
            double frameHeight = image.SizeInPixels.Height / (double)frameCount;
            var source = new Rect(0, spriteFrame * frameHeight, image.SizeInPixels.Width, frameHeight);
            double scaledWidth = width * ValorantDemoVfxScale;
            double scaledHeight = height * ValorantDemoVfxScale;
            var target = new Rect(
                cx + offsetX * ValorantDemoVfxScale - anchorX * scaledWidth,
                cy + offsetY * ValorantDemoVfxScale - anchorY * scaledHeight,
                scaledWidth, scaledHeight);
            DrawImageWithOptionalGlow(ds, image, target, source, opacity, tint, null, 0, 0);
            if (mirrored)
            {
                Matrix3x2 previous = ds.Transform;
                ds.Transform = Matrix3x2.CreateScale(-1, 1, new Vector2((float)cx, (float)cy)) * previous;
                DrawImageWithOptionalGlow(ds, image, target, source, opacity, tint, null, 0, 0);
                ds.Transform = previous;
            }
            ds.Blend = previousBlend;
        }

        private static int NextValorantSpinDirection()
        {
            lock (ValorantSpinRandomLock)
            {
                return ValorantSpinRandom.Next(0, 2) == 0 ? -1 : 1;
            }
        }

        private static double ResolveLegacyValorantBladeRotation(ValorantDemoProfile profile, int spinDirection, double elapsedMs)
        {
            double speed = profile != null && profile.IsGaia ? 1100.0 : 1350.0;
            const double windowSeconds = 1.5;
            double t = Clamp01(elapsedMs / (windowSeconds * 1000.0));
            double eased = 1.0 - Math.Pow(1.0 - t, 4.0);
            return -(speed * windowSeconds * eased * 0.25 * (spinDirection < 0 ? -1 : 1));
        }

        private static double GetLegacyValorantLifeOpacity(int killCount, double elapsedMs)
        {
            if (killCount >= 5 && elapsedMs < LegacyValorantFivePlusShowMs)
                return Clamp01(elapsedMs / LegacyValorantFivePlusShowMs);
            if (elapsedMs <= LegacyValorantFadeStartMs)
                return 1;
            return 1.0 - Clamp01((elapsedMs - LegacyValorantFadeStartMs)
                / (LegacyValorantDurationMs - LegacyValorantFadeStartMs));
        }

        private static double GetLegacyValorantEmblemYOffset(double elapsedMs) =>
            Lerp(-16.0, 0.0, LegacyCubicBezierEase(Clamp01(elapsedMs / 100.0), 0.22, 0.9, 0.28, 1));

        private static double GetLegacyValorantFlashOpacity(double elapsedMs)
        {
            const double duration = 620.0;
            if (elapsedMs <= 0 || elapsedMs >= duration) return 0;
            double percent = Clamp01(elapsedMs / duration) * 100.0;
            double[] keys = { 0, 1.35, 12.11, 13.46, 14.81, 25.57, 26.92, 28.27, 39.03, 40.38, 41.73, 52.5, 53, 100 };
            double[] values = { 0, 0.9, 0.9, 0, 0.9, 0.9, 0, 0.9, 0.9, 0, 0.9, 0.9, 0, 0 };
            for (int i = 1; i < keys.Length; i++)
            {
                if (percent <= keys[i])
                {
                    double local = (percent - keys[i - 1]) / Math.Max(0.0001, keys[i] - keys[i - 1]);
                    return Lerp(values[i - 1], values[i], local);
                }
            }
            return 0;
        }

        private static double GetLegacyValorantBarDistance(double elapsedMs, double baseDistance)
        {
            double percent = Clamp01(elapsedMs / 620.0) * 100.0;
            double extra = 9.0 * ValorantDemoVfxScale;
            if (percent <= 3.22) return baseDistance;
            if (percent <= 29.03) return Lerp(baseDistance, baseDistance + extra,
                LegacyCubicBezierEase((percent - 3.22) / 25.81, 0.22, 0.9, 0.28, 1));
            if (percent <= 43.54) return baseDistance + extra;
            return Lerp(baseDistance + extra, baseDistance,
                LegacyCubicBezierEase((percent - 43.54) / 56.46, 0.22, 0.9, 0.28, 1));
        }

        private static double GetLegacyValorantBarScale(double elapsedMs)
        {
            double percent = Clamp01(elapsedMs / 620.0) * 100.0;
            if (percent <= 3.22) return 1.6;
            if (percent <= 29.03) return Lerp(1.6, 1.0,
                LegacyCubicBezierEase((percent - 3.22) / 25.81, 0.22, 0.9, 0.28, 1));
            return 1.0;
        }

        private static double LegacyCubicBezierEase(double value, double x1, double y1, double x2, double y2)
        {
            value = Clamp01(value);
            double low = 0, high = 1, t = value;
            for (int i = 0; i < 8; i++)
            {
                t = (low + high) / 2.0;
                if (LegacyCubicBezier(t, 0, x1, x2, 1) < value) low = t; else high = t;
            }
            return LegacyCubicBezier(t, 0, y1, y2, 1);
        }

        private static double LegacyCubicBezier(double t, double p0, double p1, double p2, double p3)
        {
            double u = 1.0 - t;
            return u * u * u * p0 + 3 * u * u * t * p1 + 3 * u * t * t * p2 + t * t * t * p3;
        }
    }
}
