use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use gsi_cs2::map::Mode;
use tokio::time::sleep;

use super::event_stream::detect_counter_strike_roots;
use super::handler::resolve_crossfire_streak_count;
use super::logging::{local_state_dir, service_log};
use super::state::{AppState, CrossfireStreakMode, KillEvent};
use crate::soundpack::sound::play_audio;

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Default)]
struct TailState {
    offset: u64,
    pending: String,
    initialized: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct ControlledDeath {
    count: u16,
    enemies_alive_after: u16,
    attacker_user_id: String,
    attacker_name: String,
    victim_name: String,
    weapon: String,
    headshot: bool,
}

pub async fn watch_legacy_bridge_logs(app_state: Arc<AppState>) {
    let mut tails = HashMap::<PathBuf, TailState>::new();
    let mut last_discovery = Instant::now() - DISCOVERY_INTERVAL;

    loop {
        if tails.is_empty() && last_discovery.elapsed() >= DISCOVERY_INTERVAL {
            discover_bridge_logs(&mut tails);
            last_discovery = Instant::now();
        }

        let paths = tails.keys().cloned().collect::<Vec<_>>();
        for path in paths {
            if let Some(tail) = tails.get_mut(&path)
                && !tail.initialized
            {
                let Ok(metadata) = fs::metadata(&path) else {
                    continue;
                };
                tail.offset = metadata.len();
                tail.pending.clear();
                tail.initialized = true;
                service_log(&format!(
                    "Legacy controlled-bot bridge connected: {}",
                    path.display()
                ));
                continue;
            }

            let lines = tails
                .get_mut(&path)
                .map(|tail| read_appended_lines(&path, tail))
                .unwrap_or_default();
            for line in lines {
                if let Some(death) = parse_controlled_death(&line) {
                    emit_controlled_death(app_state.clone(), death).await;
                }
            }
        }

        sleep(POLL_INTERVAL).await;
    }
}

fn discover_bridge_logs(tails: &mut HashMap<PathBuf, TailState>) {
    let mut roots = detect_counter_strike_roots();
    roots.extend(configured_legacy_server_roots());
    for root in roots {
        if root.join("game").join("csgo").is_dir() {
            continue;
        }

        let path = root
            .join("csgo")
            .join("addons")
            .join("sourcemod")
            .join("logs")
            .join("killconfirm_bridge.log");
        if tails.contains_key(&path) {
            continue;
        }

        let metadata = fs::metadata(&path).ok();
        let initialized = metadata.is_some();
        let offset = metadata.map(|metadata| metadata.len()).unwrap_or(0);
        if initialized {
            service_log(&format!(
                "Legacy controlled-bot bridge connected: {}",
                path.display()
            ));
        } else {
            service_log(&format!(
                "Legacy controlled-bot bridge waiting for server log: {}",
                path.display()
            ));
        }
        tails.insert(
            path,
            TailState {
                offset,
                pending: String::new(),
                initialized,
            },
        );
    }
}

fn configured_legacy_server_roots() -> Vec<PathBuf> {
    let mut config_paths = vec![local_state_dir().join("legacy-bridge-servers.txt")];
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        config_paths.push(parent.join("legacy-bridge-servers.txt"));
    }

    let mut roots = Vec::new();
    for config_path in config_paths {
        let Ok(contents) = fs::read_to_string(config_path) else {
            continue;
        };
        for line in contents.lines() {
            let value = line.trim().trim_matches('"');
            if value.is_empty() || value.starts_with('#') {
                continue;
            }
            if let Some(root) = resolve_legacy_server_root(Path::new(value))
                && !roots.iter().any(|existing: &PathBuf| {
                    existing
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&root.to_string_lossy())
                })
            {
                roots.push(root);
            }
        }
    }
    roots
}

fn resolve_legacy_server_root(configured_path: &Path) -> Option<PathBuf> {
    [
        configured_path.to_path_buf(),
        configured_path.join("server"),
    ]
    .into_iter()
    .find(|root| {
        root.join("srcds.exe").is_file()
            && root.join("csgo").join("addons").join("sourcemod").is_dir()
    })
}

