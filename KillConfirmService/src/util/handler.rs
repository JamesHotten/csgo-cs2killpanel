use anyhow::Result;
use axum::body::Bytes;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use gsi_cs2::Body;
use gsi_cs2::player::Player;
use gsi_cs2::round::{BombState, RoundPhase};
use gsi_cs2::team::{TeamClass, TeamInfo};
use gsi_cs2::weapon::{WeaponName, WeaponState, WeaponType};
use serde_path_to_error::{Path as SerdePath, Segment as SerdePathSegment};
use thiserror::Error;
use tracing::{debug, error, warn};

use super::auth::has_valid_gsi_token;
use super::logging::service_log;
use super::state::{
    AppState, CrossfireStreakMode, KillEvent, MoneyRewardMode, PendingLastKill, PlayerKillSnapshot,
    PlayerViewBaseline, TrackedRoundPhase,
};
use super::{money_delta, money_rules};
use crate::soundpack::sound::play_audio;

// GSI is throttled to 100ms, so knife kills need a short history window.
const KNIFE_KILL_GRACE_WINDOW: Duration = Duration::from_millis(750);
const FINAL_KILL_GRACE_WINDOW: Duration = Duration::from_millis(1500);
const LATE_FINAL_KILL_WINDOW: Duration = Duration::from_millis(250);
const MAX_IGNORED_GSI_FIELDS: usize = 128;

struct ParsedGsiBody {
    body: Body,
    previously: Option<serde_json::Value>,
    is_legacy: bool,
}

#[derive(Default)]
struct PreviousWeaponEvidence {
    active_key: Option<String>,
    fired_key: Option<String>,
}

fn is_knife_weapon(weapon_name: &WeaponName, weapon_type: Option<&WeaponType>) -> bool {
    matches!(weapon_type, Some(WeaponType::Knife))
        || matches!(
            weapon_name,
            WeaponName::KnifeCT
                | WeaponName::KnifeT
                | WeaponName::KnifeBayonet
                | WeaponName::KnifeBowie
                | WeaponName::KnifeButterfly
                | WeaponName::KnifeClassic
                | WeaponName::KnifeFalchion
                | WeaponName::KnifeFlip
                | WeaponName::KnifeGut
                | WeaponName::KnifeHuntsman
                | WeaponName::KnifeKarambit
                | WeaponName::KnifeKukri
                | WeaponName::KnifeM9Bayonet
                | WeaponName::KnifeNavaja
                | WeaponName::KnifeNomad
                | WeaponName::KnifeParacord
                | WeaponName::KnifeShadowDaggers
                | WeaponName::KnifeSkeleton
                | WeaponName::KnifeStiletto
                | WeaponName::KnifeSurvival
                | WeaponName::KnifeTalon
                | WeaponName::KnifeUrsus
        )
}

fn resolve_is_knife_kill(
    current_weapon_is_knife: Option<bool>,
    recent_weapon_is_knife: bool,
) -> bool {
    current_weapon_is_knife.unwrap_or(recent_weapon_is_knife)
}

fn map_weapon_badge_key(weapon_type: WeaponType) -> Option<&'static str> {
    match weapon_type {
        WeaponType::Rifle => Some("assault"),
        WeaponType::MachineGun | WeaponType::Shotgun => Some("elite"),
        WeaponType::SMG => Some("scout"),
        WeaponType::SniperRifle => Some("sniper"),
        WeaponType::Knife => Some("knife"),
        WeaponType::Pistol => None,
        _ => None,
    }
}

fn map_weapon_name(weapon_name: &WeaponName) -> &'static str {
    match weapon_name {
        WeaponName::AK47 => "AK-47",
        WeaponName::AUG => "AUG",
        WeaponName::AWP => "AWP",
        WeaponName::AXE => "Axe",
        WeaponName::Bizon => "PP-Bizon",
        WeaponName::BumpMine => "Bump Mine",
        WeaponName::BreachCharge => "Breach Charge",
        WeaponName::C4 => "C4",
        WeaponName::CZ75A => "CZ-75 Auto",
        WeaponName::DesertEagle => "Desert Eagle",
        WeaponName::DecoyGrenade => "Decoy Grenade",
        WeaponName::DiversionDevice => "Diversion Device",
        WeaponName::DualBerettas => "Dual Berettas",
        WeaponName::FAMAS => "FAMAS",
        WeaponName::FireGrenade => "Fire Grenade",
        WeaponName::Firebomb => "Fire Bomb",
        WeaponName::Fists => "Fists",
        WeaponName::FiveSeven => "Five-SeveN",
        WeaponName::FlashbangGrenade => "Flashbang",
        WeaponName::FragGrenade => "Frag Grenade",
        WeaponName::G3SG1 => "G3SG1",
        WeaponName::Galilar => "Galil AR",
        WeaponName::Glock => "Glock-18",
        WeaponName::MediShot => "Medi-Shot",
        WeaponName::Hammer => "Hammer",
        WeaponName::HEGrenade => "HE Grenade",
        WeaponName::P2000 => "P2000",
        WeaponName::IncendiaryGrenade => "Incendiary Grenade",
        WeaponName::KnifeCT => "Knife",
        WeaponName::KnifeT => "Knife",
        WeaponName::KnifeBayonet => "Bayonet",
        WeaponName::KnifeBowie => "Bowie Knife",
        WeaponName::KnifeButterfly => "Butterfly Knife",
        WeaponName::KnifeClassic => "Classic Knife",
        WeaponName::KnifeFalchion => "Falchion Knife",
        WeaponName::KnifeFlip => "Flip Knife",
        WeaponName::KnifeGut => "Gut Knife",
        WeaponName::KnifeHuntsman => "Huntsman Knife",
        WeaponName::KnifeKarambit => "Karambit",
        WeaponName::KnifeKukri => "Kukri Knife",
        WeaponName::KnifeM9Bayonet => "M9 Bayonet",
        WeaponName::KnifeNavaja => "Navaja Knife",
        WeaponName::KnifeNomad => "Nomad Knife",
        WeaponName::KnifeParacord => "Paracord Knife",
        WeaponName::KnifeShadowDaggers => "Shadow Daggers",
        WeaponName::KnifeSkeleton => "Skeleton Knife",
        WeaponName::KnifeStiletto => "Stiletto Knife",
        WeaponName::KnifeSurvival => "Survival Knife",
        WeaponName::KnifeTalon => "Talon Knife",
        WeaponName::KnifeUrsus => "Ursus Knife",
        WeaponName::M249 => "M249",
        WeaponName::M4A4 => "M4A4",
        WeaponName::M4A1S => "M4A1-S",
        WeaponName::MAC10 => "MAC-10",
        WeaponName::MAG7 => "MAG-7",
        WeaponName::Molotov => "Molotov",
        WeaponName::MP5SD => "MP5-SD",
        WeaponName::MP7 => "MP7",
        WeaponName::MP9 => "MP9",
        WeaponName::Negev => "Negev",
        WeaponName::Nova => "Nova",
        WeaponName::P250 => "P250",
        WeaponName::P90 => "P90",
        WeaponName::Revolver => "R8 Revolver",
        WeaponName::SawedOff => "Sawed-Off",
        WeaponName::SCAR20 => "SCAR-20",
        WeaponName::SG556 => "SG 553",
        WeaponName::Spanner => "Spanner",
        WeaponName::Shield => "Riot Shield",
        WeaponName::SmokeGrenade => "Smoke Grenade",
        WeaponName::Snowball => "Snowball",
        WeaponName::SSG08 => "SSG 08",
        WeaponName::Tablet => "Tablet",
        WeaponName::TAGrenade => "TA Grenade",
        WeaponName::Zeus27 => "Zeus x27",
        WeaponName::Tripwirefire => "Tripwire Fire",
        WeaponName::TEC9 => "Tec-9",
        WeaponName::UMP45 => "UMP-45",
        WeaponName::USPS => "USP-S",
        WeaponName::XM1014 => "XM1014",
        WeaponName::RepulsorDevice => "Repulsor Device",
    }
}

fn same_team(left: &TeamClass, right: &TeamClass) -> bool {
    matches!(
        (left, right),
        (TeamClass::CT, TeamClass::CT) | (TeamClass::T, TeamClass::T)
    )
}

fn team_info_for<'a>(team: &TeamClass, ct: &'a TeamInfo, t: &'a TeamInfo) -> &'a TeamInfo {
    match team {
        TeamClass::CT => ct,
        TeamClass::T => t,
    }
}

fn opponent_team_display_name(team: Option<&TeamClass>) -> Option<String> {
    match team {
        Some(TeamClass::CT) => Some("恐怖分子".to_string()),
        Some(TeamClass::T) => Some("反恐精英".to_string()),
        _ => None,
    }
}

#[derive(Error, Debug)]
pub enum ApiError {}

