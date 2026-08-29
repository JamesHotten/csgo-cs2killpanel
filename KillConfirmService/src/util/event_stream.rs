use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Arc,
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, warn};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_SZ, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6432KEY,
    RegGetValueW,
};

use crate::soundpack::Preset;
use crate::soundpack::sound::{play_audio, warm_audio_cache};
use crate::util::logging::service_log;
use crate::util::playback::{get_output_stream_with_name, output_device_names};

use super::money_rules;
use super::state::{
    AppState, CrossfireStreakMode, EventSoundMode, EventSoundRoute, EventSoundSettings, KillEvent,
    MoneyRewardMode, format_streak_setting, parse_streak_setting,
};

#[derive(Debug, Deserialize)]
pub struct TestEventQuery {
    pub headshot: Option<bool>,
    pub knife: Option<bool>,
    pub assist: Option<bool>,
    pub first: Option<bool>,
    pub last: Option<bool>,
    pub main: Option<bool>,
    pub audio: Option<bool>,
    pub animation: Option<String>,
    pub event_kind: Option<String>,
    pub weapon_badge: Option<String>,
    pub weapon_name: Option<String>,
    pub money_reward: Option<u16>,
    pub round_number: Option<u8>,
    pub money_epoch: Option<u32>,
    pub player_name: Option<String>,
    pub target_name: Option<String>,
    pub steamid: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: &'static str,
}

