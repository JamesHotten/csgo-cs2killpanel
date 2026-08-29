use gsi_cs2::map::Mode;
use gsi_cs2::round::BombState;
use gsi_cs2::team::TeamClass;
use gsi_cs2::weapon::WeaponName;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EconomyVersion {
    Cs2,
    CsgoLegacy,
}

impl EconomyVersion {
    pub fn from_legacy(is_legacy: bool) -> Self {
        if is_legacy { Self::CsgoLegacy } else { Self::Cs2 }
    }
}

pub fn uses_standard_cash_economy(mode: &Mode) -> bool {
    matches!(mode, Mode::Casual | Mode::Competitive | Mode::Wingman)
}

pub fn assist_reward_for(_economy: EconomyVersion) -> u16 { 0 }

pub fn weapon_kill_reward(weapon_name: &WeaponName, mode: &Mode) -> u16 {
    weapon_kill_reward_for(weapon_name, mode, EconomyVersion::Cs2)
}

pub fn weapon_kill_reward_for(
    weapon_name: &WeaponName,
    mode: &Mode,
    economy: EconomyVersion,
) -> u16 {
    let reward = match weapon_name {
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
        | WeaponName::KnifeUrsus => 1500,
        WeaponName::Nova | WeaponName::MAG7 | WeaponName::SawedOff => 900,
        WeaponName::XM1014 if matches!(economy, EconomyVersion::CsgoLegacy) => 900,
        WeaponName::XM1014 | WeaponName::MAC10
        | WeaponName::MP5SD
        | WeaponName::MP7
        | WeaponName::MP9
        | WeaponName::Bizon
        | WeaponName::UMP45 => 600,
        WeaponName::Zeus27 if matches!(economy, EconomyVersion::CsgoLegacy) => 0,
        WeaponName::CZ75A if matches!(economy, EconomyVersion::CsgoLegacy) => 100,
        WeaponName::AWP | WeaponName::Zeus27 => 100,
        _ => 300,
    };

    match mode {
        Mode::Casual => reward / 2,
        Mode::Competitive | Mode::Wingman => reward,
        _ => 0,
    }
}

pub fn default_kill_reward_for(mode: &Mode, _economy: EconomyVersion) -> u16 {
    match mode {
        Mode::Casual => 150,
        Mode::Competitive | Mode::Wingman => 300,
        _ => 0,
    }
}

pub fn bomb_objective_reward(mode: &Mode) -> u16 {
    bomb_objective_reward_for(mode, EconomyVersion::Cs2)
}

pub fn bomb_objective_reward_for(mode: &Mode, _economy: EconomyVersion) -> u16 {
    match mode {
        Mode::Casual => 200,
        Mode::Competitive | Mode::Wingman => 300,
        _ => 0,
    }
}

pub fn short_handed_bonus_for(mode: &Mode, _economy: EconomyVersion) -> u16 {
    match mode {
        Mode::Competitive | Mode::Wingman => 1000,
        _ => 0,
    }
}

pub fn ct_elimination_team_reward_for(
    eliminated_terrorists: u16,
    mode: &Mode,
    economy: EconomyVersion,
) -> u16 {
    if !matches!(economy, EconomyVersion::Cs2) { return 0; }
    let maximum = match mode { Mode::Competitive => 5, Mode::Wingman => 2, _ => 0 };
    eliminated_terrorists.min(maximum) * 50
}

pub fn loss_bonus(
    consecutive_round_losses: u8,
    mode: &Mode,
    player_team: &TeamClass,
    bomb: Option<&BombState>,
) -> u16 {
    loss_bonus_for(
        consecutive_round_losses, mode, player_team, bomb, None,
        EconomyVersion::Cs2, false,
    )
}

