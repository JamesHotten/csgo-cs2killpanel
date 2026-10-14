#Requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$source = Get-Content -Raw -LiteralPath (Join-Path $root 'Widget/Danmaku/Engine/DanmakuLaneLayout.cs')
$support = @'
namespace KillConfirmGameBar.Danmaku {
    internal enum DanmakuDisplayArea { Top, Bottom, Center, AvoidCenter, All }
}
namespace KillConfirmGameBar.Danmaku.Engine {
    internal static class DanmakuReactionPolicies { internal const int EventMaximumVisibleCount = 9; }
}
'@
$checks = @'
namespace KillConfirmGameBar.Danmaku.Engine {
    public static class DanmakuLaneLayoutChecks {
        public static void Run() {
            foreach (KillConfirmGameBar.Danmaku.DanmakuDisplayArea area in
                System.Enum.GetValues(typeof(KillConfirmGameBar.Danmaku.DanmakuDisplayArea))) {
                foreach (double height in new double[] { 120, 240, 600, 1080 }) {
                    var lanes = DanmakuLaneLayout.Build(area, height, 9, 28);
                    if (lanes.Count < 1 || lanes.Count > 9) throw new System.Exception("Invalid lane count");
                    for (int i = 1; i < lanes.Count; i++) {
                        if (lanes[i] - lanes[i - 1] < 35)
                            throw new System.Exception("Overlapping lanes: " + area + " at " + height);
                    }
                }
            }
        }
    }
}
'@
Add-Type -TypeDefinition ($source + $support + $checks) -ErrorAction Stop
[KillConfirmGameBar.Danmaku.Engine.DanmakuLaneLayoutChecks]::Run()
Write-Output 'PASS: all danmaku areas keep safe lane spacing at compact and full-screen heights.'