fn read_appended_lines(path: &Path, tail: &mut TailState) -> Vec<String> {
    let Ok(length) = fs::metadata(path).map(|metadata| metadata.len()) else {
        return Vec::new();
    };
    if length < tail.offset {
        tail.offset = 0;
        tail.pending.clear();
    }
    if length == tail.offset {
        return Vec::new();
    }

    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    if file.seek(SeekFrom::Start(tail.offset)).is_err() {
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

fn parse_controlled_death(line: &str) -> Option<ControlledDeath> {
    let payload = line.split_once("DEATH|")?.1;
    let fields = payload
        .split('|')
        .filter_map(|field| field.split_once('='))
        .collect::<HashMap<_, _>>();
    if fields.get("controlled") != Some(&"1") {
        return None;
    }

    Some(ControlledDeath {
        count: fields.get("controlled_count")?.parse().ok()?,
        enemies_alive_after: fields.get("enemies_alive_after")?.parse().ok()?,
        attacker_user_id: fields.get("attacker_uid")?.to_string(),
        attacker_name: fields.get("attacker_name")?.to_string(),
        victim_name: fields.get("victim_name")?.to_string(),
        weapon: fields.get("weapon")?.to_ascii_lowercase(),
        headshot: fields.get("headshot") == Some(&"1"),
    })
}

async fn emit_controlled_death(app_state: Arc<AppState>, death: ControlledDeath) {
    let controlled_bot_effects_enabled = app_state
        .controlled_bot_effects_enabled
        .load(Ordering::Relaxed);
    let is_knife = is_knife_classname(&death.weapon);
    let is_last = death.enemies_alive_after == 0;
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
    if !controlled_bot_effects_enabled {
        let mut mutable = app_state.mutable.write().await;
        mutable.last_legacy_bridge_kill_at = Some(now);
        mutable.has_first_kill_in_round = true;
        drop(mutable);
        service_log("Legacy controlled-bot effect suppressed by settings");
        return;
    }
    let (round_number, money_epoch, kill_count, mode) = {
        let mut mutable = app_state.mutable.write().await;
        let elapsed = mutable
            .last_crossfire_kill_at
            .map(|previous| now.saturating_duration_since(previous));
        let streak_count = resolve_crossfire_streak_count(
            mutable.crossfire_streak_kills,
            elapsed,
            streak_mode,
            streak_window_ms,
            death.count == 1,
            1,
        );
        mutable.crossfire_streak_kills = streak_count;
        mutable.last_crossfire_kill_at = Some(now);
        mutable.last_legacy_bridge_kill_at = Some(now);
        mutable.has_first_kill_in_round = true;
        (
            mutable.current_round,
            mutable.money_epoch,
            if streak_mode_active {
                streak_count
            } else {
                death.count.max(1)
            },
            mutable.last_game_mode.clone(),
        )
    };
    let event = KillEvent {
        kill_count,
        is_headshot: death.headshot,
        is_knife_kill: is_knife,
        is_first_kill: death.count == 1 && !is_last,
        is_last_kill: is_last,
        is_assist: false,
        play_main_animation: true,
        animation_key: None,
        event_kind: Some("kill".to_string()),
        weapon_badge_key: weapon_badge_key(&death.weapon).map(str::to_string),
        weapon_name: Some(weapon_display_name(&death.weapon)),
        money_reward: weapon_money_reward(&death.weapon, mode.as_ref()),
        round_number,
        money_epoch,
        player_name: death.attacker_name,
        target_name: Some(death.victim_name),
        steamid: format!("legacy-userid:{}", death.attacker_user_id),
    };

    service_log(&format!(
        "Legacy controlled-bot kill forwarded: count={} weapon={} headshot={} last={}",
        event.kill_count, death.weapon, event.is_headshot, event.is_last_kill
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
            "failed to play Legacy controlled-bot kill audio: {error}"
        ));
    }
}

pub(crate) fn is_knife_classname(weapon: &str) -> bool {
    weapon.contains("knife") || weapon.contains("bayonet")
}

pub(crate) fn weapon_badge_key(weapon: &str) -> Option<&'static str> {
    if is_knife_classname(weapon) {
        return Some("knife");
    }
    match weapon {
        "ak47" | "aug" | "famas" | "galilar" | "m4a1" | "m4a1_silencer" | "sg556" => {
            Some("assault")
        }
        "awp" | "g3sg1" | "scar20" | "ssg08" => Some("sniper"),
        "bizon" | "mac10" | "mp5sd" | "mp7" | "mp9" | "p90" | "ump45" => Some("scout"),
        "m249" | "mag7" | "negev" | "nova" | "sawedoff" | "xm1014" => Some("elite"),
        _ => None,
    }
}

pub(crate) fn weapon_money_reward(weapon: &str, mode: Option<&Mode>) -> u16 {
    let multiplier = match mode {
        Some(Mode::Casual) => 1,
        Some(Mode::Competitive | Mode::Wingman) | None => 2,
        _ => 0,
    };
    if is_knife_classname(weapon) {
        return 750 * multiplier;
    }
    let competitive_reward = match weapon {
        "mag7" | "nova" | "sawedoff" => 900,
        "bizon" | "mac10" | "mp5sd" | "mp7" | "mp9" | "ump45" | "xm1014" => 600,
        "awp" | "taser" => 100,
        _ => 300,
    };
    competitive_reward / 2 * multiplier
}