#[derive(Debug, Serialize)]
pub struct GsiStatusResponse {
    pub posts: u64,
    pub parse_errors: u64,
    pub last_post_unix_ms: Option<u64>,
    pub last_post_age_ms: Option<u64>,
    pub last_parse_error_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SoundPackRequest {
    pub preset: String,
    pub custom_path: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VolumeRequest {
    pub percent: u32,
}

#[derive(Debug, Deserialize)]
pub struct AudioDeviceRequest {
    pub device: String,
}

#[derive(Debug, Serialize)]
pub struct AudioDevicesResponse {
    pub devices: Vec<String>,
    pub selected: String,
    pub active: String,
}

#[derive(Debug, Deserialize)]
pub struct MoneyModeRequest {
    pub mode: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct CrossfireSettingsRequest {
    pub active: bool,
    pub streak_mode: String,
    pub first_kill_special_audio: bool,
    pub last_kill_special_audio: bool,
    #[serde(default)]
    pub headshot_special_audio_priority: bool,
    #[serde(default = "default_true")]
    pub knife_special_audio_priority: bool,
    #[serde(default)]
    pub assist_audio_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct StreakSettingsRequest {
    pub active: bool,
    pub streak_mode: String,
    #[serde(default)]
    pub assist_audio_enabled: bool,
    #[serde(default)]
    pub assist_audio_setting_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CsolSettingsRequest {
    #[serde(default)]
    pub voice_picks: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub special_voice_priority: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EventSoundRouteRequest {
    pub mode: String,
    #[serde(default)]
    pub custom_path: String,
}

#[derive(Debug, Deserialize)]
pub struct EventSoundSettingsRequest {
    pub active: bool,
    pub normal: EventSoundRouteRequest,
    pub headshot: EventSoundRouteRequest,
    pub knife: EventSoundRouteRequest,
    pub assist: EventSoundRouteRequest,
}

#[derive(Debug, Deserialize)]
pub struct SpectatorSettingsRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub spectated_player_enabled: Option<bool>,
    #[serde(default)]
    pub replay_enabled: Option<bool>,
    #[serde(default)]
    pub controlled_bot_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SoundPackResponse {
    pub preset: String,
    pub display_name: String,
    pub available: Vec<SoundPackOption>,
}

#[derive(Debug, Serialize)]
pub struct MoneyModeResponse {
    pub mode: &'static str,
    pub available: Vec<MoneyModeOption>,
}

#[derive(Debug, Serialize)]
pub struct CrossfireSettingsResponse {
    pub active: bool,
    pub streak_mode: String,
    pub first_kill_special_audio: bool,
    pub last_kill_special_audio: bool,
    pub headshot_special_audio_priority: bool,
    pub knife_special_audio_priority: bool,
    pub assist_audio_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct StreakSettingsResponse {
    pub active: bool,
    pub streak_mode: String,
    pub assist_audio_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct CsolSettingsResponse {
    pub voice_picks: HashMap<String, String>,
    pub special_voice_priority: bool,
}

#[derive(Debug, Serialize)]
pub struct EventSoundRouteResponse {
    pub mode: &'static str,
    pub custom_path: String,
}

#[derive(Debug, Serialize)]
pub struct EventSoundSettingsResponse {
    pub active: bool,
    pub normal: EventSoundRouteResponse,
    pub headshot: EventSoundRouteResponse,
    pub knife: EventSoundRouteResponse,
    pub assist: EventSoundRouteResponse,
}

#[derive(Debug, Serialize)]
pub struct SpectatorSettingsResponse {
    pub enabled: bool,
    pub spectated_player_enabled: bool,
    pub replay_enabled: bool,
    pub controlled_bot_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct Cs2RootResponse {
    pub found: bool,
    pub path: Option<String>,
    pub paths: Vec<String>,
    pub cfg_status: &'static str,
    pub installations: Vec<CounterStrikeInstallationResponse>,
}

#[derive(Debug, Serialize)]
pub struct CounterStrikeInstallationResponse {
    pub path: String,
    pub version: &'static str,
    pub cfg_status: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterStrikeVersion {
    Cs2,
    Legacy,
}

impl CounterStrikeVersion {
    fn from_query(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("cs2").trim().to_ascii_lowercase().as_str() {
            "cs2" => Some(Self::Cs2),
            "csgo" | "legacy" | "csgo_legacy" => Some(Self::Legacy),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Cs2 => "cs2",
            Self::Legacy => "csgo_legacy",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CounterStrikeRootQuery {
    pub version: Option<String>,
    pub path: Option<String>,
}

const GSI_CONFIG_FILE_NAME: &str = "gamestate_integration_killconfirm.cfg";
const GSI_CONFIG_TEXT: &str = "\"KillConfirmGameBar\"\r\n{\r\n \"uri\" \"http://127.0.0.1:3000/\"\r\n \"timeout\" \"0.5\"\r\n \"buffer\"  \"0.05\"\r\n \"throttle\" \"0.05\"\r\n \"heartbeat\" \"15.0\"\r\n \"auth\"\r\n {\r\n   \"token\" \"killconfirm\"\r\n }\r\n \"data\"\r\n {\r\n   \"provider\"           \"1\"\r\n   \"map\"                \"1\"\r\n   \"round\"              \"1\"\r\n   \"bomb\"               \"1\"\r\n   \"player_id\"          \"1\"\r\n   \"player_state\"       \"1\"\r\n   \"player_weapons\"     \"1\"\r\n   \"player_match_stats\" \"1\"\r\n   \"player_position\"    \"1\"\r\n   \"allplayers_id\"          \"1\"\r\n   \"allplayers_state\"       \"1\"\r\n   \"allplayers_weapons\"     \"1\"\r\n   \"allplayers_match_stats\" \"1\"\r\n }\r\n}\r\n";

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SoundPackOption {
    pub preset: &'static str,
    pub display_name: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct MoneyModeOption {
    pub mode: &'static str,
    pub display_name: &'static str,
}

const SOUND_PACK_OPTIONS: &[SoundPackOption] = &[
    SoundPackOption {
        preset: "crossfire_swat_gr",
        display_name: "swat GR",
    },
    SoundPackOption {
        preset: "crossfire_swat_bl",
        display_name: "swat BL",
    },
    SoundPackOption {
        preset: "crossfire_flying_tiger_gr",
        display_name: "tiger GR",
    },
    SoundPackOption {
        preset: "crossfire_flying_tiger_bl",
        display_name: "tiger BL",
    },
    SoundPackOption {
        preset: "crossfire_v_sex",
        display_name: "cfsex",
    },
    SoundPackOption {
        preset: "crossfire_women_gr",
        display_name: "women GR",
    },
    SoundPackOption {
        preset: "crossfire_women_bl",
        display_name: "women BL",
    },
    SoundPackOption {
        preset: "crossfire_bunny_gr",
        display_name: "Bunny GR",
    },
    SoundPackOption {
        preset: "crossfire_bunny_bl",
        display_name: "Bunny BL",
    },
    SoundPackOption {
        preset: "crossfire_heart_judge_gr",
        display_name: "Heart Judge GR",
    },
    SoundPackOption {
        preset: "crossfire_heart_judge_bl",
        display_name: "Heart Judge BL",
    },
    SoundPackOption {
        preset: "csol4",
        display_name: "CSOL 10杀",
    },
    SoundPackOption {
        preset: "bf1",
        display_name: "Battlefield 1",
    },
    SoundPackOption {
        preset: "bf5",
        display_name: "Battlefield 5",
    },
    SoundPackOption {
        preset: "bf4",
        display_name: "Battlefield 4",
    },
    SoundPackOption {
        preset: "battlefield2042",
        display_name: "Battlefield 2042",
    },
    SoundPackOption {
        preset: "pubg",
        display_name: "PUBG",
    },
    SoundPackOption {
        preset: "deltaforce",
        display_name: "Delta Force",
    },
    SoundPackOption {
        preset: "valorant_00009_prime",
        display_name: "Prime",
    },
    SoundPackOption {
        preset: "valorant_00010_glitchpop",
        display_name: "Glitchpop",
    },
    SoundPackOption {
        preset: "valorant_00011_singularity_v1",
        display_name: "Singularity V1",
    },
    SoundPackOption {
        preset: "valorant_00012_singularity_v2",
        display_name: "Singularity V2",
    },
    SoundPackOption {
        preset: "valorant_00013_singularity_v3",
        display_name: "Singularity V3",
    },
    SoundPackOption {
        preset: "valorant_00014_gaia_s_vengeance",
        display_name: "Gaia's Vengeance",
    },
    SoundPackOption {
        preset: "valorant_00015_gaia_s_vengeance_v1",
        display_name: "Gaia's Vengeance V1",
    },
    SoundPackOption {
        preset: "valorant_00016_gaia_s_vengeance_v2",
        display_name: "Gaia's Vengeance V2",
    },
    SoundPackOption {
        preset: "valorant_00017_gaia_s_vengeance_v3",
        display_name: "Gaia's Vengeance V3",
    },
    SoundPackOption {
        preset: "valorant_00018_bubblegum_deathwish",
        display_name: "Bubblegum Deathwish",
    },
    SoundPackOption {
        preset: "valorant_00019_bubblegum_deathwish_v1",
        display_name: "Bubblegum Deathwish V1",
    },
    SoundPackOption {
        preset: "valorant_00020_bubblegum_deathwish_v2",
        display_name: "Bubblegum Deathwish V2",
    },
    SoundPackOption {
        preset: "valorant_00021_bubblegum_deathwish_v3",
        display_name: "Bubblegum Deathwish V3",
    },
    SoundPackOption {
        preset: "valorant_00022_champions_2021",
        display_name: "Champions 2021",
    },
    SoundPackOption {
        preset: "valorant_00023_prelude_to_chaos_v1",
        display_name: "Prelude to Chaos V1",
    },
    SoundPackOption {
        preset: "valorant_00024_prelude_to_chaos_v2",
        display_name: "Prelude to Chaos V2",
    },
    SoundPackOption {
        preset: "valorant_00025_prelude_to_chaos_v3",
        display_name: "Prelude to Chaos V3",
    },
    SoundPackOption {
        preset: "valorant_00026_primordium",
        display_name: "Primordium",
    },
    SoundPackOption {
        preset: "valorant_00027_primordium_v1",
        display_name: "Primordium V1",
    },
    SoundPackOption {
        preset: "valorant_00028_primordium_v2",
        display_name: "Primordium V2",
    },
    SoundPackOption {
        preset: "valorant_00029_primordium_v3",
        display_name: "Primordium V3",
    },
    SoundPackOption {
        preset: "valorant_00030_radiant_crisis_001",
        display_name: "Radiant Crisis 001",
    },
    SoundPackOption {
        preset: "valorant_00031_rgx_11z_pro",
        display_name: "RGX 11z Pro",
    },
    SoundPackOption {
        preset: "valorant_00032_rgx_11z_pro_v1",
        display_name: "RGX 11z Pro V1",
    },
    SoundPackOption {
        preset: "valorant_00033_rgx_11z_pro_v2",
        display_name: "RGX 11z Pro V2",
    },
    SoundPackOption {
        preset: "valorant_00034_rgx_11z_pro_v3",
        display_name: "RGX 11z Pro V3",
    },
];

const MONEY_MODE_OPTIONS: &[MoneyModeOption] = &[
    MoneyModeOption {
        mode: "delta",
        display_name: "GSI Delta Validation (Experimental)",
    },
    MoneyModeOption {
        mode: "rules",
        display_name: "Kill Reward Rules (Recommended)",
    },
];

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "kill-confirm-gamebar",
    })
}

pub async fn gsi_status(State(app_state): State<Arc<AppState>>) -> Json<GsiStatusResponse> {
    let now = unix_time_ms();
    let last_post = zero_to_none(app_state.last_gsi_post_unix_ms.load(Ordering::Relaxed));
    Json(GsiStatusResponse {
        posts: app_state.gsi_posts.load(Ordering::Relaxed),
        parse_errors: app_state.gsi_parse_errors.load(Ordering::Relaxed),
        last_post_unix_ms: last_post,
        last_post_age_ms: last_post.map(|value| now.saturating_sub(value)),
        last_parse_error_unix_ms: zero_to_none(
            app_state
                .last_gsi_parse_error_unix_ms
                .load(Ordering::Relaxed),
        ),
    })
}

pub async fn cs2_root() -> Json<Cs2RootResponse> {
    let paths = detect_counter_strike_roots();
    let first = paths.first().cloned();
    let cfg_status = first
        .as_ref()
        .and_then(|root| {
            detect_counter_strike_version(root).map(|version| cfg_status(root, version))
        })
        .unwrap_or("not_found");
    let installations = counter_strike_installations_response(&paths);
    Json(Cs2RootResponse {
        found: !paths.is_empty(),
        path: first.map(|value| value.display().to_string()),
        paths: paths
            .into_iter()
            .map(|value| value.display().to_string())
            .collect(),
        cfg_status,
        installations,
    })
}

pub async fn install_counter_strike_cfg(
    Query(query): Query<CounterStrikeRootQuery>,
) -> Result<Json<Cs2RootResponse>, (axum::http::StatusCode, String)> {
    let roots = detect_counter_strike_roots();
    let requested_path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let requested_version = match query.version.as_deref() {
        Some(value) => Some(
            CounterStrikeVersion::from_query(Some(value)).ok_or_else(|| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "version must be 'cs2' or 'csgo_legacy'".to_string(),
                )
            })?,
        ),
        None => None,
    };
    let root = roots
        .iter()
        .find(|root| {
            if requested_path
                .is_some_and(|path| !root.display().to_string().eq_ignore_ascii_case(path))
            {
                return false;
            }
            let detected = detect_counter_strike_version(root);
            requested_version
                .map(|version| detected == Some(version))
                .unwrap_or(detected.is_some())
        })
        .cloned()
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "requested Counter-Strike installation was not found".to_string(),
            )
        })?;
    let version = detect_counter_strike_version(&root).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "detected installation layout is unsupported".to_string(),
        )
    })?;
    let cfg_folder = counter_strike_cfg_folder(&root, version);
    fs::create_dir_all(&cfg_folder).map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "failed to create cfg folder {}: {error}",
                cfg_folder.display()
            ),
        )
    })?;
    let cfg_path = cfg_folder.join(GSI_CONFIG_FILE_NAME);
    fs::write(&cfg_path, GSI_CONFIG_TEXT.as_bytes()).map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to write cfg {}: {error}", cfg_path.display()),
        )
    })?;
    service_log(&format!(
        "installed GSI cfg through service: {}",
        cfg_path.display()
    ));

    let installations = counter_strike_installations_response(&roots);
    Ok(Json(Cs2RootResponse {
        found: true,
        path: Some(root.display().to_string()),
        paths: roots
            .into_iter()
            .map(|value| value.display().to_string())
            .collect(),
        cfg_status: cfg_status(&root, version),
        installations,
    }))
}

