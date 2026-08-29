use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};

use tokio::time::sleep;

use super::event_stream::detect_counter_strike_roots;
use super::handler::resolve_crossfire_streak_count;
use super::legacy_bridge::{
    is_knife_classname, weapon_badge_key, weapon_display_name, weapon_money_reward_for,
};
use super::logging::service_log;
use super::money_rules::EconomyVersion;
use super::state::{AppState, CrossfireStreakMode, KillEvent, PendingLastKill};
use crate::soundpack::sound::play_audio;

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const DISCOVERY_INTERVAL: Duration = Duration::from_millis(500);
const GSI_DEDUP_GRACE: Duration = Duration::from_millis(400);
const STEAM64_INDIVIDUAL_BASE: u64 = 76_561_197_960_265_728;

#[derive(Default)]
struct TailState {
    offset: u64,
    pending: String,
    dead_players: HashSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ServerKill {
    attacker_steamid: String,
    attacker_name: String,
    attacker_team: String,
    victim_name: String,
    victim_team: String,
    weapon: String,
    headshot: bool,
}

struct PendingServerKill {
    kill: ServerKill,
    observed_at: Instant,
    is_last: bool,
}

pub async fn watch_cs2_local_server_logs(app_state: Arc<AppState>) {
    let watcher_started = SystemTime::now();
    let mut tails = HashMap::<PathBuf, TailState>::new();
    let mut pending_kills = VecDeque::<PendingServerKill>::new();
    let mut last_discovery = Instant::now() - DISCOVERY_INTERVAL;

    loop {
        if last_discovery.elapsed() >= DISCOVERY_INTERVAL {
            discover_local_server_logs(&mut tails, watcher_started);
            last_discovery = Instant::now();
        }

        let paths = tails.keys().cloned().collect::<Vec<_>>();
        for path in paths {
            let lines = tails
                .get_mut(&path)
                .map(|tail| read_appended_lines(&path, tail))
                .unwrap_or_default();
            for line in lines {
                let attacker_was_dead = tails
                    .get(&path)
                    .and_then(|tail| {
                        parse_server_kill(&line).and_then(|kill| {
                            tail.dead_players
                                .contains(&kill.attacker_steamid)
                                .then_some(kill.attacker_steamid)
                        })
                    })
                    .is_some();
                if let Some(kill) = parse_server_kill(&line) {
                    if kill.attacker_team != kill.victim_team
                        && is_missing_gsi_candidate(&app_state, &kill, attacker_was_dead).await
                    {
                        pending_kills.push_back(PendingServerKill {
                            kill,
                            observed_at: Instant::now(),
                            is_last: false,
                        });
                    }
                } else if is_round_end_line(&line)
                    && let Some(pending) = pending_kills.back_mut()
                {
                    pending.is_last = true;
                }

                if let Some(tail) = tails.get_mut(&path) {
                    update_dead_players(&line, &mut tail.dead_players);
                }
            }
        }

        while pending_kills
            .front()
            .is_some_and(|pending| pending.observed_at.elapsed() >= GSI_DEDUP_GRACE)
        {
            if let Some(pending) = pending_kills.pop_front() {
                forward_missing_gsi_kill(app_state.clone(), pending).await;
            }
        }

        sleep(POLL_INTERVAL).await;
    }
}

fn discover_local_server_logs(
    tails: &mut HashMap<PathBuf, TailState>,
    watcher_started: SystemTime,
) {
    for root in detect_counter_strike_roots() {
        let cs2_dir = root.join("game").join("csgo");
        if !cs2_dir.is_dir() {
            continue;
        }

        let log_dir = cs2_dir.join("killconfirm_logs");
        let Ok(entries) = fs::read_dir(log_dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("log")
                || tails.contains_key(&path)
            {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let created_after_start = metadata
                .created()
                .or_else(|_| metadata.modified())
                .map(|created| created >= watcher_started)
                .unwrap_or(false);
            let offset = if created_after_start {
                0
            } else {
                metadata.len()
            };
            let dead_players = if created_after_start {
                HashSet::new()
            } else {
                bootstrap_dead_players(&path)
            };
            tails.insert(
                path.clone(),
                TailState {
                    offset,
                    pending: String::new(),
                    dead_players,
                },
            );
            service_log(&format!(
                "CS2 local controlled-bot log connected: {}",
                path.display()
            ));
        }
    }
}

fn read_appended_lines(path: &Path, tail: &mut TailState) -> Vec<String> {
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let Ok(length) = file.metadata().map(|metadata| metadata.len()) else {
        return Vec::new();
    };
    if length < tail.offset {
        tail.offset = 0;
        tail.pending.clear();
    }
    if length == tail.offset || file.seek(SeekFrom::Start(tail.offset)).is_err() {
        return Vec::new();
    }

    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    tail.offset += bytes.len() as u64;
    tail.pending.push_str(&String::from_utf8_lossy(&bytes));

    let mut lines = Vec::new();
    while let Some(newline) = tail.pending.find('\n') {
        let line = tail.pending[..newline].trim_end_matches('\r').to_string();
        tail.pending.drain(..=newline);
        lines.push(line);
    }
    lines
}

fn parse_server_kill(line: &str) -> Option<ServerKill> {
    let (_, payload) = line.split_once(": \"")?;
    let (attacker, remainder) = payload.split_once("\" [")?;
    let (_, remainder) = remainder.split_once("] killed \"")?;
    let (victim, remainder) = remainder.split_once("\" [")?;
    let (_, weapon_payload) = remainder.split_once("] with \"")?;
    let (weapon, suffix) = weapon_payload.split_once('"')?;

    let (attacker_name, _, attacker_steam, attacker_team) = parse_player_descriptor(attacker)?;
    let (victim_name, _, _, victim_team) = parse_player_descriptor(victim)?;
    let attacker_steamid = steam3_to_steam64(attacker_steam)?.to_string();

    Some(ServerKill {
        attacker_steamid,
        attacker_name: attacker_name.to_string(),
        attacker_team: attacker_team.to_string(),
        victim_name: victim_name.to_string(),
        victim_team: victim_team.to_string(),
        weapon: weapon.to_ascii_lowercase(),
        headshot: suffix.contains("(headshot)"),
    })
}

fn parse_player_descriptor(value: &str) -> Option<(&str, &str, &str, &str)> {
    let team_start = value.rfind("<")?;
    let team = value.get(team_start + 1..value.len().checked_sub(1)?)?;
    let before_team = value.get(..team_start)?;
    let steam_start = before_team.rfind("<")?;
    let steam = before_team.get(steam_start + 1..before_team.len().checked_sub(1)?)?;
    let before_steam = before_team.get(..steam_start)?;
    let user_start = before_steam.rfind('<')?;
    let user_id = before_steam.get(user_start + 1..before_steam.len().checked_sub(1)?)?;
    let name = before_steam.get(..user_start)?;
    Some((name, user_id, steam, team))
}

fn steam3_to_steam64(value: &str) -> Option<u64> {
    let account_id = value.strip_prefix("[U:1:")?.strip_suffix(']')?;
    STEAM64_INDIVIDUAL_BASE.checked_add(account_id.parse::<u64>().ok()?)
}

fn is_round_end_line(line: &str) -> bool {
    line.ends_with("World triggered \"Round_End\"")
}

fn is_round_start_line(line: &str) -> bool {
    line.ends_with("World triggered \"Round_Start\"")
}

fn logged_victim_steamid(line: &str) -> Option<String> {
    let (_, remainder) = line.split_once("] killed \"")?;
    let (victim, _) = remainder.split_once("\" [")?;
    let (_, _, steam, _) = parse_player_descriptor(victim)?;
    steam3_to_steam64(steam).map(|value| value.to_string())
}

fn logged_suicide_steamid(line: &str) -> Option<String> {
    if !line.contains(" committed suicide with \"") {
        return None;
    }
    let (_, payload) = line.split_once(": \"")?;
    let (player, _) = payload.split_once("\" [")?;
    let (_, _, steam, _) = parse_player_descriptor(player)?;
    steam3_to_steam64(steam).map(|value| value.to_string())
}

fn update_dead_players(line: &str, dead_players: &mut HashSet<String>) {
    if is_round_start_line(line) || line.ends_with("Log file closed") {
        dead_players.clear();
        return;
    }
    if let Some(steamid) = logged_victim_steamid(line).or_else(|| logged_suicide_steamid(line)) {
        dead_players.insert(steamid);
    }
}

fn bootstrap_dead_players(path: &Path) -> HashSet<String> {
    let mut dead_players = HashSet::new();
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            update_dead_players(line, &mut dead_players);
        }
    }
    dead_players
}