pub fn loss_bonus_for(
    consecutive_round_losses: u8,
    mode: &Mode,
    player_team: &TeamClass,
    bomb: Option<&BombState>,
    round_outcome: Option<&str>,
    economy: EconomyVersion,
    player_survived: bool,
) -> u16 {
    if matches!(player_team, TeamClass::T)
        && round_outcome == Some("ct_win_time")
        && player_survived
    { return 0; }

    let base_reward = match mode {
        Mode::Casual => 2400,
        // Competitive starts each half at loss level 1 in both games.
        // GSI reports that level directly: 0 is $1400 and 1 is the
        // pistol-round $1900 award.
        Mode::Competitive => match consecutive_round_losses {
            0 => 1400,
            1 => 1900,
            2 => 2400,
            3 => 2900,
            _ => 3400,
        },
        Mode::Wingman => match consecutive_round_losses.max(1) {
            1 => 2000,
            2 => 2300,
            3 => 2600,
            4 => 2900,
            _ => 3200,
        },
        _ => 0,
    };

    let planted_bomb_reward = if base_reward > 0
        && matches!(player_team, TeamClass::T)
        && (matches!(bomb, Some(BombState::Defused)) || round_outcome == Some("ct_win_defuse"))
        {
            match (mode, economy) {
                (Mode::Casual, _) => 200,
                (_, EconomyVersion::CsgoLegacy) => 800,
                _ => 600,
            }
        } else {
            0
        };

    base_reward + planted_bomb_reward
}

pub fn hostage_objective_kind(
    reward: u16,
    mode: &Mode,
    is_rescue_round: bool,
) -> Option<&'static str> {
    hostage_objective_kind_for(reward, mode, is_rescue_round, EconomyVersion::Cs2)
}

pub fn hostage_objective_kind_for(
    reward: u16,
    mode: &Mode,
    is_rescue_round: bool,
    _economy: EconomyVersion,
) -> Option<&'static str> {
    if !uses_standard_cash_economy(mode) { return None; }
    let is_supported_reward = if is_rescue_round {
        match mode {
            Mode::Casual => matches!(reward, 1000),
            _ => matches!(reward, 600 | 1000 | 1600),
        }
    } else {
        match mode {
            Mode::Casual => matches!(reward, 300 | 500 | 800),
            _ => matches!(reward, 300 | 600 | 900),
        }
    };

    is_supported_reward.then_some(if is_rescue_round {
        "hostage_rescue"
    } else {
        "hostage_interact"
    })
}

pub fn round_win_bonus(
    win_team: &TeamClass,
    bomb: Option<&BombState>,
    mode: &Mode,
    map_name: &str,
    round_outcome: Option<&str>,
) -> u16 {
    round_win_bonus_for(
        win_team, bomb, mode, map_name, round_outcome, EconomyVersion::Cs2,
    )
}

pub fn round_win_bonus_for(
    win_team: &TeamClass,
    bomb: Option<&BombState>,
    mode: &Mode,
    map_name: &str,
    round_outcome: Option<&str>,
    _economy: EconomyVersion,
) -> u16 {
    if !uses_standard_cash_economy(mode) { return 0; }
    if matches!(mode, Mode::Casual) {
        if is_hostage_map(map_name) {
            return match round_outcome {
                Some("ct_win_rescue") => 3000,
                Some("ct_win_elimination") => 2300,
                Some("t_win_elimination") | Some("t_win_time") => 2000,
                _ if matches!(win_team, TeamClass::CT) => 2300,
                _ => 2000,
            };
        }

        return 2700;
    }

    if matches!(mode, Mode::Wingman) {
        if is_hostage_map(map_name) {
            return match round_outcome {
                Some("ct_win_rescue") => 2900,
                Some("ct_win_time") | Some("t_win_time") => 2750,
                Some("ct_win_elimination") | Some("t_win_elimination") => 2500,
                _ => 2500,
            };
        }
        return match (round_outcome, win_team, bomb) {
            (Some("t_win_bomb" | "ct_win_defuse"), _, _) => 3000,
            (_, TeamClass::T, Some(BombState::Exploded))
            | (_, TeamClass::CT, Some(BombState::Defused)) => 3000,
            _ => 2750,
        };
    }

    if is_hostage_map(map_name) {
        return match round_outcome {
            Some("ct_win_rescue") => 2900,
            Some("ct_win_time") | Some("t_win_time") => 3250,
            Some("ct_win_elimination") | Some("t_win_elimination") => 3000,
            _ => 3000,
        };
    }

    match (round_outcome, win_team, bomb) {
        (Some("t_win_bomb" | "ct_win_defuse"), _, _) => 3500,
        (_, TeamClass::T, Some(BombState::Exploded)) | (_, TeamClass::CT, Some(BombState::Defused)) => {
            3500
        }
        _ => 3250,
    }
}