pub(crate) fn weapon_display_name(weapon: &str) -> String {
    match weapon {
        "ak47" => "AK-47",
        "m4a1" => "M4A4",
        "m4a1_silencer" => "M4A1-S",
        "deagle" => "Desert Eagle",
        "elite" => "Dual Berettas",
        "galilar" => "Galil AR",
        "hkp2000" => "P2000",
        "revolver" => "R8 Revolver",
        "sawedoff" => "Sawed-Off",
        "sg556" => "SG 553",
        "ssg08" => "SSG 08",
        "taser" => "Zeus x27",
        "tec9" => "Tec-9",
        "ump45" => "UMP-45",
        "usp_silencer" => "USP-S",
        value if is_knife_classname(value) => return knife_display_name(value),
        value => return value.to_ascii_uppercase(),
    }
    .to_string()
}

fn knife_display_name(weapon: &str) -> String {
    let name = weapon.strip_prefix("knife_").unwrap_or(weapon);
    match name {
        "bayonet" => "Bayonet",
        "butterfly" => "Butterfly Knife",
        "falchion" => "Falchion Knife",
        "flip" => "Flip Knife",
        "gut" => "Gut Knife",
        "karambit" => "Karambit",
        "m9_bayonet" => "M9 Bayonet",
        "push" => "Shadow Daggers",
        "survival_bowie" => "Bowie Knife",
        "tactical" => "Huntsman Knife",
        _ => "Knife",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        ControlledDeath, TailState, is_knife_classname, parse_controlled_death,
        read_appended_lines, weapon_badge_key, weapon_display_name, weapon_money_reward,
    };
    use std::fs;

    #[test]
    fn parses_only_controlled_deaths_from_sourcemod_log_lines() {
        let line = "L 08/11/2026 - 01:28:38: DEATH|controlled=1|controlled_bot_uid=16|controlled_count=1|enemies_alive_after=3|attacker_uid=2|attacker_name=James_Hotten|victim_name=Vexite|weapon=m4a1_silencer|headshot=0";
        assert_eq!(
            parse_controlled_death(line),
            Some(ControlledDeath {
                count: 1,
                enemies_alive_after: 3,
                attacker_user_id: "2".to_string(),
                attacker_name: "James_Hotten".to_string(),
                victim_name: "Vexite".to_string(),
                weapon: "m4a1_silencer".to_string(),
                headshot: false,
            })
        );
        assert!(parse_controlled_death(&line.replace("controlled=1", "controlled=0")).is_none());
    }

    #[test]
    fn maps_legacy_knives_and_common_weapon_rewards() {
        assert!(is_knife_classname("knife_karambit"));
        assert_eq!(weapon_badge_key("knife_butterfly"), Some("knife"));
        assert_eq!(weapon_display_name("knife_butterfly"), "Butterfly Knife");
        assert_eq!(
            weapon_money_reward("knife_karambit", Some(&gsi_cs2::map::Mode::Competitive)),
            1500
        );
        assert_eq!(
            weapon_money_reward("m4a1_silencer", Some(&gsi_cs2::map::Mode::Competitive)),
            300
        );
        assert_eq!(
            weapon_money_reward("nova", Some(&gsi_cs2::map::Mode::Casual)),
            450
        );
    }

    #[test]
    fn tailer_does_not_replay_existing_lines() {
        let folder = std::env::temp_dir().join(format!(
            "killconfirm-legacy-bridge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("bridge.log");
        fs::write(&path, "old\n").unwrap();
        let mut tail = TailState {
            offset: fs::metadata(&path).unwrap().len(),
            pending: String::new(),
            initialized: true,
        };
        assert!(read_appended_lines(&path, &mut tail).is_empty());
        use std::io::Write;
        writeln!(
            fs::OpenOptions::new().append(true).open(&path).unwrap(),
            "new"
        )
        .unwrap();
        assert_eq!(read_appended_lines(&path, &mut tail), vec!["new"]);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn resolves_a_dedicated_server_from_its_container_folder() {
        let folder = std::env::temp_dir().join(format!(
            "killconfirm-server-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server = folder.join("server");
        fs::create_dir_all(server.join("csgo/addons/sourcemod")).unwrap();
        fs::write(server.join("srcds.exe"), []).unwrap();
        assert_eq!(super::resolve_legacy_server_root(&folder), Some(server));
        fs::remove_dir_all(folder).unwrap();
    }
}
