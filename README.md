# KillConfirmGameBar

[简体中文](README.zh-CN.md) | English

KillConfirmGameBar is an Xbox Game Bar kill-confirm overlay for Counter-Strike 2 and CS:GO Legacy on Windows.

It receives match state from the game, plays a matching sound, and renders an animated confirmation over the game. CS2 and Legacy share one widget and one local service while keeping their game-specific compatibility paths isolated.

This repository is the continued development fork at [JamesHotten/csgo-cs2killpanel](https://github.com/JamesHotten/csgo-cs2killpanel), based on [eachkinji/CS2KillConfirmOverlay](https://github.com/eachkinji/CS2KillConfirmOverlay).

## Highlights

- Xbox Game Bar overlay with configurable position, scale, audio volume, and output device.
- Normal kill, headshot, knife kill, first kill, final kill, round win, and round loss events.
- CrossFire, Valorant, CSOL, Battlefield 1/4/5/2042, PUBG, Delta Force, Overwatch, Modern Warfare 2019, Apex, Doubao, Dagoujiao, and Custom Module presentation styles.
- Multiple CrossFire voices and Valorant weapon-finisher sound packs.
- Optional assist audio for CrossFire and Valorant, plus separate CF headshot/knife audio and icon priority controls.
- CSOL 1–10 kill visuals, voice variants, special-event priority, and first/final-kill icon settings.
- Per-event sound routing for supported Battlefield and Delta Force styles without changing visual event flags.
- Automatic or fixed 100%–200% Game Bar control-panel scaling for high-resolution displays.
- CS2, CS:GO Legacy, original bots, controlled bots, spectating, replay, and supported bot mods.
- Per-observed-player kill and assist baselines prevent counters from leaking across spectator or replay target changes. Teammate spectating, replay views, and controlled-bot effects have separate switches.
- Versioned JSON settings can be exported, imported on another PC, and migrated from older schemas.
- Custom Module imports CS2 Customizer-compatible sprite atlases, image sequences, ZIPs, and MP4/MOV/WebM/MKV/AVI video. Missing levels fall back to built-in visual and audio feedback.
- The CFG panel lists every detected CS2 and Legacy installation with its individual install status.
- Weapon attribution is captured at kill time, so switching weapons immediately after a kill does not relabel it.
- Duplicate-event suppression and round-end ordering prevent a kill from being replayed after the victory effect.
- CS-style cash rewards for supported modes, including weapon, objective, win, loss, and loss-streak awards.

Special assets depend on the selected pack. When a pack has no dedicated variant for an event, the overlay uses its compatible fallback instead of intentionally producing an empty sound or animation.

## Compatibility

| Scenario | Normal kills | Controlled-bot kills | Extra server component |
| --- | --- | --- | --- |
| CS2 official/community server | Yes, through client GSI | Server-dependent; no remote log bridge | None |
| CS2 local listen server | Yes | Yes, with the installed local logging config | None |
| CS:GO Legacy local/offline | Yes, through client GSI | Yes, through Legacy Bridge | SourceMod |
| CS:GO Legacy server on this PC | Yes | Yes, through server bridge logs | SourceMod on the server |
| Remote Legacy server | Yes through client GSI | Only if its bridge log is securely made visible locally | SourceMod on the server |

CS2 compatibility does not use the Legacy identity cache or SourceMod deduplication path. Install the Legacy Bridge only in CS:GO Legacy or a Legacy server.

Only the event represented by the main game view is rendered. This covers normal play, spectating, replay views, and supported bot takeovers without playing effects for unrelated players.

For exact client and server deployment steps, see [CS2 and CS:GO Legacy migration guide](docs/CS2_AND_LEGACY_MIGRATION.zh-CN.md) (Chinese).
For every animation, audio, priority, streak-window, and fallback option, see the [effect and audio settings guide](docs/EFFECT_AUDIO_SETTINGS.zh-CN.md) (Chinese).

## Requirements

- Windows 10 or Windows 11
- Xbox Game Bar enabled
- Counter-Strike 2 and/or CS:GO Legacy

Building from source additionally requires:

- Rust toolchain
- Visual Studio or Visual Studio Build Tools with UWP, MSIX, Windows SDK, and C++ components
- Inno Setup 6 when building the optional `.exe` installer
- A compatible SourceMod compiler when rebuilding the Legacy Bridge

## Install

Use a packaged release from this repository. Run its installer, or run the installation script contained in the transfer package.

After installation:

1. Press `Win + G`.
2. Open and optionally pin the Kill Confirm Overlay widget.
3. Select a presentation style, sound pack, and audio output.
4. Start the game after its GSI configuration has been installed.

Do not copy an extracted `InstalledPackage` directory to another PC. The Game Bar package must be registered, and its loopback exemption must be created by the installer or transfer script.

## Game State Integration

The companion service listens only on:

```text
http://127.0.0.1:10087/
```

The installer and local service can place `gamestate_integration_killconfirm.cfg` automatically. The folder picker accepts the Steam root, `steamapps`, `common`, the game root, `game`, `csgo`, or `cfg`. Manual locations are:

```text
<CS2 root>\game\csgo\cfg\gamestate_integration_killconfirm.cfg
<Legacy root>\csgo\cfg\gamestate_integration_killconfirm.cfg
```

Completely restart the game after adding or changing a GSI file. A system-wide proxy can intercept local HTTP traffic, so temporarily disable Clash-style system proxy mode when diagnosing a widget that connects but receives no game data.

For controlled bots on a CS2 local listen server, run this after the local server starts:

```text
exec killconfirm_local_server
```

The installer places this config in the CS2 `cfg` directory. It enables a local server log that supplements missing takeover events; it does not apply to remote online servers.

The service control API uses a per-installation token stored in app local data. Do not copy or publish `service-auth-token.txt`, and do not expose the selected local port to the network.

## Economy Rewards

The default `rules` mode calculates rewards from Counter-Strike rules rather than treating every money delta as a reward.

It distinguishes the CS2 and CS:GO Legacy values for weapon kills, bomb and hostage objectives, round wins, round losses, planted-bomb compensation, loss streaks, and short-handed income. CS2 Competitive/Wingman also includes the newer `$50` team award paid to every CT for each Terrorist eliminated; Legacy does not receive it.

Arms Race, Deathmatch, Demolition, Survival, Training, and unknown custom modes do not display a fabricated cash reward because they do not use the supported standard cash economy.

An experimental GSI-delta mode remains available for validation, but `rules` is the recommended and default mode.

See the [complete CS2 and CS:GO Legacy economy matrix](docs/CS2_CSGO_ECONOMY_RULES.zh-CN.md) (Chinese) for every value and edge case.

## Legacy Bridge

Legacy controlled-bot kills require:

```text
KillConfirmService\legacy_bridge\killconfirm_bridge.smx
```

Install it into the Legacy client or server SourceMod `plugins` directory. The bridge forwards only kills after a bot takeover; ordinary player and observed-player kills remain sourced from GSI to prevent duplicate effects.

For a Legacy server running on this PC, list its root in:

```text
%LOCALAPPDATA%\Packages\KillConfirmGameBar.Overlay_5jgcw66eyez0m\LocalState\legacy-bridge-servers.txt
```

One server path is allowed per line. The current bridge reads local files and is not an unauthenticated network relay.

## Build From Source

Run tests first:

```powershell
cd KillConfirmService
cargo fmt -- --check
cargo check
cargo test
cd ..
```

Build a signed test-certificate MSIX Bundle without installing it:

```powershell
.\Build-DevPackage.ps1 -Configuration Release
```

Create the one-click `.exe` installers (new-user package with prerequisites and update package without prerequisites):

```powershell
.\Build-ReleaseInstaller.ps1 -Configuration Release
```

The first release build downloads and verifies the pinned LGPL FFmpeg dependency used by Custom Module video import. The resulting installer contains FFmpeg and does not need to download it on the user's computer. Neither command installs or replaces the currently registered Game Bar package unless `Build-DevPackage.ps1` is explicitly given `-Install`.

Use `-DisableSigning` only for a developer registration workflow. Public distributions should use a trusted signing certificate or signing service.

## Project Layout

| Path | Purpose |
| --- | --- |
| `KillConfirmService` | Rust GSI service, audio engine, event state, economy rules, and bridges |
| `Widget` | UWP Xbox Game Bar interface and animation renderer |
| `Package` | MSIX packaging project and manifest |
| `Installer` | Transfer-package and Inno Setup support files |
| `SourceAssets/GameStyles` | Source audio, animation, icons, and per-style sound packs |
| `docs` | Migration, dual-game deployment, and troubleshooting documentation |

Build scripts refresh package-ready assets from `SourceAssets`; edit source assets rather than generated copies under the widget or service output directories.

## Troubleshooting

If the widget opens but no effect appears:

- Verify that the GSI file is in the directory of the game version actually running.
- Fully restart the game after changing the file.
- Keep the widget open or pinned and check its service/GSI status.
- Confirm that no other application occupies the selected local port (default `127.0.0.1:10087`).
- Temporarily disable the system proxy.
- For Legacy takeover issues, check `sm plugins list` and the bridge log.

Logs:

```text
%LOCALAPPDATA%\Packages\KillConfirmGameBar.Overlay_5jgcw66eyez0m\LocalState\service.log
<Legacy root>\csgo\addons\sourcemod\logs\killconfirm_bridge.log
```

When reporting an issue, include the game version, mode, whether a bot was controlled, approximate kill time, selected pack, and the matching log section.

## Credits

The Rust service builds on [st0nie/cskillconfirm](https://github.com/st0nie/cskillconfirm) and [st0nie/gsi-cs2-rs](https://github.com/st0nie/gsi-cs2-rs).

The project also incorporates resources or inspiration from [MinecraftGD656/gd656killicon](https://github.com/MinecraftGD656/gd656killicon).

Additional resource reference: [Steam Workshop item 2721562982](https://steamcommunity.com/sharedfiles/filedetails/?id=2721562982).

## License

Licensed under the [GNU Affero General Public License v3.0](LICENSE).

This community project is not affiliated with Valve, Microsoft, Xbox, CrossFire, Riot Games, Electronic Arts, Krafton, Tencent, or any other game publisher.