pub async fn shutdown(State(app_state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let _ = app_state.shutdown_tx.send(());
    Json(HealthResponse {
        ok: true,
        service: "kill-confirm-gamebar",
    })
}

pub async fn audio_devices(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<AudioDevicesResponse>, (axum::http::StatusCode, String)> {
    let devices = output_device_names().map_err(internal_server_error)?;
    let selected = app_state.selected_output_device_name.read().await.clone();
    let active = app_state.current_output_device_name.read().await.clone();
    Ok(Json(AudioDevicesResponse {
        devices,
        selected,
        active,
    }))
}

pub async fn set_audio_device(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<AudioDeviceRequest>,
) -> Result<Json<AudioDevicesResponse>, (axum::http::StatusCode, String)> {
    let requested = if request.device.trim().is_empty() {
        "default".to_string()
    } else {
        request.device.trim().to_string()
    };
    let (output_stream, active) =
        get_output_stream_with_name(&requested).map_err(internal_server_error)?;
    {
        let mut stream = app_state.stream_handle.write().await;
        *stream = output_stream;
    }
    {
        let mut selected = app_state.selected_output_device_name.write().await;
        *selected = requested.clone();
    }
    {
        let mut current = app_state.current_output_device_name.write().await;
        *current = active.clone();
    }
    service_log(&format!(
        "audio output device selected: {requested} -> {active}"
    ));
    let devices = output_device_names().map_err(internal_server_error)?;
    Ok(Json(AudioDevicesResponse {
        devices,
        selected: requested,
        active,
    }))
}

fn internal_server_error(error: anyhow::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        error.to_string(),
    )
}

pub async fn audio_reload(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, (axum::http::StatusCode, String)> {
    service_log("audio reload requested");
    let requested_device = app_state.selected_output_device_name.read().await.clone();
    let (output_stream, device_name) =
        get_output_stream_with_name(&requested_device).map_err(internal_server_error)?;

    {
        let mut stream_handle = app_state.stream_handle.write().await;
        *stream_handle = output_stream;
    }
    {
        let mut current_device = app_state.current_output_device_name.write().await;
        *current_device = device_name.clone();
    }

    service_log(&format!("audio output stream reloaded -> {device_name}"));
    Ok(Json(HealthResponse {
        ok: true,
        service: "kill-confirm-gamebar",
    }))
}

pub async fn audio_volume(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<VolumeRequest>,
) -> Json<HealthResponse> {
    let percent = request.percent.min(200);
    app_state.volume_percent.store(percent, Ordering::Relaxed);
    service_log(&format!("audio volume set to {percent}%"));

    Json(HealthResponse {
        ok: true,
        service: "kill-confirm-gamebar",
    })
}

pub async fn money_mode(State(app_state): State<Arc<AppState>>) -> Json<MoneyModeResponse> {
    Json(money_mode_response(MoneyRewardMode::from_u8(
        app_state.money_reward_mode.load(Ordering::Relaxed),
    )))
}

pub async fn set_money_mode(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<MoneyModeRequest>,
) -> Result<Json<MoneyModeResponse>, (axum::http::StatusCode, String)> {
    let mode = MoneyRewardMode::from_str(&request.mode).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "unsupported money reward mode".to_string(),
        )
    })?;

    app_state
        .money_reward_mode
        .store(mode.as_u8(), Ordering::Relaxed);
    service_log(&format!("money reward mode set to '{}'", mode.as_str()));

    Ok(Json(money_mode_response(mode)))
}