async fn is_missing_gsi_candidate(
    app_state: &AppState,
    kill: &ServerKill,
    attacker_was_dead: bool,
) -> bool {
    let mutable = app_state.mutable.read().await;
    mutable.initialized && mutable.steamid == kill.attacker_steamid && attacker_was_dead
}

async fn forward_missing_gsi_kill(app_state: Arc<AppState>, pending: PendingServerKill) {
    let controlled_bot_effects_enabled = app_state
        .controlled_bot_effects_enabled
        .load(Ordering::Relaxed);
    let crossfire_mode_active = app_state.crossfire_mode_active.load(Ordering::Relaxed);
    let shared_mode_active = app_state.shared_streak_mode_active.load(Ordering::Relaxed);
    let streak_mode_active = crossfire_mode_active || shared_mode_active;
    let streak_mode = CrossfireStreakMode::from_u8(if shared_mode_active {
        app_state.shared_streak_mode.load(Ordering::Relaxed)
    } else {
        app_state.crossfire_streak_mode.load(Ordering::Relaxed)
    });
    let streak_window_ms = if shared_mode_active {
        app_state.shared_streak_window_ms.load(Ordering::Relaxed)
    } else {
        app_state.crossfire_streak_window_ms.load(Ordering::Relaxed)
    };
    let now = Instant::now();

    let event = {
        let mut mutable = app_state.mutable.write().await;
        if mutable.steamid != pending.kill.attacker_steamid
            || mutable.last_cs2_gsi_kill_at.is_some_and(|gsi_at| {
                gsi_at >= pending.observed_at
                    || pending.observed_at.saturating_duration_since(gsi_at) <= GSI_DEDUP_GRACE
            })
        {
            return;
        }

        if mutable.cs2_local_log_round != mutable.current_round {
            mutable.cs2_local_log_round = mutable.current_round;
            mutable.cs2_local_log_round_kills = mutable.ply_kills;
            mutable.cs2_local_log_unconfirmed_kills = 0;
        }
        mutable.cs2_local_log_round_kills = mutable.cs2_local_log_round_kills.saturating_add(1);
        mutable.cs2_local_log_unconfirmed_kills =
            mutable.cs2_local_log_unconfirmed_kills.saturating_add(1);

        if !controlled_bot_effects_enabled {
            mutable.has_first_kill_in_round = true;
            mutable.pending_last_kill = None;
            service_log("CS2 local controlled-bot effect suppressed by settings");
            return;
        }

        let elapsed = mutable
            .last_crossfire_kill_at
            .map(|previous| now.saturating_duration_since(previous));
        let streak_count = resolve_crossfire_streak_count(
            mutable.crossfire_streak_kills,
            elapsed,
            streak_mode,
            streak_window_ms,
            mutable.cs2_local_log_round_kills == 1,
            1,
        );
        mutable.crossfire_streak_kills = streak_count;
        mutable.last_crossfire_kill_at = Some(now);

        let is_knife = is_knife_classname(&pending.kill.weapon);
        let money_reward = weapon_money_reward_for(
            &pending.kill.weapon,
            mutable.last_game_mode.as_ref(),
            EconomyVersion::Cs2,
        );
        let is_first = !pending.is_last && !mutable.has_first_kill_in_round;
        let kill_count = if streak_mode_active {
            streak_count
        } else {
            mutable.cs2_local_log_round_kills
        };
        mutable.has_first_kill_in_round = true;
        mutable.pending_last_kill = if pending.is_last {
            None
        } else {
            Some(PendingLastKill {
                recorded_at: now,
                kill_count,
                is_headshot: pending.kill.headshot,
                is_knife_kill: is_knife,
                weapon_badge_key: weapon_badge_key(&pending.kill.weapon).map(str::to_string),
                weapon_name: Some(weapon_display_name(&pending.kill.weapon)),
                money_reward,
            })
        };

        KillEvent {
            kill_count,
            is_headshot: pending.kill.headshot,
            is_knife_kill: is_knife,
            is_first_kill: is_first,
            is_last_kill: pending.is_last,
            is_assist: false,
            play_main_animation: true,
            animation_key: None,
            event_kind: Some("kill".to_string()),
            weapon_badge_key: weapon_badge_key(&pending.kill.weapon).map(str::to_string),
            weapon_name: Some(weapon_display_name(&pending.kill.weapon)),
            money_reward,
            round_number: mutable.current_round,
            money_epoch: mutable.money_epoch,
            player_name: pending.kill.attacker_name.clone(),
            target_name: Some(pending.kill.victim_name.clone()),
            steamid: pending.kill.attacker_steamid.clone(),
        }
    };

    service_log(&format!(
        "CS2 local controlled-bot kill forwarded: count={} weapon={} headshot={} last={}",
        event.kill_count, pending.kill.weapon, event.is_headshot, event.is_last_kill
    ));
    let _ = app_state.event_tx.send(event.clone());
    if let Err(error) = play_audio(
        app_state,
        event.kill_count,
        event.is_headshot,
        event.is_first_kill,
        event.is_knife_kill,
        event.is_last_kill,
        false,
        event.money_reward,
        event.event_kind,
        true,
    )
    .await
    {
        service_log(&format!(
            "failed to play CS2 local controlled-bot kill audio: {error}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ServerKill, is_round_end_line, logged_suicide_steamid, logged_victim_steamid,
        parse_server_kill, steam3_to_steam64, update_dead_players,
    };
    use std::collections::HashSet;

    #[test]
    fn parses_cs2_enemy_kill_with_weapon_and_headshot() {
        let line = r#"L 08/11/2026 - 02:30:47: "James_Hotten<0><[U:1:404007339]><TERRORIST>" [333 1854 97] killed "Colin<9><BOT><CT>" [735 2676 96] with "ak47" (headshot)"#;
        assert_eq!(
            parse_server_kill(line),
            Some(ServerKill {
                attacker_steamid: "76561198364273067".to_string(),
                attacker_name: "James_Hotten".to_string(),
                attacker_team: "TERRORIST".to_string(),
                victim_name: "Colin".to_string(),
                victim_team: "CT".to_string(),
                weapon: "ak47".to_string(),
                headshot: true,
            })
        );
    }

    #[test]
    fn parses_knife_and_exposes_teamkill_for_filtering() {
        let line = r#"L 08/11/2026 - 02:34:49: "James_Hotten<0><[U:1:404007339]><TERRORIST>" [0 0 0] killed "Maru<6><BOT><TERRORIST>" [0 0 0] with "knife_t""#;
        let kill = parse_server_kill(line).unwrap();
        assert_eq!(kill.weapon, "knife_t");
        assert_eq!(kill.attacker_team, kill.victim_team);
        assert!(!kill.headshot);
    }

    #[test]
    fn rejects_bot_attackers_and_recognizes_round_end() {
        let bot = r#"L 08/11/2026 - 02:33:37: "Mayer<5><BOT><TERRORIST>" [0 0 0] killed "Rivers<4><BOT><CT>" [0 0 0] with "knife_t""#;
        assert!(parse_server_kill(bot).is_none());
        assert!(is_round_end_line(
            r#"L 08/11/2026 - 02:31:07: World triggered "Round_End""#
        ));
        assert_eq!(
            steam3_to_steam64("[U:1:404007339]"),
            Some(76_561_198_364_273_067)
        );
    }

    #[test]
    fn tracks_human_death_and_clears_it_on_round_start() {
        let killed = r#"L 08/11/2026 - 09:19:23: "Bot<2><BOT><TERRORIST>" [0 0 0] killed "James_Hotten<0><[U:1:404007339]><CT>" [0 0 0] with "ak47""#;
        let suicide = r#"L 08/11/2026 - 09:39:25: "James_Hotten<0><[U:1:404007339]><TERRORIST>" [0 0 0] committed suicide with "world""#;
        assert_eq!(
            logged_victim_steamid(killed).as_deref(),
            Some("76561198364273067")
        );
        assert_eq!(
            logged_suicide_steamid(suicide).as_deref(),
            Some("76561198364273067")
        );

        let mut dead = HashSet::new();
        update_dead_players(suicide, &mut dead);
        assert!(dead.contains("76561198364273067"));
        update_dead_players(
            r#"L 08/11/2026 - 09:40:00: World triggered "Round_Start""#,
            &mut dead,
        );
        assert!(dead.is_empty());
    }
}
