use gsi_cs2::map::Mode;
use gsi_cs2::round::BombState;
use gsi_cs2::team::TeamClass;
use gsi_cs2::weapon::WeaponName;

pub fn uses_standard_cash_economy(mode: &Mode) -> bool {
    matches!(mode, Mode::Casual | Mode::Competitive | Mode::Wingman)
}

pub fn weapon_kill_reward(weapon_name: &WeaponName, mode: &Mode) -> u16 {
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
        WeaponName::XM1014
        | WeaponName::MAC10
        | WeaponName::MP5SD
        | WeaponName::MP7
        | WeaponName::MP9
        | WeaponName::Bizon
        | WeaponName::UMP45 => 600,
        WeaponName::AWP | WeaponName::Zeus27 => 100,
        _ => 300,
    };

    match mode {
        Mode::Casual => reward / 2,
        Mode::Competitive | Mode::Wingman => reward,
        _ => 0,
    }
}

pub fn default_kill_reward(mode: &Mode) -> u16 {
    match mode {
        Mode::Casual => 150,
        Mode::Competitive | Mode::Wingman => 300,
        _ => 0,
    }
}

pub fn bomb_objective_reward(mode: &Mode) -> u16 {
    match mode {
        Mode::Casual => 200,
        Mode::Competitive | Mode::Wingman => 300,
        _ => 0,
    }
}

pub fn loss_bonus(
    consecutive_round_losses: u8,
    mode: &Mode,
    player_team: &TeamClass,
    bomb: Option<&BombState>,
    round_outcome: Option<&str>,
) -> u16 {
    let base_reward = match mode {
        Mode::Casual => 2400,
        Mode::Competitive => match consecutive_round_losses.max(1) {
            1 => 1400,
            2 => 1900,
            3 => 2400,
            4 => 2900,
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
        if matches!(mode, Mode::Casual) {
            200
        } else {
            600
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
    if !uses_standard_cash_economy(mode) {
        return None;
    }

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
    if !uses_standard_cash_economy(mode) {
        return 0;
    }

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
        (_, TeamClass::T, Some(BombState::Exploded))
        | (_, TeamClass::CT, Some(BombState::Defused)) => 3500,
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
        bomb_objective_reward, default_kill_reward, hostage_objective_kind, loss_bonus,
        round_win_bonus, weapon_kill_reward,
    };
    use gsi_cs2::map::Mode;
    use gsi_cs2::round::BombState;
    use gsi_cs2::team::TeamClass;
    use gsi_cs2::weapon::WeaponName;

    #[test]
    fn modes_without_cash_economy_do_not_report_rewards() {
        for mode in [
            Mode::ArmsRace,
            Mode::Custom,
            Mode::Deathmatch,
            Mode::Demolition,
            Mode::Survival,
            Mode::Training,
        ] {
            assert_eq!(weapon_kill_reward(&WeaponName::AK47, &mode), 0);
            assert_eq!(default_kill_reward(&mode), 0);
            assert_eq!(bomb_objective_reward(&mode), 0);
            assert_eq!(loss_bonus(5, &mode, &TeamClass::T, None, None), 0);
            assert_eq!(
                round_win_bonus(
                    &TeamClass::CT,
                    None,
                    &mode,
                    "de_dust2",
                    Some("ct_win_elimination"),
                ),
                0
            );
        }
    }

    #[test]
    fn wingman_uses_its_separate_round_economy() {
        assert_eq!(
            loss_bonus(1, &Mode::Wingman, &TeamClass::CT, None, None),
            2000
        );
        assert_eq!(
            loss_bonus(5, &Mode::Wingman, &TeamClass::CT, None, None),
            3200
        );
        assert_eq!(
            loss_bonus(
                1,
                &Mode::Wingman,
                &TeamClass::T,
                None,
                Some("ct_win_defuse"),
            ),
            2600
        );
        assert_eq!(
            round_win_bonus(
                &TeamClass::CT,
                None,
                &Mode::Wingman,
                "de_inferno",
                Some("ct_win_elimination"),
            ),
            2750
        );
        assert_eq!(
            round_win_bonus(
                &TeamClass::T,
                None,
                &Mode::Wingman,
                "de_inferno",
                Some("t_win_bomb"),
            ),
            3000
        );
    }

    #[test]
    fn round_outcome_preserves_objective_win_reward_when_bomb_state_is_absent() {
        assert_eq!(
            round_win_bonus(
                &TeamClass::CT,
                None,
                &Mode::Competitive,
                "de_nuke",
                Some("ct_win_defuse"),
            ),
            3500
        );
        assert_eq!(
            round_win_bonus(
                &TeamClass::T,
                None,
                &Mode::Competitive,
                "de_nuke",
                Some("t_win_bomb"),
            ),
            3500
        );
    }

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
        assert_eq!(
            loss_bonus(1, &Mode::Casual, &TeamClass::CT, None, None),
            2400
        );
        assert_eq!(
            loss_bonus(5, &Mode::Casual, &TeamClass::T, None, None),
            2400
        );
    }

    #[test]
    fn a_losing_t_team_gets_the_planted_bomb_reward_after_a_defuse() {
        assert_eq!(
            loss_bonus(
                1,
                &Mode::Casual,
                &TeamClass::T,
                Some(&BombState::Defused),
                None,
            ),
            2600
        );
        assert_eq!(
            loss_bonus(
                1,
                &Mode::Competitive,
                &TeamClass::T,
                Some(&BombState::Defused),
                None,
            ),
            2000
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