pub async fn crossfire_settings(
    State(app_state): State<Arc<AppState>>,
) -> Json<CrossfireSettingsResponse> {
    Json(crossfire_settings_response(&app_state))
}

pub async fn set_crossfire_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<CrossfireSettingsRequest>,
) -> Result<Json<CrossfireSettingsResponse>, (axum::http::StatusCode, String)> {
    let (streak_mode, streak_window_ms) =
        parse_streak_setting(&request.streak_mode).ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported CrossFire streak mode".to_string(),
            )
        })?;

    let previous_mode = app_state
        .crossfire_streak_mode
        .swap(streak_mode.as_u8(), Ordering::Relaxed);
    let previous_window_ms = app_state
        .crossfire_streak_window_ms
        .swap(streak_window_ms, Ordering::Relaxed);
    let previous_active = app_state
        .crossfire_mode_active
        .swap(request.active, Ordering::Relaxed);
    let previous_shared_active = if request.active {
        app_state
            .shared_streak_mode_active
            .swap(false, Ordering::Relaxed)
    } else {
        app_state.shared_streak_mode_active.load(Ordering::Relaxed)
    };
    app_state
        .crossfire_first_kill_special_audio
        .store(request.first_kill_special_audio, Ordering::Relaxed);
    app_state
        .crossfire_last_kill_special_audio
        .store(request.last_kill_special_audio, Ordering::Relaxed);
    app_state
        .crossfire_headshot_special_audio_priority
        .store(request.headshot_special_audio_priority, Ordering::Relaxed);
    app_state
        .crossfire_knife_special_audio_priority
        .store(request.knife_special_audio_priority, Ordering::Relaxed);
    if request.active {
        app_state
            .assist_audio_enabled
            .store(request.assist_audio_enabled, Ordering::Relaxed);
        app_state
            .assist_audio_setting_active
            .store(true, Ordering::Relaxed);
    } else if previous_active {
        app_state
            .assist_audio_enabled
            .store(false, Ordering::Relaxed);
        app_state
            .assist_audio_setting_active
            .store(false, Ordering::Relaxed);
    }

    if previous_mode != streak_mode.as_u8()
        || previous_window_ms != streak_window_ms
        || previous_active != request.active
        || (request.active && previous_shared_active)
    {
        let mut mutable = app_state.mutable.write().await;
        mutable.crossfire_streak_kills = 0;
        mutable.last_crossfire_kill_at = None;
    }

    service_log(&format!(
        "CrossFire settings: active={}, streak={}, first_special={}, last_special={}, headshot_priority={}, knife_priority={}, assist_audio={}",
        request.active,
        format_streak_setting(streak_mode, streak_window_ms),
        request.first_kill_special_audio,
        request.last_kill_special_audio,
        request.headshot_special_audio_priority,
        request.knife_special_audio_priority,
        request.assist_audio_enabled
    ));

    Ok(Json(crossfire_settings_response(&app_state)))
}

pub async fn streak_settings(
    State(app_state): State<Arc<AppState>>,
) -> Json<StreakSettingsResponse> {
    Json(streak_settings_response(&app_state))
}