pub fn is_hostage_map(map_name: &str) -> bool {
    map_name
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(|name| name.to_ascii_lowercase().starts_with("cs_"))
}

#[cfg(test)]
mod tests {
    use super::{
        EconomyVersion, hostage_objective_kind, loss_bonus, loss_bonus_for, round_win_bonus,
    };
    use gsi_cs2::map::Mode;
    use gsi_cs2::round::BombState;
    use gsi_cs2::team::TeamClass;

    #[test]
    fn casual_bomb_rounds_use_the_fixed_2700_win_award() {
        assert_eq!(
            round_win_bonus(
                &TeamClass::CT,
                Some(&BombState::Defused),
                &Mode::Casual,
                "de_mirage",
                Some("ct_win_defuse"),
            ),
            2700
        );
        assert_eq!(
            round_win_bonus(
                &TeamClass::T,
                None,
                &Mode::Casual,
                "de_dust2",
                Some("t_win_elimination"),
            ),
            2700
        );
    }

    #[test]
    fn casual_loss_award_is_always_2400() {
        assert_eq!(loss_bonus(1, &Mode::Casual, &TeamClass::CT, None), 2400);
        assert_eq!(loss_bonus(5, &Mode::Casual, &TeamClass::T, None), 2400);
    }

    #[test]
    fn a_losing_t_team_gets_the_planted_bomb_reward_after_a_defuse() {
        assert_eq!(
            loss_bonus(1, &Mode::Casual, &TeamClass::T, Some(&BombState::Defused)),
            2600
        );
        assert_eq!(
            loss_bonus(
                1,
                &Mode::Competitive,
                &TeamClass::T,
                Some(&BombState::Defused)
            ),
            2500
        );
    }

    #[test]
    fn competitive_loss_level_is_direct_but_game_specific_plant_income_is_preserved() {
        for economy in [EconomyVersion::Cs2, EconomyVersion::CsgoLegacy] {
            for (level, expected) in [
                (0, 1400),
                (1, 1900),
                (2, 2400),
                (3, 2900),
                (4, 3400),
            ] {
                assert_eq!(
                    loss_bonus_for(
                        level,
                        &Mode::Competitive,
                        &TeamClass::CT,
                        None,
                        None,
                        economy,
                        false,
                    ),
                    expected
                );
            }
        }

        assert_eq!(
            loss_bonus_for(
                1,
                &Mode::Competitive,
                &TeamClass::T,
                Some(&BombState::Defused),
                None,
                EconomyVersion::Cs2,
                false,
            ),
            2500
        );
        assert_eq!(
            loss_bonus_for(
                1,
                &Mode::Competitive,
                &TeamClass::T,
                Some(&BombState::Defused),
                None,
                EconomyVersion::CsgoLegacy,
                false,
            ),
            2700
        );
    }

    #[test]
    fn hostage_rewards_accept_personal_team_and_combined_payments() {
        assert_eq!(
            hostage_objective_kind(800, &Mode::Casual, false),
            Some("hostage_interact")
        );
        assert_eq!(
            hostage_objective_kind(900, &Mode::Competitive, false),
            Some("hostage_interact")
        );
        assert_eq!(
            hostage_objective_kind(1600, &Mode::Competitive, true),
            Some("hostage_rescue")
        );
        assert_eq!(hostage_objective_kind(700, &Mode::Competitive, false), None);
    }

    #[test]
    fn casual_hostage_rounds_follow_their_separate_awards() {
        assert_eq!(
            round_win_bonus(
                &TeamClass::CT,
                None,
                &Mode::Casual,
                "cs_office",
                Some("ct_win_elimination"),
            ),
            2300
        );
        assert_eq!(
            round_win_bonus(
                &TeamClass::CT,
                None,
                &Mode::Casual,
                "cs_office",
                Some("ct_win_rescue"),
            ),
            3000
        );
        assert_eq!(
            round_win_bonus(
                &TeamClass::T,
                None,
                &Mode::Casual,
                "cs_office",
                Some("t_win_time"),
            ),
            2000
        );
    }
}
