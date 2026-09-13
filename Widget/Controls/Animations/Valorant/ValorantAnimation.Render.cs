using Microsoft.Graphics.Canvas;

namespace KillConfirmGameBar.Controls
{
    public sealed partial class KillConfirmAnimation
    {
        private void DrawValorantKillFrame(CanvasDrawingSession drawingSession, int frame)
        {
            ValorantKillAsset asset = _currentValorantAsset;
            if (asset == null)
            {
                return;
            }

            if (asset.DemoProfile?.LegacyRendering == true)
            {
                DrawLegacyValorantKillFrame(drawingSession, frame, asset);
                return;
            }

            DrawNativeValorantFrame(drawingSession, frame, asset);
        }

    }
}