pub async fn set_streak_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<StreakSettingsRequest>,
) -> Result<Json<StreakSettingsResponse>, (axum::http::StatusCode, String)> {
    let (streak_mode, streak_window_ms) =
        parse_streak_setting(&request.streak_mode).ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported kill streak mode".to_string(),
            )
        })?;

    let previous_mode = app_state
        .shared_streak_mode
        .swap(streak_mode.as_u8(), Ordering::Relaxed);
    let previous_window_ms = app_state
        .shared_streak_window_ms
        .swap(streak_window_ms, Ordering::Relaxed);
    let previous_active = app_state
        .shared_streak_mode_active
        .swap(request.active, Ordering::Relaxed);
    let previous_crossfire_active = if request.active {
        app_state
            .crossfire_mode_active
            .swap(false, Ordering::Relaxed)
    } else {
        app_state.crossfire_mode_active.load(Ordering::Relaxed)
    };

    if request.active {
        app_state
            .assist_audio_enabled
            .store(request.assist_audio_enabled, Ordering::Relaxed);
        app_state
            .assist_audio_setting_active
            .store(request.assist_audio_setting_active, Ordering::Relaxed);
    } else if previous_active {
        app_state
            .assist_audio_enabled
            .store(false, Ordering::Relaxed);
        app_state
            .assist_audio_setting_active
            .store(false, Ordering::Relaxed);
    }

    if previous_mode != streak_mode.as_u8()
        || previous_window_ms != streak_window_ms
        || previous_active != request.active
        || (request.active && previous_crossfire_active)
    {
        let mut mutable = app_state.mutable.write().await;
        mutable.crossfire_streak_kills = 0;
        mutable.last_crossfire_kill_at = None;
    }

    service_log(&format!(
        "shared streak settings: active={}, streak={}, assist_audio={}, assist_audio_controlled={}",
        request.active,
        format_streak_setting(streak_mode, streak_window_ms),
        request.assist_audio_enabled,
        request.assist_audio_setting_active
    ));

    Ok(Json(streak_settings_response(&app_state)))
}

pub async fn csol_settings(State(app_state): State<Arc<AppState>>) -> Json<CsolSettingsResponse> {
    Json(csol_settings_response(&app_state).await)
}

pub async fn set_csol_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<CsolSettingsRequest>,
) -> Result<Json<CsolSettingsResponse>, (axum::http::StatusCode, String)> {
    service_log(&format!(
        "CSOL settings: voice_picks={:?}, special_voice_priority={}",
        request.voice_picks, request.special_voice_priority
    ));
    {
        let mut picks = app_state.csol_voice_picks.write().await;
        *picks = request.voice_picks;
    }
    app_state
        .csol_special_voice_priority
        .store(request.special_voice_priority, Ordering::Relaxed);

    Ok(Json(csol_settings_response(&app_state).await))
}

pub async fn event_sound_settings(
    State(app_state): State<Arc<AppState>>,
) -> Json<EventSoundSettingsResponse> {
    let settings = app_state.event_sound_settings.read().await;
    Json(event_sound_settings_response(&settings))
}

pub async fn set_event_sound_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<EventSoundSettingsRequest>,
) -> Result<Json<EventSoundSettingsResponse>, (axum::http::StatusCode, String)> {
    let settings = EventSoundSettings {
        active: request.active,
        normal: parse_event_sound_route(request.normal)?,
        headshot: parse_event_sound_route(request.headshot)?,
        knife: parse_event_sound_route(request.knife)?,
        assist: parse_event_sound_route(request.assist)?,
    };
    let response = event_sound_settings_response(&settings);
    service_log(&format!(
        "event sound settings: active={}, normal={}, headshot={}, knife={}, assist={}",
        settings.active,
        settings.normal.mode.as_str(),
        settings.headshot.mode.as_str(),
        settings.knife.mode.as_str(),
        settings.assist.mode.as_str()
    ));
    *app_state.event_sound_settings.write().await = settings;
    Ok(Json(response))
}

fn parse_event_sound_route(
    request: EventSoundRouteRequest,
) -> Result<EventSoundRoute, (axum::http::StatusCode, String)> {
    let mode = EventSoundMode::from_str(&request.mode).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("unsupported event sound mode: {}", request.mode),
        )
    })?;
    let custom_path = request.custom_path.trim();
    Ok(EventSoundRoute {
        mode,
        custom_path: (!custom_path.is_empty()).then(|| custom_path.to_string()),
    })
}

pub async fn spectator_settings(
    State(app_state): State<Arc<AppState>>,
) -> Json<SpectatorSettingsResponse> {
    Json(spectator_settings_response(&app_state))
}

pub async fn set_spectator_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<SpectatorSettingsRequest>,
) -> Json<SpectatorSettingsResponse> {
    let legacy = request.enabled;
    let spectated_player_enabled =
        request
            .spectated_player_enabled
            .or(legacy)
            .unwrap_or_else(|| {
                app_state
                    .spectated_player_effects_enabled
                    .load(Ordering::Relaxed)
            });
    let replay_enabled = request
        .replay_enabled
        .or(legacy)
        .unwrap_or_else(|| app_state.replay_effects_enabled.load(Ordering::Relaxed));
    let controlled_bot_enabled = request
        .controlled_bot_enabled
        .or(legacy)
        .unwrap_or_else(|| {
            app_state
                .controlled_bot_effects_enabled
                .load(Ordering::Relaxed)
        });
    app_state
        .spectated_player_effects_enabled
        .store(spectated_player_enabled, Ordering::Relaxed);
    app_state
        .replay_effects_enabled
        .store(replay_enabled, Ordering::Relaxed);
    app_state
        .controlled_bot_effects_enabled
        .store(controlled_bot_enabled, Ordering::Relaxed);
    service_log(&format!(
        "observed effects: spectated={}, replay={}, controlled_bot={}",
        spectated_player_enabled, replay_enabled, controlled_bot_enabled
    ));
    Json(spectator_settings_response(&app_state))
}

pub async fn soundpack(State(app_state): State<Arc<AppState>>) -> Json<SoundPackResponse> {
    let preset = app_state.preset.read().await;
    Json(soundpack_response(
        &preset.preset_name,
        &preset.display_name,
    ))
}