#[derive(Error, Debug)]
enum GsiBodyError {
    #[error("invalid GSI JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing or invalid GSI auth token")]
    Unauthorized,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

pub async fn update(
    State(app_state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    app_state.gsi_posts.fetch_add(1, Ordering::Relaxed);
    app_state
        .last_gsi_post_unix_ms
        .store(unix_time_ms(), Ordering::Relaxed);

    let parsed = match parse_gsi_frame(&body) {
        Ok(parsed) => parsed,
        Err(error) => {
            app_state.gsi_parse_errors.fetch_add(1, Ordering::Relaxed);
            app_state
                .last_gsi_parse_error_unix_ms
                .store(unix_time_ms(), Ordering::Relaxed);
            warn!("failed to parse GSI payload: {error}");
            service_log(&format!("failed to parse GSI payload: {error}"));
            let status = if matches!(&error, GsiBodyError::Unauthorized) {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            };
            return Ok(status);
        }
    };
    let is_legacy = parsed.is_legacy;
    let data = parsed.body;

    let map = data.map.as_ref();
    let player_data = select_tracked_player(&data);
    let round = data.round.as_ref();

    if map.is_none() || player_data.is_none() {
        warn!("map or player data is missing");
        return Ok(StatusCode::OK);
    }

    if let Some(whitelist) = &app_state.args.steamid {
        let steamid = player_data.as_ref().unwrap().1;
        if steamid != whitelist {
            return Ok(StatusCode::OK);
        }
    }

    let (ply, tracked_steamid) = player_data.unwrap();
    let Some(ply_state) = ply.state.as_ref() else {
        warn!("player state is missing");
        return Ok(StatusCode::OK);
    };
    let now = Instant::now();
    let map_data = map.unwrap();
    let current_round = map_data.round;
    let current_mode = &map_data.mode;
    let current_player_money = ply_state.money;
    let current_bomb_state = data
        .bomb
        .as_ref()
        .map(|bomb| bomb.state.trim().to_ascii_lowercase());
    let current_bomb_player = data
        .bomb
        .as_ref()
        .and_then(|bomb| bomb.player.as_deref())
        .map(str::to_string);
    let current_round_phase = round
        .map(|value| map_round_phase(&value.phase))
        .or_else(|| infer_round_phase_from_kills(ply_state.round_kills));

    let current_active_weapon_entry = ply
        .weapons
        .iter()
        .find(|(_, weapon)| matches!(weapon.state, WeaponState::Active));
    let current_active_weapon_key = current_active_weapon_entry.map(|(key, _)| key.as_str());
    let current_active_weapon = current_active_weapon_entry.map(|(_, weapon)| weapon);
    let previous_weapon_evidence =
        previous_weapon_evidence(parsed.previously.as_ref(), tracked_steamid, &ply.weapons);
    let kill_weapon_key = resolve_kill_weapon_key(
        current_active_weapon_key,
        previous_weapon_evidence.active_key.as_deref(),
        previous_weapon_evidence.fired_key.as_deref(),
    );
    let current_active_weapon_is_knife =
        current_active_weapon.map(|weapon| is_knife_weapon(&weapon.name, weapon.r#type.as_ref()));
    let current_active_weapon_badge_key = current_active_weapon
        .and_then(|weapon| weapon.r#type.clone())
        .and_then(map_weapon_badge_key)
        .map(str::to_string);
    let current_active_weapon_name =
        current_active_weapon.map(|weapon| map_weapon_name(&weapon.name).to_string());
    let current_active_weapon_money_reward = current_active_weapon
        .map(|weapon| money_rules::weapon_kill_reward(&weapon.name, current_mode));

    let binding = app_state.mutable.read().await;
    let current_raw_round_kills = ply_state.round_kills;
    let previous_raw_round_kills = binding.raw_round_kills;
    let previous_resolved_round_kills = binding.ply_kills;
    let current_match_kills = ply.match_stats.as_ref().map(|stats| stats.kills);
    let previous_match_kills = binding.match_kills;
    let cached_player_kill_snapshot = binding.player_kill_snapshots.get(tracked_steamid).copied();
    let cached_player_view_baseline = binding.player_view_baselines.get(tracked_steamid).copied();

    let current_hs_kills = ply_state.round_killhs;
    let global_origin_hs_kills = binding.ply_hs_kills;
    let current_assists = ply
        .match_stats
        .as_ref()
        .map(|stats| stats.assists)
        .unwrap_or(0);
    let global_original_assists = binding.ply_assists;
    let current_deaths = ply
        .match_stats
        .as_ref()
        .map(|stats| stats.deaths)
        .unwrap_or(0);
    let global_original_deaths = binding.ply_deaths;
    let current_score = ply
        .match_stats
        .as_ref()
        .map(|stats| stats.score)
        .unwrap_or(0);
    let global_original_score = binding.ply_score;
    let global_previous_player_health = binding.last_player_health;

    let global_is_initialized = binding.initialized;
    let original_steamid = binding.steamid.clone();
    let previous_round = binding.current_round;
    let previous_round_phase = binding.last_round_phase;
    let pending_round_over_at = binding.pending_round_over_at;
    let had_first_kill_in_round = binding.has_first_kill_in_round;
    let pending_last_kill = binding.pending_last_kill.clone();
    let previous_player_money = binding.last_player_money;
    let previous_money_epoch = binding.money_epoch;
    let previous_bomb_state = binding.last_bomb_state.clone();
    let previous_bomb_player = binding.last_bomb_player.clone();
    let previous_crossfire_streak_kills = binding.crossfire_streak_kills;
    let previous_crossfire_kill_at = binding.last_crossfire_kill_at;
    let last_legacy_bridge_kill_at = binding.last_legacy_bridge_kill_at;
    let cs2_local_log_unconfirmed_kills = binding.cs2_local_log_unconfirmed_kills;
    let recent_weapon_is_knife = binding.last_active_weapon_is_knife
        && binding
            .last_active_weapon_seen_at
            .map(|seen_at| now.saturating_duration_since(seen_at) <= KNIFE_KILL_GRACE_WINDOW)
            .unwrap_or(false);
    let recent_weapon_badge_key = binding.last_active_weapon_badge_key.clone().filter(|_| {
        binding
            .last_active_weapon_seen_at
            .map(|seen_at| now.saturating_duration_since(seen_at) <= KNIFE_KILL_GRACE_WINDOW)
            .unwrap_or(false)
    });
    let recent_weapon_name = binding.last_active_weapon_name.clone().filter(|_| {
        binding
            .last_active_weapon_seen_at
            .map(|seen_at| now.saturating_duration_since(seen_at) <= KNIFE_KILL_GRACE_WINDOW)
            .unwrap_or(false)
    });
    let recent_weapon_money_reward = binding
        .last_active_weapon_seen_at
        .filter(|seen_at| now.saturating_duration_since(*seen_at) <= KNIFE_KILL_GRACE_WINDOW)
        .map(|_| binding.last_active_weapon_money_reward);
    drop(binding);

    let money_reward_mode =
        MoneyRewardMode::from_u8(app_state.money_reward_mode.load(Ordering::Relaxed));
    let crossfire_streak_mode =
        CrossfireStreakMode::from_u8(app_state.crossfire_streak_mode.load(Ordering::Relaxed));
    let crossfire_streak_window_ms = app_state.crossfire_streak_window_ms.load(Ordering::Relaxed);
    let crossfire_mode_active = app_state.crossfire_mode_active.load(Ordering::Relaxed);
    let shared_streak_mode =
        CrossfireStreakMode::from_u8(app_state.shared_streak_mode.load(Ordering::Relaxed));
    let shared_streak_window_ms = app_state.shared_streak_window_ms.load(Ordering::Relaxed);
    let shared_streak_mode_active = app_state.shared_streak_mode_active.load(Ordering::Relaxed);
    let active_streak_mode = if shared_streak_mode_active {
        shared_streak_mode
    } else {
        crossfire_streak_mode
    };
    let active_streak_window_ms = if shared_streak_mode_active {
        shared_streak_window_ms
    } else {
        crossfire_streak_window_ms
    };
    let streak_mode_active = crossfire_mode_active || shared_streak_mode_active;

    let steamid = tracked_steamid;
    let player_name = ply.name.as_deref().unwrap_or("").to_string();
    let player_team = ply.team.as_ref();
    let target_name = opponent_team_display_name(player_team);

    let round_changed = previous_round != current_round;
    let round_reset =
        round_changed || matches!(current_round_phase, Some(TrackedRoundPhase::FreezeTime));
    let phase_transition_to_over = previous_round_phase == Some(TrackedRoundPhase::Live)
        && current_round_phase == Some(TrackedRoundPhase::Over);
    let player_team_won_round = round
        .and_then(|round_data| round_data.win_team.as_ref())
        .and_then(|win_team| player_team.map(|team| same_team(team, win_team)));
    let latest_round_outcome = map_data
        .round_wins
        .iter()
        .max_by_key(|(round_number, _)| *round_number)
        .map(|(_, outcome)| outcome.as_str());
    let bomb_exploded = bomb_state_is_exploded(
        round.and_then(|round_data| round_data.bomb.as_ref()),
        current_bomb_state.as_deref(),
    );
    let is_hostage_rescue_round =
        latest_round_outcome == Some("ct_win_rescue") && matches!(player_team, Some(TeamClass::CT));
    let player_identity_matches = steamid == original_steamid || original_steamid.is_empty();
    let switched_view_baseline = select_same_round_view_baseline(
        player_identity_matches,
        current_round,
        cached_player_view_baseline,
    );
    let is_initialized = if player_identity_matches {
        global_is_initialized
    } else {
        switched_view_baseline.is_some()
    };
    let origin_hs_kills = switched_view_baseline
        .map(|baseline| baseline.round_headshot_kills)
        .unwrap_or(global_origin_hs_kills);
    let original_assists = switched_view_baseline
        .map(|baseline| baseline.assists)
        .unwrap_or(global_original_assists);
    let original_deaths = switched_view_baseline
        .map(|baseline| baseline.deaths)
        .unwrap_or(global_original_deaths);
    let original_score = switched_view_baseline
        .map(|baseline| baseline.score)
        .unwrap_or(global_original_score);
    let previous_player_health = switched_view_baseline
        .map(|baseline| baseline.health)
        .unwrap_or(global_previous_player_health);
    let recent_weapon_is_knife = player_identity_matches && recent_weapon_is_knife;
    let recent_weapon_badge_key = player_identity_matches
        .then_some(recent_weapon_badge_key)
        .flatten();
    let recent_weapon_name = player_identity_matches
        .then_some(recent_weapon_name)
        .flatten();
    let recent_weapon_money_reward = player_identity_matches
        .then_some(recent_weapon_money_reward)
        .flatten();
    let (current_kills, observed_kill_delta) = resolve_main_view_kill_observation(
        is_initialized,
        player_identity_matches,
        true,
        round_reset,
        current_round,
        current_raw_round_kills,
        previous_raw_round_kills,
        current_match_kills,
        previous_match_kills,
        previous_resolved_round_kills,
        cached_player_kill_snapshot,
    );
    let original_kills = previous_resolved_round_kills;
    let death_count_reset = current_deaths > original_deaths && player_identity_matches;
    let health_death_reset = is_initialized
        && previous_player_health > 0
        && ply_state.health == 0
        && player_identity_matches;
    let death_reset = death_count_reset || health_death_reset;
    let freeze_phase_started = previous_round_phase != Some(TrackedRoundPhase::FreezeTime)
        && current_round_phase == Some(TrackedRoundPhase::FreezeTime);
    let money_scope_reset =
        round_changed || freeze_phase_started || death_reset || !player_identity_matches;
    let current_money_epoch = if money_scope_reset {
        previous_money_epoch.wrapping_add(1)
    } else {
        previous_money_epoch
    };
    let previous_player_money_for_delta = if money_scope_reset {
        None
    } else {
        previous_player_money
    };
    let cs2_local_log_confirmed_delta = if is_legacy {
        0
    } else {
        observed_kill_delta.min(cs2_local_log_unconfirmed_kills)
    };
    let effective_observed_kill_delta =
        observed_kill_delta.saturating_sub(cs2_local_log_confirmed_delta);
    let kill_count_increased = effective_observed_kill_delta > 0;
    let suppress_death_round_end_kill = should_suppress_death_round_end_kill(
        death_reset,
        previous_player_health,
        ply_state.health,
        current_score > original_score,
        current_round_phase,
        player_team_won_round,
    );
    let main_view_switch_kill = !player_identity_matches && observed_kill_delta > 0;
    let bridge_already_reported_kill =
        should_suppress_gsi_after_legacy_bridge(is_legacy, last_legacy_bridge_kill_at, now);
    let can_emit_kill = kill_count_increased
        && (player_identity_matches || main_view_switch_kill)
        && !suppress_death_round_end_kill
        && !bridge_already_reported_kill
        && !bomb_exploded;
    let can_emit_assist = should_emit_assist(
        is_initialized,
        player_identity_matches,
        current_assists,
        original_assists,
    );
    let pending_round_over_age =
        pending_round_over_at.map(|recorded_at| now.saturating_duration_since(recorded_at));
    let round_end_reordering_active = phase_transition_to_over
        || pending_round_over_age
            .map(|age| age <= LATE_FINAL_KILL_WINDOW)
            .unwrap_or(false);
    let crossfire_kill_delta = resolve_kill_delta(
        is_initialized,
        can_emit_kill,
        current_kills,
        original_kills,
        pending_round_over_age,
    );
    let crossfire_elapsed =
        previous_crossfire_kill_at.map(|last_kill_at| now.saturating_duration_since(last_kill_at));
    let crossfire_streak_kills = resolve_crossfire_streak_count(
        previous_crossfire_streak_kills,
        crossfire_elapsed,
        active_streak_mode,
        active_streak_window_ms,
        should_reset_streak_before_kill(
            round_reset,
            round_end_reordering_active,
            player_identity_matches,
            can_emit_kill,
        ),
        crossfire_kill_delta,
    );
    let event_kill_count = if streak_mode_active {
        crossfire_streak_kills
    } else {
        current_kills
    };
    let first_kill_already_seen = if !player_identity_matches {
        cached_player_kill_snapshot
            .filter(|snapshot| snapshot.round == current_round)
            .is_some_and(|snapshot| snapshot.resolved_round_kills > 0)
    } else if round_reset && !round_end_reordering_active {
        false
    } else {
        had_first_kill_in_round
    };
    let is_late_final_kill =
        should_mark_late_final_kill(can_emit_kill, pending_round_over_age, player_team_won_round);
    let defer_round_resolution = is_initialized && phase_transition_to_over && !can_emit_kill;

    let should_clear_pending_last_kill =
        !player_identity_matches || (round_reset && !round_end_reordering_active);
    let mut pending_last_kill_for_next = if should_clear_pending_last_kill {
        None
    } else {
        pending_last_kill.clone()
    };
    let mut kill_event_to_send = None;
    let mut badge_only_event_to_send = None;
    let mut assist_event_to_send = None;
    let mut bomb_objective_event_to_send = None;
    let mut hostage_objective_event_to_send = None;
    let mut round_bonus_event_to_send = None;

    if is_initialized && !steamid.is_empty() && player_identity_matches {
        let completed_bomb_action = match (
            previous_bomb_state.as_deref(),
            current_bomb_state.as_deref(),
            previous_bomb_player.as_deref(),
        ) {
            (Some("planting"), Some("planted"), Some(actor)) if actor == steamid => {
                Some("bomb_plant")
            }
            (Some("defusing"), Some("defused"), Some(actor)) if actor == steamid => {
                Some("bomb_defuse")
            }
            _ => None,
        };

        if let Some(event_kind) = completed_bomb_action {
            bomb_objective_event_to_send = Some(KillEvent {
                kill_count: 0,
                is_headshot: false,
                is_knife_kill: false,
                is_first_kill: false,
                is_last_kill: false,
                is_assist: false,
                play_main_animation: false,
                animation_key: Some(event_kind.to_string()),
                event_kind: Some(event_kind.to_string()),
                weapon_badge_key: None,
                weapon_name: None,
                money_reward: money_rules::bomb_objective_reward(current_mode),
                round_number: current_round,
                money_epoch: current_money_epoch,
                player_name: player_name.clone(),
                target_name: None,
                steamid: steamid.to_string(),
            });
        }
    }

    if is_initialized && can_emit_kill {
        let is_headshot = current_hs_kills > origin_hs_kills;
        let kill_weapon = kill_weapon_key
            .as_deref()
            .and_then(|key| ply.weapons.get(key));
        let evidence_points_away_from_current = kill_weapon_key.as_deref().is_some()
            && kill_weapon_key.as_deref() != current_active_weapon_key;
        let kill_weapon_is_knife = kill_weapon
            .map(|weapon| is_knife_weapon(&weapon.name, weapon.r#type.as_ref()))
            .or(if evidence_points_away_from_current {
                None
            } else {
                current_active_weapon_is_knife
            });
        let is_knife_kill = resolve_is_knife_kill(kill_weapon_is_knife, recent_weapon_is_knife);
        let weapon_badge_key = kill_weapon
            .and_then(|weapon| weapon.r#type.clone())
            .and_then(map_weapon_badge_key)
            .map(str::to_string)
            .or_else(|| recent_weapon_badge_key.clone());
        let weapon_name = kill_weapon
            .map(|weapon| map_weapon_name(&weapon.name).to_string())
            .or_else(|| recent_weapon_name.clone());
        let kill_weapon_money_reward =
            kill_weapon.map(|weapon| money_rules::weapon_kill_reward(&weapon.name, current_mode));
        let rule_money_reward = if money_rules::uses_standard_cash_economy(current_mode) {
            kill_weapon_money_reward
                .or(recent_weapon_money_reward)
                .unwrap_or_else(|| money_rules::default_kill_reward(current_mode))
        } else {
            0
        };
        let money_reward = match money_reward_mode {
            MoneyRewardMode::Delta => money_delta::kill_reward(
                previous_player_money_for_delta,
                current_player_money,
                rule_money_reward,
            ),
            MoneyRewardMode::Rules => rule_money_reward,
        };
        let is_last_kill = phase_transition_to_over || is_late_final_kill;
        let is_first_kill = !is_last_kill && !first_kill_already_seen;
        if is_last_kill {
            pending_last_kill_for_next = None;
        } else {
            pending_last_kill_for_next = Some(PendingLastKill {
                recorded_at: now,
                kill_count: event_kill_count,
                is_headshot,
                is_knife_kill,
                weapon_badge_key: weapon_badge_key.clone(),
                weapon_name: weapon_name.clone(),
                money_reward,
            });
        }

        kill_event_to_send = Some(KillEvent {
            kill_count: event_kill_count,
            is_headshot,
            is_knife_kill,
            is_first_kill,
            is_last_kill,
            is_assist: false,
            play_main_animation: true,
            animation_key: None,
            event_kind: Some("kill".to_string()),
            weapon_badge_key: weapon_badge_key.clone(),
            weapon_name: weapon_name.clone(),
            money_reward,
            round_number: current_round,
            money_epoch: current_money_epoch,
            player_name: player_name.clone(),
            target_name: target_name.clone(),
            steamid: steamid.to_string(),
        });

        let app_state_clone = app_state.clone();

        tokio::spawn(async move {
            let result = play_audio(
                app_state_clone,
                event_kill_count,
                is_headshot,
                is_first_kill,
                is_knife_kill,
                is_last_kill,
                false,
                money_reward,
                Some("kill".to_string()),
                true,
            )
            .await;

            if let Err(e) = result {
                error!("Failed to play audio: {}", e);
            }
        });
        debug!(
            "player: {}, kills: {}, headshot: {}, knife: {}, first: {}, last: {}",
            ply.name.as_deref().unwrap_or(""),
            event_kill_count,
            is_headshot,
            is_knife_kill,
            is_first_kill,
            is_last_kill
        );
    } else if is_initialized && phase_transition_to_over {
        if !round_end_allows_delayed_last_kill(
            round.and_then(|round_data| round_data.bomb.as_ref()),
            current_bomb_state.as_deref(),
            latest_round_outcome,
        ) {
            pending_last_kill_for_next = None;
        } else if let Some(pending_last_kill) = pending_last_kill {
            if should_promote_pending_last_kill(
                now.saturating_duration_since(pending_last_kill.recorded_at),
                death_reset,
                player_team_won_round,
            ) {
                badge_only_event_to_send = Some(KillEvent {
                    kill_count: pending_last_kill.kill_count,
                    is_headshot: pending_last_kill.is_headshot,
                    is_knife_kill: pending_last_kill.is_knife_kill,
                    is_first_kill: false,
                    is_last_kill: true,
                    is_assist: false,
                    play_main_animation: pending_last_kill.kill_count == 1
                        && pending_last_kill.is_headshot,
                    animation_key: None,
                    event_kind: Some("kill".to_string()),
                    weapon_badge_key: pending_last_kill.weapon_badge_key.clone(),
                    weapon_name: pending_last_kill.weapon_name.clone(),
                    money_reward: pending_last_kill.money_reward,
                    round_number: current_round,
                    money_epoch: current_money_epoch,
                    player_name: player_name.clone(),
                    target_name: target_name.clone(),
                    steamid: steamid.to_string(),
                });
                debug!(
                    "player: {}, resolved delayed final kill for round kill {}",
                    ply.name.as_deref().unwrap_or(""),
                    pending_last_kill.kill_count
                );
            }

            pending_last_kill_for_next = None;
        }
    }
    if is_initialized && can_emit_assist {
        assist_event_to_send = Some(KillEvent {
            kill_count: 0,
            is_headshot: false,
            is_knife_kill: false,
            is_first_kill: false,
            is_last_kill: false,
            is_assist: true,
            play_main_animation: false,
            animation_key: Some("assist".to_string()),
            event_kind: Some("assist".to_string()),
            weapon_badge_key: None,
            weapon_name: current_active_weapon_name.clone(),
            money_reward: 0,
            round_number: current_round,
            money_epoch: current_money_epoch,
            player_name: player_name.clone(),
            target_name: target_name.clone(),
            steamid: steamid.to_string(),
        });
    }

    if is_initialized && phase_transition_to_over {
        if let (Some(round_data), Some(player_team), Some(win_team)) = (
            round,
            player_team,
            round.and_then(|value| value.win_team.as_ref()),
        ) {
            let did_win = same_team(player_team, win_team);
            let rule_money_reward = if did_win {
                money_rules::round_win_bonus(
                    win_team,
                    round_data.bomb.as_ref(),
                    current_mode,
                    &map_data.name,
                    latest_round_outcome,
                )
            } else {
                let team_info = team_info_for(player_team, &map_data.team_ct, &map_data.team_t);
                money_rules::loss_bonus(
                    team_info.consecutive_round_losses,
                    current_mode,
                    player_team,
                    round_data.bomb.as_ref(),
                    latest_round_outcome,
                )
            };
            let already_assigned_money = kill_event_to_send
                .as_ref()
                .or(badge_only_event_to_send.as_ref())
                .map(|event| event.money_reward)
                .unwrap_or(0)
                .saturating_add(
                    bomb_objective_event_to_send
                        .as_ref()
                        .map(|event| event.money_reward)
                        .unwrap_or(0),
                );
            let money_reward = match money_reward_mode {
                MoneyRewardMode::Delta => money_delta::round_reward(
                    previous_player_money_for_delta,
                    current_player_money,
                    rule_money_reward,
                    already_assigned_money,
                ),
                MoneyRewardMode::Rules => rule_money_reward,
            };
            round_bonus_event_to_send = Some(KillEvent {
                kill_count: 0,
                is_headshot: false,
                is_knife_kill: false,
                is_first_kill: false,
                is_last_kill: false,
                is_assist: false,
                play_main_animation: false,
                animation_key: Some(if did_win {
                    "round_win".to_string()
                } else {
                    "round_loss".to_string()
                }),
                event_kind: Some(if did_win {
                    "round_win".to_string()
                } else {
                    "round_loss".to_string()
                }),
                weapon_badge_key: None,
                weapon_name: None,
                money_reward,
                round_number: current_round,
                money_epoch: current_money_epoch,
                player_name: player_name.clone(),
                target_name: None,
                steamid: steamid.to_string(),
            });
        }
    }

    if is_initialized
        && player_identity_matches
        && !can_emit_kill
        && !can_emit_assist
        && money_rules::is_hostage_map(&map_data.name)
        && matches!(
            current_round_phase,
            Some(TrackedRoundPhase::Live | TrackedRoundPhase::Over)
        )
    {
        let already_assigned_money = kill_event_to_send
            .as_ref()
            .map(|event| event.money_reward)
            .unwrap_or(0)
            .saturating_add(
                bomb_objective_event_to_send
                    .as_ref()
                    .map(|event| event.money_reward)
                    .unwrap_or(0),
            )
            .saturating_add(
                round_bonus_event_to_send
                    .as_ref()
                    .map(|event| event.money_reward)
                    .unwrap_or(0),
            );

        if let Some(money_reward) = money_delta::unassigned_objective_reward(
            previous_player_money_for_delta,
            current_player_money,
            already_assigned_money,
        ) {
            if let Some(event_kind) = money_rules::hostage_objective_kind(
                money_reward,
                current_mode,
                is_hostage_rescue_round,
            ) {
                hostage_objective_event_to_send = Some(KillEvent {
                    kill_count: 0,
                    is_headshot: false,
                    is_knife_kill: false,
                    is_first_kill: false,
                    is_last_kill: false,
                    is_assist: false,
                    play_main_animation: false,
                    animation_key: Some(event_kind.to_string()),
                    event_kind: Some(event_kind.to_string()),
                    weapon_badge_key: None,
                    weapon_name: None,
                    money_reward,
                    round_number: current_round,
                    money_epoch: current_money_epoch,
                    player_name: player_name.clone(),
                    target_name: None,
                    steamid: steamid.to_string(),
                });
            }
        }
    }

    let mut binding = app_state.mutable.write().await;

    if !binding.initialized {
        binding.initialized = true;
    }

    binding.ply_kills = current_kills;
    binding.raw_round_kills = current_raw_round_kills;
    if current_match_kills.is_some() {
        binding.match_kills = current_match_kills;
    }
    binding.ply_hs_kills = current_hs_kills;
    binding.ply_assists = current_assists;
    binding.ply_deaths = current_deaths;
    binding.ply_score = current_score;
    binding.last_player_health = ply_state.health;
    binding.steamid = steamid.to_string();
    binding.current_round = current_round;
    binding.last_round_phase = current_round_phase;
    binding.cs2_local_log_unconfirmed_kills = if is_legacy || round_changed {
        0
    } else {
        cs2_local_log_unconfirmed_kills.saturating_sub(cs2_local_log_confirmed_delta)
    };
    if !is_legacy && can_emit_kill {
        binding.last_cs2_gsi_kill_at = Some(now);
    }
    binding.pending_round_over_at = if phase_transition_to_over && !can_emit_kill {
        Some(now)
    } else if is_late_final_kill {
        None
    } else {
        pending_round_over_at
    };
    binding.last_player_money = Some(current_player_money);
    binding.money_epoch = current_money_epoch;
    binding.last_bomb_state = current_bomb_state;
    binding.last_bomb_player = current_bomb_player;
    if (round_reset && !round_end_reordering_active) || !player_identity_matches || death_reset {
        binding.crossfire_streak_kills = 0;
        binding.last_crossfire_kill_at = None;
    } else {
        binding.crossfire_streak_kills = crossfire_streak_kills;
        binding.last_crossfire_kill_at = if crossfire_kill_delta > 0 {
            Some(now)
        } else if crossfire_streak_kills == 0 {
            None
        } else {
            previous_crossfire_kill_at
        };
    }
    binding.has_first_kill_in_round = current_kills > 0
        || (!(round_reset && !round_end_reordering_active) && had_first_kill_in_round)
        || can_emit_kill;
    binding.pending_last_kill = pending_last_kill_for_next;
    binding.last_game_mode = Some(current_mode.clone());
    binding.player_kill_snapshots.insert(
        steamid.to_string(),
        PlayerKillSnapshot {
            round: current_round,
            resolved_round_kills: current_kills,
            raw_round_kills: current_raw_round_kills,
            match_kills: current_match_kills,
        },
    );
    binding.player_view_baselines.insert(
        steamid.to_string(),
        PlayerViewBaseline {
            round: current_round,
            round_headshot_kills: current_hs_kills,
            assists: current_assists,
            deaths: current_deaths,
            score: current_score,
            health: ply_state.health,
        },
    );
    if let Some(is_knife) = current_active_weapon_is_knife {
        binding.last_active_weapon_is_knife = is_knife;
        binding.last_active_weapon_seen_at = Some(now);
    }
    binding.last_active_weapon_badge_key = current_active_weapon_badge_key;
    binding.last_active_weapon_name = current_active_weapon_name;
    if let Some(money_reward) = current_active_weapon_money_reward {
        binding.last_active_weapon_money_reward = money_reward;
    }

    drop(binding);

    if defer_round_resolution {
        let deferred_last_kill = badge_only_event_to_send.take();
        let deferred_round_bonus = round_bonus_event_to_send.take();
        let app_state_clone = app_state.clone();
        let round_over_at = now;
        let play_delayed_last_audio = !crossfire_mode_active
            || app_state
                .crossfire_last_kill_special_audio
                .load(Ordering::Relaxed);

        tokio::spawn(async move {
            tokio::time::sleep(LATE_FINAL_KILL_WINDOW).await;

            let should_emit_cached_last_kill = {
                let mut mutable = app_state_clone.mutable.write().await;
                if mutable.pending_round_over_at == Some(round_over_at) {
                    mutable.pending_round_over_at = None;
                    true
                } else {
                    false
                }
            };

            let last_kill_audio = if should_emit_cached_last_kill {
                deferred_last_kill
            } else {
                None
            };

            if let Some(event) = last_kill_audio.as_ref() {
                let _ = app_state_clone.event_tx.send(event.clone());
            }
            if let Some(event) = deferred_round_bonus.as_ref() {
                let _ = app_state_clone.event_tx.send(event.clone());
            }

            if play_delayed_last_audio {
                if let Some(event) = last_kill_audio {
                    if let Err(error) = play_audio(
                        app_state_clone.clone(),
                        event.kill_count,
                        event.is_headshot,
                        false,
                        event.is_knife_kill,
                        true,
                        false,
                        0,
                        Some("kill".to_string()),
                        false,
                    )
                    .await
                    {
                        error!("Failed to play audio: {}", error);
                    }
                }
            }

            if let Some(event) = deferred_round_bonus {
                if let Err(error) = play_audio(
                    app_state_clone,
                    event.kill_count,
                    event.is_headshot,
                    event.is_first_kill,
                    event.is_knife_kill,
                    event.is_last_kill,
                    event.is_assist,
                    event.money_reward,
                    event.event_kind,
                    event.play_main_animation,
                )
                .await
                {
                    error!("Failed to play audio: {}", error);
                }
            }
        });
    }

    if let Some(kill_event) = kill_event_to_send {
        let _ = app_state.event_tx.send(kill_event);
    }

    if let Some(badge_only_event) = badge_only_event_to_send {
        let _ = app_state.event_tx.send(badge_only_event);
    }
    if let Some(bomb_objective_event) = bomb_objective_event_to_send {
        let _ = app_state.event_tx.send(bomb_objective_event);
    }
    if let Some(hostage_objective_event) = hostage_objective_event_to_send {
        let _ = app_state.event_tx.send(hostage_objective_event);
    }
    if let Some(assist_event) = assist_event_to_send {
        let audio_event = assist_event.clone();
        let _ = app_state.event_tx.send(assist_event);
        let app_state_clone = app_state.clone();
        tokio::spawn(async move {
            let result = play_audio(
                app_state_clone,
                audio_event.kill_count,
                audio_event.is_headshot,
                audio_event.is_first_kill,
                audio_event.is_knife_kill,
                audio_event.is_last_kill,
                audio_event.is_assist,
                audio_event.money_reward,
                audio_event.event_kind.clone(),
                audio_event.play_main_animation,
            )
            .await;

            if let Err(e) = result {
                error!("Failed to play audio: {}", e);
            }
        });
    }
    if let Some(round_bonus_event) = round_bonus_event_to_send {
        let audio_event = round_bonus_event.clone();
        let _ = app_state.event_tx.send(round_bonus_event);
        let app_state_clone = app_state.clone();
        tokio::spawn(async move {
            let result = play_audio(
                app_state_clone,
                audio_event.kill_count,
                audio_event.is_headshot,
                audio_event.is_first_kill,
                audio_event.is_knife_kill,
                audio_event.is_last_kill,
                audio_event.is_assist,
                audio_event.money_reward,
                audio_event.event_kind.clone(),
                audio_event.play_main_animation,
            )
            .await;

            if let Err(e) = result {
                error!("Failed to play audio: {}", e);
            }
        });
    }

    Ok(StatusCode::OK)
}

#[cfg(test)]
fn parse_gsi_body(body: &[u8]) -> Result<Body, GsiBodyError> {
    Ok(parse_gsi_frame(body)?.body)
}

fn parse_gsi_frame(body: &[u8]) -> Result<ParsedGsiBody, GsiBodyError> {
    let mut value: serde_json::Value = serde_json::from_slice(body)?;
    if !has_valid_gsi_token(&value) {
        return Err(GsiBodyError::Unauthorized);
    }
    let is_legacy = is_legacy_gsi_payload(&value);
    let previously = value.get("previously").cloned();
    normalize_forward_compatible_enums(&mut value);
    normalize_legacy_gsi_defaults(&mut value);

    Ok(ParsedGsiBody {
        body: deserialize_forward_compatible_gsi(value)?,
        previously,
        is_legacy,
    })
}

fn is_legacy_gsi_payload(value: &serde_json::Value) -> bool {
    value
        .pointer("/provider/name")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|name| name.to_ascii_lowercase().contains("global offensive"))
        || value
            .pointer("/provider/appid")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|app_id| app_id > u16::MAX as u64)
}

fn normalize_legacy_gsi_defaults(value: &mut serde_json::Value) {
    // Some CS:GO Legacy builds report a depot-specific application id that does not fit
    // the third-party GSI model's u16 field. The service does not use this metadata; map
    // it to CS:GO's public app id while leaving normal CS2 frames (appid 730) untouched.
    if let Some(app_id) = value.pointer_mut("/provider/appid")
        && app_id
            .as_u64()
            .is_some_and(|app_id| app_id > u16::MAX as u64)
    {
        *app_id = serde_json::Value::from(730);
    }

    let Some(map) = value
        .get_mut("map")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    map.entry("num_matches_to_win_series")
        .or_insert(serde_json::Value::from(0));

    for team_key in ["team_ct", "team_t"] {
        let Some(team) = map
            .get_mut(team_key)
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        team.entry("timeouts_remaining")
            .or_insert(serde_json::Value::from(0));
        team.entry("matches_won_this_series")
            .or_insert(serde_json::Value::from(0));
    }
}

fn previous_weapon_evidence(
    previously: Option<&serde_json::Value>,
    tracked_steamid: &str,
    current_weapons: &std::collections::HashMap<String, gsi_cs2::weapon::Weapon>,
) -> PreviousWeaponEvidence {
    let Some(previously) = previously else {
        return PreviousWeaponEvidence::default();
    };
    let weapons = previously
        .get("allplayers")
        .and_then(|allplayers| allplayers.get(tracked_steamid))
        .and_then(|player| player.get("weapons"))
        .or_else(|| {
            let previous_player = previously.get("player")?;
            let belongs_to_another_player = previous_player
                .get("steamid")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|steamid| steamid != tracked_steamid);
            (!belongs_to_another_player)
                .then(|| previous_player.get("weapons"))
                .flatten()
        });
    let Some(weapons) = weapons.and_then(serde_json::Value::as_object) else {
        return PreviousWeaponEvidence::default();
    };

    let active_key = weapons.iter().find_map(|(key, weapon)| {
        (weapon.get("state").and_then(serde_json::Value::as_str) == Some("active"))
            .then(|| key.clone())
    });
    let fired_keys = weapons
        .iter()
        .filter_map(|(key, previous_weapon)| {
            let previous_ammo = previous_weapon.get("ammo_clip")?.as_u64()?;
            let current_ammo = u64::from(current_weapons.get(key)?.ammo_clip);
            (previous_ammo > current_ammo).then(|| key.clone())
        })
        .collect::<Vec<_>>();
    let fired_key = active_key
        .as_ref()
        .filter(|key| fired_keys.contains(key))
        .cloned()
        .or_else(|| fired_keys.into_iter().next());

    PreviousWeaponEvidence {
        active_key,
        fired_key,
    }
}

fn normalize_forward_compatible_enums(value: &mut serde_json::Value) {
    normalize_legacy_weapon_aliases(value);

    const KNOWN_MAP_MODES: &[&str] = &[
        "gungameprogressive",
        "competitive",
        "casual",
        "custom",
        "deathmatch",
        "gungametrbomb",
        "survival",
        "training",
        "scrimcomp2v2",
    ];

    let Some(mode) = value.pointer_mut("/map/mode") else {
        return;
    };
    let Some(mode_name) = mode.as_str() else {
        return;
    };

    if !KNOWN_MAP_MODES.contains(&mode_name) {
        *mode = serde_json::Value::String("custom".to_string());
    }
}

fn normalize_legacy_weapon_aliases(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(weapons) = object
                .get_mut("weapons")
                .and_then(serde_json::Value::as_object_mut)
            {
                for weapon in weapons.values_mut() {
                    let replacement = match weapon.get("name").and_then(serde_json::Value::as_str) {
                        Some("weapon_knife_balisong") => Some("weapon_knife_butterfly"),
                        Some("weapon_knife_cs15") => Some("weapon_knife_css"),
                        Some("weapon_knife_ghost" | "weapon_knife_kunai" | "weapon_knifegg") => {
                            Some("weapon_knife")
                        }
                        _ => None,
                    };
                    if let Some(replacement) = replacement
                        && let Some(name) = weapon.get_mut("name")
                    {
                        *name = serde_json::Value::String(replacement.to_string());
                    }
                }
            }

            for child in object.values_mut() {
                normalize_legacy_weapon_aliases(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_legacy_weapon_aliases(item);
            }
        }
        _ => {}
    }
}

fn select_tracked_player(data: &Body) -> Option<(&Player, &str)> {
    let player = data.player.as_ref()?;
    if player.state.is_some() {
        let steamid = player
            .spectarget
            .as_deref()
            .or(player.steam_id.as_deref())
            .unwrap_or("");
        return Some((player, steamid));
    }

    let spectarget = player.spectarget.as_deref()?;
    if let Some(observed) = data.allplayers.get(spectarget)
        && observed.state.is_some()
    {
        return Some((observed, observed.steam_id.as_deref().unwrap_or(spectarget)));
    }

    data.allplayers
        .iter()
        .find(|(_, candidate)| {
            candidate.state.is_some() && candidate.steam_id.as_deref() == Some(spectarget)
        })
        .map(|(key, observed)| (observed, observed.steam_id.as_deref().unwrap_or(key)))
}

fn select_same_round_view_baseline(
    player_identity_matches: bool,
    current_round: u8,
    cached: Option<PlayerViewBaseline>,
) -> Option<PlayerViewBaseline> {
    (!player_identity_matches)
        .then_some(cached)
        .flatten()
        .filter(|baseline| baseline.round == current_round)
}

fn should_emit_assist(
    is_initialized: bool,
    player_identity_matches: bool,
    current_assists: u16,
    previous_assists: u16,
) -> bool {
    is_initialized && player_identity_matches && current_assists > previous_assists
}

fn deserialize_forward_compatible_gsi(
    mut value: serde_json::Value,
) -> Result<Body, serde_json::Error> {
    for _ in 0..MAX_IGNORED_GSI_FIELDS {
        match serde_path_to_error::deserialize(value.clone()) {
            Ok(body) => return Ok(body),
            Err(error) => {
                let Some(field_name) = unknown_field_name(error.inner()) else {
                    return Err(error.into_inner());
                };
                let path = error.path().clone();
                if !remove_unknown_field(&mut value, &path, &field_name) {
                    return Err(error.into_inner());
                }
            }
        }
    }

    serde_json::from_value(value)
}

fn unknown_field_name(error: &serde_json::Error) -> Option<String> {
    let message = error.to_string();
    let remainder = message.strip_prefix("unknown field `")?;
    let end = remainder.find('`')?;
    Some(remainder[..end].to_string())
}

fn remove_unknown_field(value: &mut serde_json::Value, path: &SerdePath, field_name: &str) -> bool {
    let segments = path.iter().collect::<Vec<_>>();

    if let Some(SerdePathSegment::Map { key }) = segments.last()
        && key == field_name
        && let Some(parent) = value_at_path_mut(value, &segments[..segments.len() - 1])
        && let Some(object) = parent.as_object_mut()
    {
        return object.remove(field_name).is_some();
    }

    value_at_path_mut(value, &segments)
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|object| object.remove(field_name))
        .is_some()
}

fn value_at_path_mut<'a>(
    mut value: &'a mut serde_json::Value,
    segments: &[&SerdePathSegment],
) -> Option<&'a mut serde_json::Value> {
    for segment in segments {
        value = match segment {
            SerdePathSegment::Map { key } => value.as_object_mut()?.get_mut(key)?,
            SerdePathSegment::Seq { index } => value.as_array_mut()?.get_mut(*index)?,
            SerdePathSegment::Enum { .. } | SerdePathSegment::Unknown => return None,
        };
    }
    Some(value)
}

pub(crate) fn resolve_crossfire_streak_count(
    previous_count: u16,
    elapsed_since_last_kill: Option<Duration>,
    mode: CrossfireStreakMode,
    custom_window_ms: u64,
    reset_before_kill: bool,
    kill_delta: u16,
) -> u16 {
    if mode == CrossfireStreakMode::None {
        return kill_delta;
    }

    let timeout = match mode {
        CrossfireStreakMode::None | CrossfireStreakMode::Life => None,
        CrossfireStreakMode::Custom => Some(Duration::from_millis(custom_window_ms)),
        CrossfireStreakMode::Timed5 => Some(Duration::from_secs(5)),
        CrossfireStreakMode::Timed10 => Some(Duration::from_secs(10)),
        CrossfireStreakMode::Timed15 => Some(Duration::from_secs(15)),
    };
    let timed_out = timeout
        .map(|limit| {
            elapsed_since_last_kill
                .map(|elapsed| elapsed >= limit)
                .unwrap_or(previous_count > 0)
        })
        .unwrap_or(false);
    let base = if reset_before_kill || timed_out {
        0
    } else {
        previous_count
    };

    base.saturating_add(kill_delta)
}

fn should_suppress_gsi_after_legacy_bridge(
    is_legacy: bool,
    last_bridge_kill_at: Option<Instant>,
    now: Instant,
) -> bool {
    is_legacy
        && last_bridge_kill_at.is_some_and(|recorded_at| {
            now.saturating_duration_since(recorded_at) <= Duration::from_millis(750)
        })
}

fn should_reset_streak_before_kill(
    round_reset: bool,
    round_end_reordering_active: bool,
    player_identity_matches: bool,
    can_emit_kill: bool,
) -> bool {
    (round_reset && !round_end_reordering_active && !can_emit_kill) || !player_identity_matches
}

fn should_suppress_death_round_end_kill(
    death_reset: bool,
    previous_health: u8,
    current_health: u8,
    score_increased: bool,
    current_round_phase: Option<TrackedRoundPhase>,
    player_team_won_round: Option<bool>,
) -> bool {
    let death_related = death_reset || (previous_health == 0 && current_health == 0);
    death_related
        && (!score_increased
            || (current_round_phase == Some(TrackedRoundPhase::Over)
                && player_team_won_round != Some(true)))
}

fn should_mark_late_final_kill(
    can_emit_kill: bool,
    pending_round_over_age: Option<Duration>,
    player_team_won_round: Option<bool>,
) -> bool {
    can_emit_kill
        && pending_round_over_age
            .map(|age| age <= LATE_FINAL_KILL_WINDOW)
            .unwrap_or(false)
        && player_team_won_round != Some(false)
}

fn resolve_kill_delta(
    is_initialized: bool,
    can_emit_kill: bool,
    current_kills: u16,
    previous_kills: u16,
    pending_round_over_age: Option<Duration>,
) -> u16 {
    if !is_initialized {
        return current_kills;
    }
    if !can_emit_kill {
        return 0;
    }
    if pending_round_over_age
        .map(|age| age <= LATE_FINAL_KILL_WINDOW)
        .unwrap_or(false)
    {
        return 1;
    }

    current_kills.saturating_sub(previous_kills).max(1)
}

fn resolve_kill_observation(
    is_initialized: bool,
    player_identity_matches: bool,
    round_reset: bool,
    current_raw_round_kills: u16,
    previous_raw_round_kills: u16,
    current_match_kills: Option<u16>,
    previous_match_kills: Option<u16>,
    previous_resolved_round_kills: u16,
) -> (u16, u16) {
    if !is_initialized {
        return (current_raw_round_kills, 0);
    }
    if !player_identity_matches {
        let fresh_first_kill = previous_raw_round_kills == 0 && current_raw_round_kills == 1;
        return (current_raw_round_kills, u16::from(fresh_first_kill));
    }

    let raw_delta = current_raw_round_kills.saturating_sub(previous_raw_round_kills);
    let match_delta = current_match_kills
        .zip(previous_match_kills)
        .map(|(current, previous)| current.saturating_sub(previous))
        .unwrap_or(0);
    // Legacy can publish the per-round and match counters in separate frames. Once a kill has
    // already advanced the resolved round count, the other counter catching up is confirmation
    // of that kill rather than a second kill event.
    let raw_already_resolved =
        !round_reset && raw_delta > 0 && current_raw_round_kills <= previous_resolved_round_kills;
    let match_already_resolved = !round_reset
        && match_delta > 0
        && current_raw_round_kills > 0
        && current_raw_round_kills <= previous_resolved_round_kills;
    let effective_raw_delta = if raw_already_resolved { 0 } else { raw_delta };
    let effective_match_delta = if match_already_resolved {
        0
    } else {
        match_delta
    };
    let kill_delta = effective_raw_delta.max(effective_match_delta);
    let raw_counter_rewound = current_raw_round_kills < previous_raw_round_kills;

    let resolved_round_kills = if current_raw_round_kills > 0 {
        current_raw_round_kills
    } else if kill_delta > 0 {
        if round_reset || raw_counter_rewound {
            kill_delta
        } else {
            previous_resolved_round_kills.saturating_add(kill_delta)
        }
    } else if round_reset || raw_counter_rewound {
        0
    } else {
        previous_resolved_round_kills
    };

    (resolved_round_kills, kill_delta)
}

#[allow(clippy::too_many_arguments)]
fn resolve_main_view_kill_observation(
    is_initialized: bool,
    player_identity_matches: bool,
    view_cache_enabled: bool,
    round_reset: bool,
    current_round: u8,
    current_raw_round_kills: u16,
    previous_raw_round_kills: u16,
    current_match_kills: Option<u16>,
    previous_match_kills: Option<u16>,
    previous_resolved_round_kills: u16,
    cached_player: Option<PlayerKillSnapshot>,
) -> (u16, u16) {
    if view_cache_enabled
        && !player_identity_matches
        && let Some(cached_player) = cached_player
    {
        let same_round = cached_player.round == current_round;
        if !same_round {
            return (current_raw_round_kills, 0);
        }
        return resolve_kill_observation(
            is_initialized,
            true,
            round_reset,
            current_raw_round_kills,
            cached_player.raw_round_kills,
            current_match_kills,
            cached_player.match_kills,
            cached_player.resolved_round_kills,
        );
    }

    resolve_kill_observation(
        is_initialized,
        player_identity_matches,
        round_reset,
        current_raw_round_kills,
        previous_raw_round_kills,
        current_match_kills,
        previous_match_kills,
        previous_resolved_round_kills,
    )
}

fn resolve_kill_weapon_key(
    current_active_key: Option<&str>,
    previous_active_key: Option<&str>,
    fired_weapon_key: Option<&str>,
) -> Option<String> {
    fired_weapon_key
        .or(previous_active_key)
        .or(current_active_key)
        .map(str::to_string)
}

fn should_promote_pending_last_kill(
    age: Duration,
    death_reset: bool,
    player_team_won_round: Option<bool>,
) -> bool {
    age <= FINAL_KILL_GRACE_WINDOW
        && player_team_won_round != Some(false)
        && (!death_reset || player_team_won_round == Some(true))
}

fn bomb_state_is_exploded(bomb_state: Option<&BombState>, raw_bomb_state: Option<&str>) -> bool {
    matches!(bomb_state, Some(BombState::Exploded)) || raw_bomb_state == Some("exploded")
}

fn round_end_allows_delayed_last_kill(
    bomb_state: Option<&BombState>,
    raw_bomb_state: Option<&str>,
    round_outcome: Option<&str>,
) -> bool {
    let ended_by_bomb_objective =
        matches!(bomb_state, Some(BombState::Defused | BombState::Exploded))
            || matches!(raw_bomb_state, Some("defused" | "exploded"));
    let ended_by_non_kill_outcome = matches!(
        round_outcome,
        Some("ct_win_defuse" | "t_win_bomb" | "ct_win_time" | "t_win_time" | "ct_win_rescue")
    );

    !ended_by_bomb_objective && !ended_by_non_kill_outcome
}

#[cfg(test)]
mod tests {
    use super::{
        CrossfireStreakMode, bomb_state_is_exploded, is_knife_weapon, is_legacy_gsi_payload,
        normalize_forward_compatible_enums, normalize_legacy_gsi_defaults,
        opponent_team_display_name, parse_gsi_body, parse_gsi_frame, previous_weapon_evidence,
        resolve_crossfire_streak_count, resolve_is_knife_kill, resolve_kill_delta,
        resolve_kill_observation, resolve_kill_weapon_key, resolve_main_view_kill_observation,
        round_end_allows_delayed_last_kill, select_same_round_view_baseline, select_tracked_player,
        should_emit_assist, should_mark_late_final_kill, should_promote_pending_last_kill,
        should_reset_streak_before_kill, should_suppress_death_round_end_kill,
        should_suppress_gsi_after_legacy_bridge,
    };
    use crate::util::state::{PlayerKillSnapshot, PlayerViewBaseline};
    use gsi_cs2::round::BombState;
    use gsi_cs2::team::TeamClass;
    use gsi_cs2::weapon::WeaponName;
    use serde_json::json;
    use std::time::{Duration, Instant};

