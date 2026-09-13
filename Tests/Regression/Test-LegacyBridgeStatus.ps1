$ErrorActionPreference = 'Stop'
$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$bridge = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'KillConfirmService/src/infrastructure/legacy_bridge.rs')
$response = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'KillConfirmService/src/api/requests.rs')
$endpoint = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'KillConfirmService/src/api/core_endpoints.rs')
$monitor = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'Widget/Services/Runtime/GsiStatusMonitor.cs')
$cs2Bridge = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'KillConfirmService/src/infrastructure/cs2_local_bridge.rs')

if ($bridge -match 'tails\.is_empty\(\)\s*&&\s*last_discovery') {
    throw 'Legacy bridge only discovers configured SourceMod logs once and misses paths added while running.'
}
foreach ($token in @('legacy_bridge_connected', 'legacy_bridge_events', 'last_legacy_bridge_activity_unix_ms')) {
    if ($response -notmatch $token -or $endpoint -notmatch $token) {
        throw "Legacy bridge telemetry is not exposed by /gsi-status: $token"
    }
}
if ($monitor -notmatch 'GsiGameVersionSettingsStore\.CsgoLegacy' -or
    $monitor -notmatch 'legacy_bridge_connected') {
    throw 'The status light does not use the original SourceMod Legacy bridge when Legacy is selected.'
}
foreach ($token in @('cs2_local_bridge_connected', 'cs2_local_bridge_events', 'last_cs2_local_bridge_activity_unix_ms')) {
    if ($response -notmatch $token -or $endpoint -notmatch $token -or $cs2Bridge -notmatch $token) {
        throw "CS2 controlled-bot bridge telemetry is missing: $token"
    }
}
if ($monitor -notmatch '!legacySelected\s*&&\s*cs2LocalBridgeConnected') {
    throw 'The CS2 status light does not accept the original controlled-bot local log bridge.'
}

'PASS: CS2 controlled-bot and Legacy SourceMod bridges are represented independently from normal GSI.'