pub async fn set_soundpack(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<SoundPackRequest>,
) -> Result<Json<SoundPackResponse>, (axum::http::StatusCode, String)> {
    let (preset_name, preset, display_name) = if let Some(custom_path) =
        request.custom_path.as_deref()
    {
        let display_name = request
            .display_name
            .clone()
            .unwrap_or_else(|| request.preset.clone());
        let preset =
            Preset::load_custom(&request.preset, &display_name, custom_path).map_err(|error| {
                service_log(&format!(
                    "failed to load custom sound pack '{}': {error:?}",
                    request.preset
                ));
                (axum::http::StatusCode::BAD_REQUEST, format!("{error:?}"))
            })?;
        (request.preset.as_str(), preset, display_name)
    } else {
        let preset_name = resolve_soundpack_alias(&request.preset).ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                "unsupported sound pack".to_string(),
            )
        })?;
        let preset = Preset::load(preset_name).map_err(|error| {
            service_log(&format!(
                "failed to load sound pack '{preset_name}': {error:?}"
            ));
            (axum::http::StatusCode::BAD_REQUEST, format!("{error:?}"))
        })?;
        (
            preset_name,
            preset,
            soundpack_display_name(preset_name).to_string(),
        )
    };

    {
        let mut current = app_state.preset.write().await;
        *current = preset;
    }

    service_log(&format!(
        "sound pack set to '{preset_name}' ({display_name})"
    ));
    {
        let cache_state = app_state.clone();
        tokio::spawn(async move {
            warm_audio_cache(cache_state).await;
        });
    }

    Ok(Json(soundpack_response(preset_name, &display_name)))
}

pub async fn events_ws(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rx = app_state.event_tx.subscribe();
    let shutdown_rx = app_state.shutdown_tx.subscribe();
    ws.on_upgrade(move |socket| send_events(socket, rx, shutdown_rx))
}

pub async fn test_event(
    Path(kill_count): Path<u16>,
    Query(query): Query<TestEventQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Json<KillEvent> {
    service_log(&format!(
        "test event requested: kills={kill_count}, audio={}, headshot={}, knife={}, assist={}, first={}, last={}, main={}",
        query.audio.unwrap_or(false),
        query.headshot.unwrap_or(false),
        query.knife.unwrap_or(false),
        query.assist.unwrap_or(false),
        query.first.unwrap_or(false),
        query.last.unwrap_or(false),
        query.main.unwrap_or(true)
    ));

    let event = KillEvent {
        kill_count,
        is_headshot: query.headshot.unwrap_or(false),
        is_knife_kill: query.knife.unwrap_or(false),
        is_first_kill: query.first.unwrap_or(false),
        is_last_kill: query.last.unwrap_or(false),
        is_assist: query.assist.unwrap_or(false),
        play_main_animation: query.main.unwrap_or(true),
        animation_key: query.animation.filter(|value| !value.trim().is_empty()),
        event_kind: query.event_kind.filter(|value| !value.trim().is_empty()),
        weapon_badge_key: query.weapon_badge.filter(|value| !value.trim().is_empty()),
        weapon_name: query
            .weapon_name
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some("AK-47".to_string())),
        money_reward: if query.assist.unwrap_or(false) {
            money_rules::assist_reward_for(money_rules::EconomyVersion::Cs2)
        } else {
            query.money_reward.unwrap_or_else(|| {
                if query.knife.unwrap_or(false) {
                    1500
                } else {
                    300
                }
            })
        },
        round_number: query.round_number.unwrap_or(0),
        money_epoch: query
            .money_epoch
            .unwrap_or_else(|| query.round_number.unwrap_or(0) as u32),
        player_name: query
            .player_name
            .unwrap_or_else(|| "\u{73a9}\u{5bb6}".to_string()),
        target_name: query
            .target_name
            .or_else(|| Some("\u{6050}\u{6016}\u{5206}\u{5b50}".to_string())),
        steamid: query.steamid.unwrap_or_else(|| "test".to_string()),
    };

    let _ = app_state.event_tx.send(event.clone());

    if query.audio.unwrap_or(false) {
        let app_state_clone = app_state.clone();
        let event_clone = event.clone();
        tokio::spawn(async move {
            let result = play_audio(
                app_state_clone,
                event_clone.kill_count,
                event_clone.is_headshot,
                event_clone.is_first_kill,
                event_clone.is_knife_kill,
                event_clone.is_last_kill,
                event_clone.is_assist,
                event_clone.money_reward,
                event_clone.event_kind.clone(),
                event_clone.play_main_animation,
            )
            .await;

            if let Err(error) = result {
                error!("failed to play test audio: {error}");
                service_log(&format!("failed to play test audio: {error}"));
            }
        });
    }

    Json(event)
}

fn resolve_soundpack_alias(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    if let Some(option) = SOUND_PACK_OPTIONS
        .iter()
        .find(|option| option.preset.eq_ignore_ascii_case(&normalized))
    {
        return Some(option.preset);
    }

    match normalized.as_str() {
        "cf" | "crossfire" | "swatgr" | "swat_gr" | "crossfire_swat_gr" => {
            Some("crossfire_swat_gr")
        }
        "swatbl" | "swat_bl" | "crossfire_swat_bl" => Some("crossfire_swat_bl"),
        "cfftgr"
        | "ftgr"
        | "tiger_gr"
        | "flying_tiger_gr"
        | "crossfire_flying_tiger_gr"
        | "cffhd"
        | "cf_fhd"
        | "crossfire_fhd"
        | "crossfire_v_fhd" => Some("crossfire_flying_tiger_gr"),
        "cfsex" | "cf_sex" | "crossfire_sex" | "crossfire_v_sex" => Some("crossfire_v_sex"),
        "cfftbl" | "ftbl" | "tiger_bl" | "flying_tiger_bl" | "crossfire_flying_tiger_bl" => {
            Some("crossfire_flying_tiger_bl")
        }
        "cwgr" | "women_gr" | "crossfire_women_gr" | "kkgr" | "knifegr" | "knifekill_gr" => {
            Some("crossfire_women_gr")
        }
        "cwbl" | "women_bl" | "crossfire_women_bl" | "kkbl" | "knifebl" | "knifekill_bl" => {
            Some("crossfire_women_bl")
        }
        "bunnygr" | "bunny_gr" | "crossfire_bunny_gr" => Some("crossfire_bunny_gr"),
        "bunnybl" | "bunny_bl" | "crossfire_bunny_bl" => Some("crossfire_bunny_bl"),
        "heartjudgegr" | "heart_judge_gr" | "judge_gr" | "crossfire_heart_judge_gr" => {
            Some("crossfire_heart_judge_gr")
        }
        "heartjudgebl" | "heart_judge_bl" | "judge_bl" | "crossfire_heart_judge_bl" => {
            Some("crossfire_heart_judge_bl")
        }
        "bf1" | "battlefield1" | "battlefield_1" => Some("bf1"),
        "bf5" | "battlefield5" | "battlefield_5" => Some("bf5"),
        "bf4" | "battlefield4" | "battlefield_4" => Some("bf4"),
        "bf2042" | "battlefield2042" | "battlefield_2042" | "2042" => Some("battlefield2042"),
        "pubg" | "pubg_elimination" | "pubg_subtitle" => Some("pubg"),
        "delta" | "df" | "deltaforce" | "delta_force" => Some("deltaforce"),
        "csol4" | "csol" => Some("csol4"),
        _ => None,
    }
}