    #[test]
    fn bridge_deduplication_is_strictly_legacy_only() {
        let now = Instant::now();
        assert!(should_suppress_gsi_after_legacy_bridge(
            true,
            Some(now - Duration::from_millis(100)),
            now
        ));
        assert!(!should_suppress_gsi_after_legacy_bridge(
            false,
            Some(now - Duration::from_millis(100)),
            now
        ));
        assert!(!should_suppress_gsi_after_legacy_bridge(
            true,
            Some(now - Duration::from_secs(1)),
            now
        ));
    }

    #[test]
    fn objective_round_end_does_not_replay_a_pending_kill_as_the_last_kill() {
        assert!(!round_end_allows_delayed_last_kill(
            Some(&BombState::Defused),
            Some("defused"),
            Some("ct_win_defuse")
        ));
        assert!(!round_end_allows_delayed_last_kill(
            Some(&BombState::Exploded),
            Some("exploded"),
            Some("t_win_bomb")
        ));
        assert!(!round_end_allows_delayed_last_kill(
            None,
            None,
            Some("ct_win_rescue")
        ));
        assert!(round_end_allows_delayed_last_kill(
            None,
            None,
            Some("ct_win_elimination")
        ));
    }

    #[test]
    fn bomb_explosion_kill_deltas_do_not_emit_player_kill_audio() {
        assert!(bomb_state_is_exploded(
            Some(&BombState::Exploded),
            Some("exploded")
        ));
        assert!(!bomb_state_is_exploded(None, None));
    }

