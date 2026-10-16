#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$folder = Join-Path $root 'Widget/Controls/Animations/Overwatch'
# Execute production playback with a deliberately delayed bitmap loader. Only
# Win2D/XAML and unrelated style renderers are replaced by platform stubs.
$sources = @('OverwatchAnimation.Data.cs', 'OverwatchAnimation.Playback.cs') | ForEach-Object {
    (Get-Content -Raw -LiteralPath (Join-Path $folder $_)) -replace '(?m)^using .*;\r?\n', ''
}
$cards = Get-Content -Raw -LiteralPath (Join-Path $folder 'OverwatchAnimation.Cards.cs')
$start = $cards.IndexOf('        private void AddOverwatchFeedItem(')
$end = $cards.IndexOf('        private void UpdateOverwatchCardSelectionBounds()', $start)
if ($start -lt 0 -or $end -lt 0) { throw 'Cannot locate production feed insertion method.' }
$feedMethod = $cards.Substring($start, $end - $start)
$header = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Graphics.Canvas;
using Windows.UI.Xaml;
'@
$stubs = @'
namespace Microsoft.Graphics.Canvas { public sealed class CanvasBitmap {} }
namespace Windows.UI.Xaml { public enum Visibility { Visible, Collapsed } }
namespace KillConfirmGameBar { public static class App { public static void Log(string text) {} } }
namespace KillConfirmGameBar.Controls {
    public sealed partial class KillConfirmAnimation {
        private static readonly SemaphoreSlim PreloadGate = new SemaphoreSlim(1, 1);
        private int _resourceGeneration = 0, _playToken = 0;
        private const double FrameSequenceFps = 30;
        private readonly Stopwatch _playbackClock = new Stopwatch();
        private readonly FakeCanvas SpriteCanvas = new FakeCanvas();
        private readonly FakeTimer _timer = new FakeTimer();
        private bool _isBattlefieldTextOverlayActive, _contentSizedViewport, _isBattlefield1CompactLayoutActive;
        private object _currentCodeAsset, _currentValorantAsset, _currentBattlefieldAsset, _currentCsolAsset;
        private SpriteMetadata _currentMetadata;
        private Visibility Visibility;
        private event EventHandler LogicalViewportSizeChanged;
        private TaskCompletionSource<CanvasBitmap> _pendingIcon;
        private sealed class FakeCanvas { public void Invalidate() {} }
        private sealed class FakeTimer { public TimeSpan Interval; public void Start() {} public void Stop() {} }
        private sealed class SpriteMetadata { public int FrameWidth, FrameHeight, Frames, Fps; }
        private async Task<CanvasBitmap> LoadOverwatchKillIconBitmapAsync() {
            _overwatchKillIconBitmap = await _pendingIcon.Task;
            return _overwatchKillIconBitmap;
        }
        private Task<CanvasBitmap> LoadOverwatchEffectSheetBitmapAsync() { return Task.FromResult(new CanvasBitmap()); }
        private static string NormalizeOverwatchTargetName(string text) { return (text ?? "enemy").Trim(); }
        private void ReleaseAllAnimationResourceCaches() {}
        private void ApplyViewportSize(double width, double height) {}
        private void HideLoadingProgress() {}
        private void UpdateOverwatchCardSelectionBounds() {}
        private void ResetBattlefield5ScrollingState() {}
        private void ResetBattlefield4HudState() {}
        private void ResetBattlefield2042HudState() {}
        private void ResetPubgHudState() {}
        private void ResetDeltaForceHudState() {}
        private void ResetDoubaoState() {}
        private void ResetDagoujiaoState() {}
        private void ResetModernWarfare2019State() {}
        private void ResetApexFeedState() {}
        private static void Check(bool condition, string message) { if (!condition) throw new Exception(message); }
        public static void RunPreviewChecks() {
            var view = new KillConfirmAnimation { _pendingIcon = new TaskCompletionSource<CanvasBitmap>() };
            _overwatchKillIconBitmap = null;
            for (int i = 1; i <= 8; i++) view.PlayOverwatchLowerThirdKill("enemy" + i, i % 2 == 0);
            view._pendingIcon.SetResult(new CanvasBitmap());
            Check(view._overwatchFeedItems.Count == 5, "Cold-load burst must retain five cards");
            for (int i = 0; i < 5; i++) {
                Check(view._overwatchFeedItems[i].TargetName == "enemy" + (i + 4), "Must retain newest events");
                Check(view._overwatchFeedItems[i].IsAssist == ((i + 4) % 2 == 0), "Assist flag must follow its event");
            }
            view.PlayOverwatchLowerThirdKill("newest");
            Check(view._overwatchFeedItems[4].TargetName == "newest", "Repeated active preview must append");
            view.ResetOverwatchState(); view._playbackClock.Stop();
            view.PlayOverwatchLowerThirdKill("replay");
            Check(view._isOverwatchActive && view._overwatchFeedItems.Count == 1
                && view._overwatchFeedItems[0].TargetName == "replay", "Preview must restart after reset");
            var stale = new KillConfirmAnimation { _pendingIcon = new TaskCompletionSource<CanvasBitmap>() };
            stale.PlayOverwatchLowerThirdKill("stale1");
            stale.PlayOverwatchLowerThirdKill("stale2");
            stale.ResetOverwatchState(); stale._resourceGeneration++; stale._playToken++;
            stale._pendingIcon.SetResult(new CanvasBitmap());
            Check(!stale._isOverwatchActive && stale._overwatchPendingCards.Count == 0
                && stale._overwatchFeedItems.Count == 0, "Cancelled resource load must not revive old cards");
        }
    }
}
'@
$code = $header + "`n" + ($sources -join "`n") + "`n" + $stubs +
    "`nnamespace KillConfirmGameBar.Controls { public sealed partial class KillConfirmAnimation {`n" + $feedMethod + "`n} }"
Add-Type -TypeDefinition $code -CompilerOptions '/nowarn:0067,0169,0414,0649' -ErrorAction Stop
[KillConfirmGameBar.Controls.KillConfirmAnimation]::RunPreviewChecks()
Write-Output 'PASS: Overwatch cold-load bursts retain newest cards and assist flags; active replay, reset and stale-load cancellation verified.'