fn soundpack_response(preset_name: &str, display_name: &str) -> SoundPackResponse {
    SoundPackResponse {
        preset: preset_name.to_string(),
        display_name: display_name.to_string(),
        available: SOUND_PACK_OPTIONS.to_vec(),
    }
}

fn money_mode_response(mode: MoneyRewardMode) -> MoneyModeResponse {
    MoneyModeResponse {
        mode: mode.as_str(),
        available: MONEY_MODE_OPTIONS.to_vec(),
    }
}

fn crossfire_settings_response(app_state: &AppState) -> CrossfireSettingsResponse {
    let mode =
        CrossfireStreakMode::from_u8(app_state.crossfire_streak_mode.load(Ordering::Relaxed));
    CrossfireSettingsResponse {
        active: app_state.crossfire_mode_active.load(Ordering::Relaxed),
        streak_mode: format_streak_setting(
            mode,
            app_state.crossfire_streak_window_ms.load(Ordering::Relaxed),
        ),
        first_kill_special_audio: app_state
            .crossfire_first_kill_special_audio
            .load(Ordering::Relaxed),
        last_kill_special_audio: app_state
            .crossfire_last_kill_special_audio
            .load(Ordering::Relaxed),
        headshot_special_audio_priority: app_state
            .crossfire_headshot_special_audio_priority
            .load(Ordering::Relaxed),
        knife_special_audio_priority: app_state
            .crossfire_knife_special_audio_priority
            .load(Ordering::Relaxed),
        assist_audio_enabled: app_state.assist_audio_enabled.load(Ordering::Relaxed),
    }
}

fn streak_settings_response(app_state: &AppState) -> StreakSettingsResponse {
    let mode = CrossfireStreakMode::from_u8(app_state.shared_streak_mode.load(Ordering::Relaxed));
    StreakSettingsResponse {
        active: app_state.shared_streak_mode_active.load(Ordering::Relaxed),
        streak_mode: format_streak_setting(
            mode,
            app_state.shared_streak_window_ms.load(Ordering::Relaxed),
        ),
        assist_audio_enabled: app_state.assist_audio_enabled.load(Ordering::Relaxed),
    }
}

async fn csol_settings_response(app_state: &AppState) -> CsolSettingsResponse {
    CsolSettingsResponse {
        voice_picks: app_state.csol_voice_picks.read().await.clone(),
        special_voice_priority: app_state
            .csol_special_voice_priority
            .load(Ordering::Relaxed),
    }
}

fn event_sound_settings_response(settings: &EventSoundSettings) -> EventSoundSettingsResponse {
    EventSoundSettingsResponse {
        active: settings.active,
        normal: event_sound_route_response(&settings.normal),
        headshot: event_sound_route_response(&settings.headshot),
        knife: event_sound_route_response(&settings.knife),
        assist: event_sound_route_response(&settings.assist),
    }
}

fn event_sound_route_response(route: &EventSoundRoute) -> EventSoundRouteResponse {
    EventSoundRouteResponse {
        mode: route.mode.as_str(),
        custom_path: route.custom_path.clone().unwrap_or_default(),
    }
}

fn spectator_settings_response(app_state: &AppState) -> SpectatorSettingsResponse {
    let spectated_player_enabled = app_state
        .spectated_player_effects_enabled
        .load(Ordering::Relaxed);
    let replay_enabled = app_state.replay_effects_enabled.load(Ordering::Relaxed);
    let controlled_bot_enabled = app_state
        .controlled_bot_effects_enabled
        .load(Ordering::Relaxed);
    SpectatorSettingsResponse {
        enabled: spectated_player_enabled && replay_enabled && controlled_bot_enabled,
        spectated_player_enabled,
        replay_enabled,
        controlled_bot_enabled,
    }
}

fn soundpack_display_name(preset_name: &str) -> &'static str {
    SOUND_PACK_OPTIONS
        .iter()
        .find(|option| option.preset == preset_name)
        .map(|option| option.display_name)
        .unwrap_or("custom")
}

pub(crate) fn detect_counter_strike_roots() -> Vec<PathBuf> {
    let mut installations = Vec::new();
    for library_root in steam_library_roots() {
        let common = library_root.join("steamapps").join("common");
        for installation in find_counter_strike_installations_in_common(&common) {
            push_unique_path(&mut installations, installation);
        }
    }

    installations
}

fn detect_counter_strike_version(root: &std::path::Path) -> Option<CounterStrikeVersion> {
    if root.join("game").join("csgo").join("cfg").is_dir() {
        Some(CounterStrikeVersion::Cs2)
    } else if root.join("csgo.exe").is_file() && root.join("csgo").join("cfg").is_dir() {
        Some(CounterStrikeVersion::Legacy)
    } else {
        None
    }
}

fn counter_strike_cfg_folder(root: &std::path::Path, version: CounterStrikeVersion) -> PathBuf {
    match version {
        CounterStrikeVersion::Cs2 => root.join("game").join("csgo").join("cfg"),
        CounterStrikeVersion::Legacy => root.join("csgo").join("cfg"),
    }
}