    #[test]
    fn knife_equipped_in_the_kill_update_is_detected_without_prior_history() {
        assert!(resolve_is_knife_kill(Some(true), false));
    }

    #[test]
    fn gun_kill_after_switching_from_knife_is_not_marked_as_a_knife_kill() {
        assert!(!resolve_is_knife_kill(Some(false), true));
    }

    #[test]
    fn recent_knife_is_used_only_when_the_kill_frame_has_no_weapon() {
        assert!(resolve_is_knife_kill(None, true));
    }

    #[test]
    fn kill_then_switch_prefers_the_weapon_that_fired() {
        assert_eq!(
            resolve_kill_weapon_key(Some("weapon_1"), Some("weapon_0"), Some("weapon_0")),
            Some("weapon_0".to_string())
        );
    }

    #[test]
    fn switch_then_kill_prefers_the_new_weapon_when_its_ammo_dropped() {
        assert_eq!(
            resolve_kill_weapon_key(Some("weapon_1"), Some("weapon_0"), Some("weapon_1")),
            Some("weapon_1".to_string())
        );
    }

    #[test]
    fn knife_kill_then_switch_uses_the_previous_active_weapon_without_ammo_evidence() {
        assert_eq!(
            resolve_kill_weapon_key(Some("weapon_1"), Some("weapon_knife"), None),
            Some("weapon_knife".to_string())
        );
    }

    #[test]
    fn gsi_previously_identifies_the_weapon_used_before_a_fast_switch() {
        let payload = json!({
            "auth": { "token": "killconfirm" },
            "player": {
                "steamid": "76561198000000001",
                "name": "Fast Switch",
                "team": "CT",
                "activity": "playing",
                "match_stats": { "kills": 2, "assists": 0, "deaths": 0, "mvps": 0, "score": 4 },
                "state": {
                    "health": 100, "armor": 100, "helmet": true, "flashed": 0,
                    "smoked": 0, "burning": 0, "money": 3000, "round_kills": 1,
                    "round_killhs": 0, "equip_value": 4000
                },
                "weapons": {
                    "weapon_0": {
                        "name": "weapon_ak47", "paintkit": "default", "type": "Rifle",
                        "state": "holstered", "ammo_clip": 29, "ammo_clip_max": 30,
                        "ammo_reserve": 90
                    },
                    "weapon_1": {
                        "name": "weapon_knife", "paintkit": "default", "type": "Knife",
                        "state": "active", "ammo_clip": 0, "ammo_clip_max": 0,
                        "ammo_reserve": 0
                    }
                }
            },
            "previously": {
                "player": {
                    "weapons": {
                        "weapon_0": { "state": "active", "ammo_clip": 30 },
                        "weapon_1": { "state": "holstered" }
                    }
                }
            }
        });
        let frame = parse_gsi_frame(payload.to_string().as_bytes()).unwrap();
        let player = frame.body.player.as_ref().unwrap();
        let evidence = previous_weapon_evidence(
            frame.previously.as_ref(),
            "76561198000000001",
            &player.weapons,
        );

        assert_eq!(evidence.active_key.as_deref(), Some("weapon_0"));
        assert_eq!(evidence.fired_key.as_deref(), Some("weapon_0"));
        assert_eq!(
            resolve_kill_weapon_key(
                Some("weapon_1"),
                evidence.active_key.as_deref(),
                evidence.fired_key.as_deref(),
            )
            .as_deref(),
            Some("weapon_0")
        );

        let observer_previous = json!({
            "allplayers": {
                "76561198000000001": {
                    "weapons": {
                        "weapon_0": { "state": "active", "ammo_clip": 30 },
                        "weapon_1": { "state": "holstered" }
                    }
                }
            }
        });
        let observer_evidence = previous_weapon_evidence(
            Some(&observer_previous),
            "76561198000000001",
            &player.weapons,
        );
        assert_eq!(observer_evidence.active_key.as_deref(), Some("weapon_0"));
        assert_eq!(observer_evidence.fired_key.as_deref(), Some("weapon_0"));

        let mut reload_previous = observer_previous;
        reload_previous["allplayers"]["76561198000000001"]["weapons"]["weapon_0"]["ammo_clip"] =
            json!(20);
        let reload_evidence =
            previous_weapon_evidence(Some(&reload_previous), "76561198000000001", &player.weapons);
        assert_eq!(reload_evidence.fired_key, None);

        let previous_other_view = json!({
            "player": {
                "steamid": "76561198000000002",
                "weapons": {
                    "weapon_0": { "state": "active", "ammo_clip": 30 }
                }
            }
        });
        let other_view_evidence = previous_weapon_evidence(
            Some(&previous_other_view),
            "76561198000000001",
            &player.weapons,
        );
        assert_eq!(other_view_evidence.active_key, None);
        assert_eq!(other_view_evidence.fired_key, None);
    }