fn cfg_status(root: &std::path::Path, version: CounterStrikeVersion) -> &'static str {
    let cfg_path = counter_strike_cfg_folder(root, version).join(GSI_CONFIG_FILE_NAME);
    let Ok(actual) = fs::read_to_string(cfg_path) else {
        return "missing";
    };
    let normalize = |value: &str| {
        value
            .trim_start_matches('\u{feff}')
            .replace("\r\n", "\n")
            .replace('\r', "\n")
    };
    if normalize(&actual) == normalize(GSI_CONFIG_TEXT) {
        "ready"
    } else {
        "outdated"
    }
}

fn counter_strike_installations_response(
    roots: &[PathBuf],
) -> Vec<CounterStrikeInstallationResponse> {
    roots
        .iter()
        .filter_map(|root| {
            let version = detect_counter_strike_version(root)?;
            Some(CounterStrikeInstallationResponse {
                path: root.display().to_string(),
                version: version.as_str(),
                cfg_status: cfg_status(root, version),
            })
        })
        .collect()
}

fn find_counter_strike_installations_in_common(common: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(common) else {
        return Vec::new();
    };
    let mut installations = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|root| {
            let has_cs2_layout = root.join("game").join("csgo").join("cfg").is_dir();
            let has_legacy_layout =
                root.join("csgo.exe").is_file() && root.join("csgo").join("cfg").is_dir();
            has_cs2_layout || has_legacy_layout
        })
        .collect::<Vec<_>>();
    installations.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    installations
}

fn steam_library_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for root in steam_roots() {
        push_unique_path(&mut roots, root.clone());

        let library_folders = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(library_folders) {
            for path in parse_steam_library_paths(&text) {
                push_unique_path(&mut roots, path);
            }
        }
    }

    roots
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for value_name in ["SteamPath", "InstallPath"] {
        for (hive, key, wow6432) in [
            (HKEY_CURRENT_USER, r"Software\Valve\Steam", false),
            (HKEY_LOCAL_MACHINE, r"Software\Valve\Steam", true),
            (HKEY_LOCAL_MACHINE, r"Software\Valve\Steam", false),
        ] {
            if let Some(path) = query_registry_string(hive, key, value_name, wow6432) {
                push_unique_path(&mut roots, PathBuf::from(path.replace('/', "\\")));
            }
        }
    }

    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        push_unique_path(&mut roots, PathBuf::from(program_files_x86).join("Steam"));
    }

    roots
}

fn query_registry_string(hive: HKEY, key: &str, value_name: &str, wow6432: bool) -> Option<String> {
    let key = key.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let value_name = value_name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let flags = RRF_RT_REG_SZ | if wow6432 { RRF_SUBKEY_WOW6432KEY } else { 0 };
    let mut value_type = 0;
    let mut byte_count = 0;
    let status = unsafe {
        RegGetValueW(
            hive,
            key.as_ptr(),
            value_name.as_ptr(),
            flags,
            &mut value_type,
            std::ptr::null_mut(),
            &mut byte_count,
        )
    };
    if status != ERROR_SUCCESS || value_type != REG_SZ || byte_count < 2 {
        return None;
    }

    let mut buffer = vec![0u16; byte_count as usize / 2];
    let status = unsafe {
        RegGetValueW(
            hive,
            key.as_ptr(),
            value_name.as_ptr(),
            flags,
            &mut value_type,
            buffer.as_mut_ptr().cast(),
            &mut byte_count,
        )
    };
    if status != ERROR_SUCCESS || value_type != REG_SZ {
        return None;
    }
    if let Some(terminator) = buffer.iter().position(|value| *value == 0) {
        buffer.truncate(terminator);
    }
    let value = String::from_utf16(&buffer).ok()?;
    (!value.trim().is_empty()).then_some(value)
}

fn parse_steam_library_paths(text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if !(trimmed.starts_with("\"path\"") || starts_with_quoted_number(trimmed)) {
            continue;
        }

        let quoted: Vec<&str> = trimmed.split('"').collect();
        if quoted.len() >= 4 {
            let value = quoted[3].replace("\\\\", "\\");
            if !value.is_empty() {
                paths.push(PathBuf::from(value));
            }
        }
    }

    paths
}

fn starts_with_quoted_number(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('"') else {
        return false;
    };
    let Some((number, _)) = rest.split_once('"') else {
        return false;
    };
    !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit())
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !path.exists() {
        return;
    }

    let normalized = path.to_string_lossy();
    if !paths
        .iter()
        .any(|existing| existing.to_string_lossy().eq_ignore_ascii_case(&normalized))
    {
        paths.push(path);
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

fn zero_to_none(value: u64) -> Option<u64> {
    if value == 0 { None } else { Some(value) }
}

async fn send_events(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<KillEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    debug!("kill event websocket connected");

    loop {
        let event = tokio::select! {
            event = rx.recv() => match event {
                Ok(event) => event,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!("kill event websocket skipped {skipped} stale events");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            _ = shutdown_rx.recv() => break,
        };

        let payload = match serde_json::to_string(&event) {
            Ok(payload) => payload,
            Err(error) => {
                warn!("failed to serialize kill event: {error}");
                continue;
            }
        };

        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }

    debug!("kill event websocket disconnected");
}

#[cfg(test)]
mod tests {
    use super::find_counter_strike_installations_in_common;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn installation_scan_finds_cs2_and_a_custom_named_legacy_copy() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let common = std::env::temp_dir().join(format!("killconfirm-cs-layouts-{suffix}"));
        let cs2 = common.join("Counter-Strike Global Offensive");
        let legacy = common.join("csgo legacy");
        fs::create_dir_all(cs2.join("game").join("csgo").join("cfg")).unwrap();
        fs::create_dir_all(legacy.join("csgo").join("cfg")).unwrap();
        fs::write(legacy.join("csgo.exe"), []).unwrap();

        let installations = find_counter_strike_installations_in_common(&common);

        assert_eq!(installations, vec![cs2, legacy]);
        fs::remove_dir_all(common).unwrap();
    }
}