    #[test]
    fn retakes_falls_back_to_match_kills_when_round_kills_stay_zero() {
        assert_eq!(
            resolve_kill_observation(true, true, false, 0, 0, Some(8), Some(7), 0),
            (1, 1)
        );
        assert_eq!(
            resolve_kill_observation(true, true, false, 0, 0, Some(9), Some(8), 1),
            (2, 1)
        );
    }

    #[test]
    fn modes_with_both_counters_only_count_each_kill_once() {
        assert_eq!(
            resolve_kill_observation(true, true, false, 2, 1, Some(8), Some(7), 1),
            (2, 1)
        );
    }

    #[test]
    fn delayed_match_counter_does_not_repeat_a_round_counter_kill() {
        assert_eq!(
            resolve_kill_observation(true, true, false, 1, 0, Some(4), Some(4), 0),
            (1, 1)
        );
        assert_eq!(
            resolve_kill_observation(true, true, false, 1, 1, Some(5), Some(4), 1),
            (1, 0)
        );
    }

    #[test]
    fn delayed_round_counter_does_not_repeat_a_match_counter_kill() {
        assert_eq!(
            resolve_kill_observation(true, true, false, 0, 0, Some(5), Some(4), 0),
            (1, 1)
        );
        assert_eq!(
            resolve_kill_observation(true, true, false, 1, 0, Some(5), Some(5), 1),
            (1, 0)
        );
    }

    #[test]
    fn main_view_switch_preserves_a_fresh_first_kill_without_replaying_history() {
        assert_eq!(
            resolve_kill_observation(true, false, false, 1, 0, Some(3), Some(4), 0),
            (1, 1)
        );
        assert_eq!(
            resolve_kill_observation(true, false, false, 2, 0, Some(6), Some(4), 0),
            (2, 0)
        );
    }

    #[test]
    fn returning_main_view_uses_that_players_cached_kill_baseline() {
        let cached_bot = PlayerKillSnapshot {
            round: 12,
            resolved_round_kills: 1,
            raw_round_kills: 1,
            match_kills: Some(7),
        };

        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                false,
                true,
                false,
                12,
                2,
                0,
                Some(8),
                Some(6),
                0,
                Some(cached_bot),
            ),
            (2, 1)
        );
    }

    #[test]
    fn observation_switch_uses_only_the_selected_players_same_round_baseline() {
        let observed_player = PlayerViewBaseline {
            round: 12,
            round_headshot_kills: 2,
            assists: 4,
            deaths: 3,
            score: 18,
            health: 72,
        };

        assert_eq!(
            select_same_round_view_baseline(false, 12, Some(observed_player))
                .map(|baseline| baseline.assists),
            Some(4)
        );
        assert!(select_same_round_view_baseline(true, 12, Some(observed_player)).is_none());
        assert!(select_same_round_view_baseline(false, 13, Some(observed_player)).is_none());
    }

    #[test]
    fn assist_delta_is_never_carried_across_an_observation_switch() {
        assert!(!should_emit_assist(true, false, 8, 2));
        assert!(!should_emit_assist(false, true, 8, 2));
        assert!(should_emit_assist(true, true, 3, 2));
    }

    #[test]
    fn stable_cs2_player_ignores_other_view_cache() {
        let unrelated_cached_view = PlayerKillSnapshot {
            round: 12,
            resolved_round_kills: 9,
            raw_round_kills: 9,
            match_kills: Some(99),
        };

        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                true,
                false,
                false,
                12,
                2,
                1,
                Some(8),
                Some(7),
                1,
                Some(unrelated_cached_view),
            ),
            (2, 1)
        );
    }

    #[test]
    fn cs2_view_switch_does_not_use_the_legacy_mod_cache() {
        let legacy_only_cache = PlayerKillSnapshot {
            round: 12,
            resolved_round_kills: 1,
            raw_round_kills: 1,
            match_kills: Some(7),
        };

        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                false,
                false,
                false,
                12,
                2,
                0,
                Some(8),
                Some(6),
                0,
                Some(legacy_only_cache),
            ),
            (2, 0)
        );
    }

    #[test]
    fn legacy_view_cache_is_selected_only_for_csgo_payloads() {
        assert!(is_legacy_gsi_payload(&json!({
            "provider": { "name": "Counter-Strike: Global Offensive", "appid": 730 }
        })));
        assert!(is_legacy_gsi_payload(&json!({
            "provider": { "name": "Counter-Strike: Global Offensive", "appid": 4465480 }
        })));
        assert!(!is_legacy_gsi_payload(&json!({
            "provider": { "name": "Counter-Strike 2", "appid": 730 }
        })));
    }

    #[test]
    fn first_sighting_of_a_view_does_not_replay_multiple_historical_kills() {
        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                false,
                true,
                false,
                12,
                2,
                0,
                Some(8),
                Some(6),
                0,
                None,
            ),
            (2, 0)
        );
    }

    #[test]
    fn cached_view_establishes_a_baseline_on_a_new_round() {
        let cached_bot = PlayerKillSnapshot {
            round: 11,
            resolved_round_kills: 2,
            raw_round_kills: 2,
            match_kills: Some(7),
        };

        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                false,
                true,
                false,
                12,
                1,
                0,
                Some(8),
                Some(6),
                0,
                Some(cached_bot),
            ),
            (1, 0)
        );
    }

    #[test]
    fn taking_control_only_establishes_a_new_round_baseline() {
        let cached_bot_from_previous_round = PlayerKillSnapshot {
            round: 11,
            resolved_round_kills: 1,
            raw_round_kills: 1,
            match_kills: Some(7),
        };

        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                false,
                true,
                false,
                12,
                1,
                0,
                Some(8),
                Some(6),
                0,
                Some(cached_bot_from_previous_round),
            ),
            (1, 0)
        );

        let controlled_bot_baseline = PlayerKillSnapshot {
            round: 12,
            resolved_round_kills: 1,
            raw_round_kills: 1,
            match_kills: Some(8),
        };
        assert_eq!(
            resolve_main_view_kill_observation(
                true,
                false,
                true,
                false,
                12,
                2,
                0,
                Some(9),
                Some(6),
                0,
                Some(controlled_bot_baseline),
            ),
            (2, 1)
        );
    }

    #[test]
    fn respawn_modes_start_a_new_counter_when_round_kills_rewind() {
        assert_eq!(
            resolve_kill_observation(true, true, false, 0, 2, Some(9), Some(8), 2),
            (1, 1)
        );
    }

    #[test]
    fn replay_payload_accepts_spectator_only_fields() {
        let payload = json!({
            "auth": { "token": "killconfirm" },
            "map": {
                "mode": "competitive",
                "name": "de_dust2",
                "phase": "live",
                "round": 3,
                "team_ct": {
                    "name": null,
                    "flag": null,
                    "score": 2,
                    "consecutive_round_losses": 0,
                    "timeouts_remaining": 1,
                    "matches_won_this_series": 0,
                    "logo": "spectator-only-field"
                },
                "team_t": {
                    "name": null,
                    "flag": null,
                    "score": 1,
                    "consecutive_round_losses": 1,
                    "timeouts_remaining": 1,
                    "matches_won_this_series": 0
                },
                "num_matches_to_win_series": 0,
                "current_spectators": 12,
                "souvenirs_total": 0
            },
            "player": {
                "steamid": "76561198000000001",
                "name": "Observed Player",
                "observer_slot": 1,
                "team": "CT",
                "activity": "playing",
                "match_stats": {
                    "kills": 4,
                    "assists": 1,
                    "deaths": 2,
                    "mvps": 0,
                    "score": 9
                },
                "state": {
                    "health": 100,
                    "armor": 100,
                    "helmet": true,
                    "flashed": 0,
                    "smoked": 0,
                    "burning": 0,
                    "money": 4200,
                    "round_kills": 1,
                    "round_killhs": 0,
                    "equip_value": 5100,
                    "replay_tick": 12345
                },
                "weapons": {}
            }
        });

        let parsed = parse_gsi_body(payload.to_string().as_bytes());
        assert!(
            parsed.is_ok(),
            "replay/spectator payload should remain forward compatible"
        );
    }

    #[test]
    fn newly_added_game_modes_do_not_reject_the_entire_gsi_frame() {
        let payload = json!({
            "auth": { "token": "killconfirm" },
            "map": {
                "mode": "retakes",
                "name": "de_mirage",
                "phase": "live",
                "round": 2,
                "team_ct": {
                    "name": null,
                    "flag": null,
                    "score": 1,
                    "consecutive_round_losses": 0,
                    "timeouts_remaining": 0,
                    "matches_won_this_series": 0
                },
                "team_t": {
                    "name": null,
                    "flag": null,
                    "score": 1,
                    "consecutive_round_losses": 0,
                    "timeouts_remaining": 0,
                    "matches_won_this_series": 0
                },
                "num_matches_to_win_series": 0
            },
            "player": {
                "steamid": "76561198000000001",
                "name": "Retake Player",
                "team": "CT",
                "activity": "playing",
                "match_stats": {
                    "kills": 1,
                    "assists": 0,
                    "deaths": 0,
                    "mvps": 0,
                    "score": 2
                },
                "state": {
                    "health": 100,
                    "armor": 100,
                    "helmet": true,
                    "flashed": 0,
                    "smoked": 0,
                    "burning": 0,
                    "money": 0,
                    "round_kills": 0,
                    "round_killhs": 0,
                    "equip_value": 3000
                },
                "weapons": {}
            }
        });

        assert!(parse_gsi_body(payload.to_string().as_bytes()).is_ok());
    }

    #[test]
    fn cs2_provider_appid_is_unchanged_by_legacy_normalization() {
        let mut payload = json!({
            "provider": {
                "name": "Counter-Strike 2",
                "appid": 730,
                "version": 14000,
                "steamid": "76561198000000001",
                "timestamp": 1786376911
            }
        });

        normalize_legacy_gsi_defaults(&mut payload);

        assert_eq!(
            payload
                .pointer("/provider/appid")
                .and_then(|id| id.as_u64()),
            Some(730)
        );
    }

    #[test]
    fn cs2_standard_weapon_names_are_unchanged_by_legacy_normalization() {
        let mut payload = json!({
            "player": {
                "weapons": {
                    "weapon_0": { "name": "weapon_knife_butterfly" },
                    "weapon_1": { "name": "weapon_ak47" }
                }
            }
        });

        normalize_forward_compatible_enums(&mut payload);

        assert_eq!(
            payload
                .pointer("/player/weapons/weapon_0/name")
                .and_then(|name| name.as_str()),
            Some("weapon_knife_butterfly")
        );
        assert_eq!(
            payload
                .pointer("/player/weapons/weapon_1/name")
                .and_then(|name| name.as_str()),
            Some("weapon_ak47")
        );

        for (legacy_name, expected_name) in [
            ("weapon_knife_balisong", "weapon_knife_butterfly"),
            ("weapon_knife_cs15", "weapon_knife_css"),
            ("weapon_knife_ghost", "weapon_knife"),
            ("weapon_knife_kunai", "weapon_knife"),
            ("weapon_knifegg", "weapon_knife"),
        ] {
            let mut legacy_payload = json!({
                "player": {
                    "weapons": {
                        "weapon_0": {
                            "name": legacy_name,
                            "paintkit": "default",
                            "type": "Knife",
                            "state": "active",
                            "ammo_clip": 0,
                            "ammo_clip_max": 0,
                            "ammo_reserve": 0
                        }
                    }
                }
            });
            normalize_forward_compatible_enums(&mut legacy_payload);
            assert_eq!(
                legacy_payload
                    .pointer("/player/weapons/weapon_0/name")
                    .and_then(|name| name.as_str()),
                Some(expected_name)
            );
            let weapon: gsi_cs2::weapon::Weapon = serde_json::from_value(
                legacy_payload
                    .pointer("/player/weapons/weapon_0")
                    .expect("legacy knife payload should contain a weapon")
                    .clone(),
            )
            .expect("normalized legacy knife should deserialize");
            assert!(is_knife_weapon(&weapon.name, weapon.r#type.as_ref()));
        }
    }

    #[test]
    fn legacy_gsi_payload_accepts_team_fields_added_after_csgo() {
        let payload = json!({
            "auth": { "token": "killconfirm" },
            "provider": {
                "name": "Counter-Strike: Global Offensive",
                "appid": 4465480,
                "version": 13804,
                "steamid": "76561198000000001",
                "timestamp": 1786376911
            },
            "map": {
                "mode": "competitive",
                "name": "de_dust2",
                "phase": "live",
                "round": 4,
                "team_ct": {
                    "name": null,
                    "flag": null,
                    "score": 2,
                    "consecutive_round_losses": 0
                },
                "team_t": {
                    "name": null,
                    "flag": null,
                    "score": 2,
                    "consecutive_round_losses": 1
                }
            },
            "player": {
                "steamid": "76561198000000001",
                "name": "Legacy Player",
                "team": "CT",
                "activity": "playing",
                "match_stats": {
                    "kills": 5,
                    "assists": 1,
                    "deaths": 3,
                    "mvps": 1,
                    "score": 12
                },
                "state": {
                    "health": 100,
                    "armor": 100,
                    "helmet": true,
                    "flashed": 0,
                    "smoked": 0,
                    "burning": 0,
                    "money": 4200,
                    "round_kills": 1,
                    "round_killhs": 0,
                    "equip_value": 5100
                },
                "weapons": {
                    "weapon_0": {
                        "name": "weapon_knife_balisong",
                        "paintkit": "default",
                        "type": "Knife",
                        "state": "active",
                        "ammo_clip": 0,
                        "ammo_clip_max": 0,
                        "ammo_reserve": 0
                    },
                    "weapon_1": {
                        "name": "weapon_knife_cs15",
                        "paintkit": "default",
                        "type": "Knife",
                        "state": "holstered",
                        "ammo_clip": 0,
                        "ammo_clip_max": 0,
                        "ammo_reserve": 0
                    }
                }
            }
        });

        let parsed = parse_gsi_body(payload.to_string().as_bytes())
            .expect("Legacy provider metadata should not reject the GSI frame");
        assert_eq!(parsed.provider.map(|provider| provider.app_id), Some(730));
        let weapons = parsed
            .player
            .map(|player| player.weapons)
            .expect("Legacy knives should remain available for knife-kill detection");
        assert!(
            weapons
                .values()
                .any(|weapon| matches!(weapon.name, WeaponName::KnifeButterfly))
        );
        assert!(
            weapons
                .values()
                .any(|weapon| matches!(weapon.name, WeaponName::KnifeClassic))
        );
    }

    #[test]
    fn observer_payload_tracks_the_current_spectarget_from_allplayers() {
        let payload = json!({
            "auth": { "token": "killconfirm" },
            "player": {
                "activity": "playing",
                "spectarget": "76561198000000042"
            },
            "allplayers": {
                "76561198000000042": {
                    "name": "Observed Player",
                    "team": "T",
                    "state": {
                        "health": 84,
                        "armor": 50,
                        "helmet": false,
                        "flashed": 0,
                        "smoked": 0,
                        "burning": 0,
                        "money": 2300,
                        "round_kills": 2,
                        "round_killhs": 1,
                        "equip_value": 3600
                    },
                    "weapons": {}
                }
            }
        });
        let body = parse_gsi_body(payload.to_string().as_bytes()).unwrap();
        let (player, steamid) = select_tracked_player(&body).unwrap();

        assert_eq!(steamid, "76561198000000042");
        assert_eq!(player.name.as_deref(), Some("Observed Player"));
        assert_eq!(
            player.state.as_ref().map(|state| state.round_kills),
            Some(2)
        );
    }

    #[test]
    fn replay_payload_uses_spectarget_identity_when_player_state_is_present() {
        let payload = json!({
            "auth": { "token": "killconfirm" },
            "player": {
                "steamid": "76561198000000001",
                "spectarget": "76561198000000042",
                "state": {
                    "health": 84,
                    "armor": 50,
                    "helmet": false,
                    "flashed": 0,
                    "smoked": 0,
                    "burning": 0,
                    "money": 2300,
                    "round_kills": 2,
                    "round_killhs": 1,
                    "equip_value": 3600
                },
                "weapons": {}
            }
        });
        let body = parse_gsi_body(payload.to_string().as_bytes()).unwrap();
        let (_, steamid) = select_tracked_player(&body).unwrap();

        assert_eq!(steamid, "76561198000000042");
    }

    #[test]
    fn knife_name_is_detected_when_gsi_omits_the_weapon_type() {
        assert!(is_knife_weapon(&WeaponName::KnifeKarambit, None));
        assert!(!is_knife_weapon(&WeaponName::AK47, None));
    }

    #[test]
    fn life_mode_keeps_count_without_a_time_limit() {
        assert_eq!(
            resolve_crossfire_streak_count(
                1,
                Some(Duration::from_secs(120)),
                CrossfireStreakMode::Life,
                1_000,
                false,
                1,
            ),
            2
        );
    }

    #[test]
    fn timed_modes_reset_at_the_selected_interval() {
        for (mode, seconds) in [
            (CrossfireStreakMode::Timed5, 5),
            (CrossfireStreakMode::Timed10, 10),
            (CrossfireStreakMode::Timed15, 15),
        ] {
            assert_eq!(
                resolve_crossfire_streak_count(
                    1,
                    Some(Duration::from_secs(seconds)),
                    mode,
                    1_000,
                    false,
                    1,
                ),
                1,
                "mode {mode:?} should reset at {seconds} seconds"
            );
        }
    }

    #[test]
    fn timed_modes_keep_the_streak_before_the_selected_interval() {
        for (mode, seconds) in [
            (CrossfireStreakMode::Timed5, 5),
            (CrossfireStreakMode::Timed10, 10),
            (CrossfireStreakMode::Timed15, 15),
        ] {
            assert_eq!(
                resolve_crossfire_streak_count(
                    1,
                    Some(Duration::from_secs(seconds - 1)),
                    mode,
                    1_000,
                    false,
                    1,
                ),
                2,
                "mode {mode:?} should remain active before {seconds} seconds"
            );
        }
    }

    #[test]
    fn custom_subsecond_window_resets_after_the_configured_delay() {
        assert_eq!(
            resolve_crossfire_streak_count(
                2,
                Some(Duration::from_millis(500)),
                CrossfireStreakMode::Custom,
                400,
                false,
                1,
            ),
            1
        );
        assert_eq!(
            resolve_crossfire_streak_count(
                2,
                Some(Duration::from_millis(399)),
                CrossfireStreakMode::Custom,
                400,
                false,
                1,
            ),
            3
        );
    }

    #[test]
    fn no_window_never_combines_separate_kills() {
        assert_eq!(
            resolve_crossfire_streak_count(
                6,
                Some(Duration::from_millis(1)),
                CrossfireStreakMode::None,
                1_000,
                false,
                1,
            ),
            1
        );
    }

    #[test]
    fn scope_reset_starts_a_new_streak() {
        assert_eq!(
            resolve_crossfire_streak_count(
                4,
                Some(Duration::from_secs(1)),
                CrossfireStreakMode::Life,
                1_000,
                true,
                1,
            ),
            1
        );
    }

    #[test]
    fn round_end_does_not_reset_before_a_kill_from_the_same_update() {
        assert!(!should_reset_streak_before_kill(true, true, true, true));
    }

    #[test]
    fn round_end_before_the_late_kill_preserves_the_streak_temporarily() {
        assert!(!should_reset_streak_before_kill(true, true, true, false));
    }

    #[test]
    fn round_end_without_a_kill_still_resets_the_streak() {
        assert!(should_reset_streak_before_kill(true, false, true, false));
    }

    #[test]
    fn player_identity_change_still_resets_the_streak() {
        assert!(should_reset_streak_before_kill(false, false, false, false));
    }

    #[test]
    fn death_at_round_end_does_not_emit_a_kill_for_the_losing_player() {
        assert!(should_suppress_death_round_end_kill(
            true,
            100,
            0,
            false,
            Some(super::TrackedRoundPhase::Over),
            Some(false),
        ));
    }

    #[test]
    fn dead_player_count_rebound_does_not_emit_a_kill_without_an_outcome() {
        assert!(should_suppress_death_round_end_kill(
            false,
            0,
            0,
            false,
            Some(super::TrackedRoundPhase::Over),
            None,
        ));
    }

    #[test]
    fn winning_trade_is_not_suppressed() {
        assert!(!should_suppress_death_round_end_kill(
            true,
            100,
            0,
            true,
            Some(super::TrackedRoundPhase::Over),
            Some(true),
        ));
    }

    #[test]
    fn posthumous_live_kill_with_score_evidence_is_not_suppressed() {
        assert!(!should_suppress_death_round_end_kill(
            false,
            0,
            0,
            true,
            Some(super::TrackedRoundPhase::Live),
            None,
        ));
    }

    #[test]
    fn kill_arriving_just_after_round_over_is_marked_as_final() {
        assert!(should_mark_late_final_kill(
            true,
            Some(Duration::from_millis(50)),
            Some(true),
        ));
    }

    #[test]
    fn late_kill_is_not_marked_as_final_after_a_round_loss() {
        assert!(!should_mark_late_final_kill(
            true,
            Some(Duration::from_millis(50)),
            Some(false),
        ));
    }

    #[test]
    fn kill_outside_the_reorder_window_is_not_marked_as_final() {
        assert!(!should_mark_late_final_kill(
            true,
            Some(Duration::from_millis(251)),
            Some(true),
        ));
    }

    #[test]
    fn late_final_update_contributes_exactly_one_kill_after_counter_rebase() {
        assert_eq!(
            resolve_kill_delta(true, true, 5, 0, Some(Duration::from_millis(50)),),
            1
        );
    }

    #[test]
    fn ordinary_update_keeps_multi_kill_deltas() {
        assert_eq!(resolve_kill_delta(true, true, 5, 3, None), 2);
    }

    #[test]
    fn recent_kill_is_not_promoted_when_the_player_dies_and_loses() {
        assert!(!should_promote_pending_last_kill(
            Duration::from_millis(100),
            true,
            Some(false),
        ));
    }

    #[test]
    fn recent_kill_is_not_promoted_when_a_death_has_no_round_outcome() {
        assert!(!should_promote_pending_last_kill(
            Duration::from_millis(100),
            true,
            None,
        ));
    }

    #[test]
    fn winning_trade_can_still_promote_the_players_recent_kill() {
        assert!(should_promote_pending_last_kill(
            Duration::from_millis(100),
            true,
            Some(true),
        ));
    }

    #[test]
    fn stale_pending_kill_is_never_promoted() {
        assert!(!should_promote_pending_last_kill(
            Duration::from_millis(1_501),
            false,
            Some(true),
        ));
    }

    #[test]
    fn kill_target_uses_the_opposing_team_name() {
        assert_eq!(
            opponent_team_display_name(Some(&TeamClass::CT)),
            Some("\u{6050}\u{6016}\u{5206}\u{5b50}".to_string())
        );
        assert_eq!(
            opponent_team_display_name(Some(&TeamClass::T)),
            Some("\u{53cd}\u{6050}\u{7cbe}\u{82f1}".to_string())
        );
        assert_eq!(opponent_team_display_name(None), None);
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn map_round_phase(phase: &RoundPhase) -> TrackedRoundPhase {
    match phase {
        RoundPhase::Live => TrackedRoundPhase::Live,
        RoundPhase::FreezeTime => TrackedRoundPhase::FreezeTime,
        RoundPhase::Over => TrackedRoundPhase::Over,
    }
}

fn infer_round_phase_from_kills(current_kills: u16) -> Option<TrackedRoundPhase> {
    if current_kills == 0 {
        return Some(TrackedRoundPhase::FreezeTime);
    }

    None
}
