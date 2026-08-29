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
        if is_legacy {
            Self::CsgoLegacy
        } else {
            Self::Cs2
        }
    }
}

pub fn uses_standard_cash_economy(mode: &Mode) -> bool {
    matches!(mode, Mode::Casual | Mode::Competitive | Mode::Wingman)
}

/// CS2 records an assist as contribution/score, but does not grant player cash for it.
pub fn assist_reward_for(_economy: EconomyVersion) -> u16 {
    0
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
        WeaponName::XM1014
        | WeaponName::MAC10
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

pub fn bomb_objective_reward_for(mode: &Mode, _economy: EconomyVersion) -> u16 {
    match mode {
        Mode::Casual => 200,
        Mode::Competitive | Mode::Wingman => 300,
        _ => 0,
    }
}

/// Competitive and Wingman can grant this after every round once the server marks
/// the team as eligible for short-handed income. GSI does not expose that eligibility,
/// so rules mode cannot add it speculatively; delta mode can recognize the actual payout.
pub fn short_handed_bonus_for(mode: &Mode, _economy: EconomyVersion) -> u16 {
    match mode {
        Mode::Competitive | Mode::Wingman => 1000,
        _ => 0,
    }
}

/// Since the 2025-07-16 CS2 update, every Counter-Terrorist receives $50 for
/// each Terrorist eliminated during a Competitive or Wingman round. This is a
/// team award paid on both wins and losses; CS:GO Legacy does not have it.
pub fn ct_elimination_team_reward_for(
    eliminated_terrorists: u16,
    mode: &Mode,
    economy: EconomyVersion,
) -> u16 {
    if !matches!(economy, EconomyVersion::Cs2) {
        return 0;
    }

    let maximum_eliminations = match mode {
        Mode::Competitive => 5,
        Mode::Wingman => 2,
        _ => 0,
    };

    eliminated_terrorists.min(maximum_eliminations) * 50
}

pub fn maximum_ct_elimination_team_reward_for(mode: &Mode, economy: EconomyVersion) -> u16 {
    ct_elimination_team_reward_for(u16::MAX, mode, economy)
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
    // A surviving terrorist gets no loss income when CT wins a defusal map by timeout.
    if matches!(player_team, TeamClass::T)
        && round_outcome == Some("ct_win_time")
        && player_survived
    {
        return 0;
    }

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

pub fn hostage_objective_kind_for(
    reward: u16,
    mode: &Mode,
    is_rescue_round: bool,
    _economy: EconomyVersion,
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

pub fn round_win_bonus_for(
    win_team: &TeamClass,
    bomb: Option<&BombState>,
    mode: &Mode,
    map_name: &str,
    round_outcome: Option<&str>,
    _economy: EconomyVersion,
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
    use super::*;
    use gsi_cs2::map::Mode;
    use gsi_cs2::round::BombState;
    use gsi_cs2::team::TeamClass;
    use gsi_cs2::weapon::WeaponName;

    #[test]
    fn shared_personal_and_team_rules_are_explicit_for_both_games() {
        for economy in [EconomyVersion::Cs2, EconomyVersion::CsgoLegacy] {
            assert_eq!(assist_reward_for(economy), 0);
            assert_eq!(default_kill_reward_for(&Mode::Competitive, economy), 300);
            assert_eq!(default_kill_reward_for(&Mode::Casual, economy), 150);
            assert_eq!(bomb_objective_reward_for(&Mode::Competitive, economy), 300);
            assert_eq!(bomb_objective_reward_for(&Mode::Wingman, economy), 300);
            assert_eq!(bomb_objective_reward_for(&Mode::Casual, economy), 200);
            assert_eq!(short_handed_bonus_for(&Mode::Competitive, economy), 1000);
            assert_eq!(short_handed_bonus_for(&Mode::Wingman, economy), 1000);
            assert_eq!(short_handed_bonus_for(&Mode::Casual, economy), 0);
        }
    }

    #[test]
    fn cs2_alone_grants_the_ct_elimination_team_award() {
        assert_eq!(
            ct_elimination_team_reward_for(5, &Mode::Competitive, EconomyVersion::Cs2),
            250
        );
        assert_eq!(
            ct_elimination_team_reward_for(2, &Mode::Wingman, EconomyVersion::Cs2),
            100
        );
        assert_eq!(
            ct_elimination_team_reward_for(5, &Mode::Casual, EconomyVersion::Cs2),
            0
        );
        assert_eq!(
            ct_elimination_team_reward_for(5, &Mode::Competitive, EconomyVersion::CsgoLegacy,),
            0
        );
        assert_eq!(
            maximum_ct_elimination_team_reward_for(&Mode::Competitive, EconomyVersion::Cs2),
            250
        );
    }

    #[test]
    fn cs2_and_legacy_use_their_own_changed_weapon_rewards() {
        let cases = [
            (WeaponName::CZ75A, 300, 100, 150, 50),
            (WeaponName::XM1014, 600, 900, 300, 450),
            (WeaponName::Zeus27, 100, 0, 50, 0),
        ];

        for (weapon, cs2, legacy, cs2_casual, legacy_casual) in cases {
            assert_eq!(
                weapon_kill_reward_for(&weapon, &Mode::Competitive, EconomyVersion::Cs2),
                cs2
            );
            assert_eq!(
                weapon_kill_reward_for(&weapon, &Mode::Competitive, EconomyVersion::CsgoLegacy,),
                legacy
            );
            assert_eq!(
                weapon_kill_reward_for(&weapon, &Mode::Casual, EconomyVersion::Cs2),
                cs2_casual
            );
            assert_eq!(
                weapon_kill_reward_for(&weapon, &Mode::Casual, EconomyVersion::CsgoLegacy,),
                legacy_casual
            );
        }
    }

    #[test]
    fn complete_shared_weapon_reward_groups_remain_unchanged() {
        for economy in [EconomyVersion::Cs2, EconomyVersion::CsgoLegacy] {
            for (weapon, competitive, casual) in [
                (WeaponName::KnifeKarambit, 1500, 750),
                (WeaponName::Nova, 900, 450),
                (WeaponName::MAG7, 900, 450),
                (WeaponName::SawedOff, 900, 450),
                (WeaponName::MAC10, 600, 300),
                (WeaponName::MP5SD, 600, 300),
                (WeaponName::MP7, 600, 300),
                (WeaponName::MP9, 600, 300),
                (WeaponName::Bizon, 600, 300),
                (WeaponName::UMP45, 600, 300),
                (WeaponName::AWP, 100, 50),
                (WeaponName::AK47, 300, 150),
            ] {
                assert_eq!(
                    weapon_kill_reward_for(&weapon, &Mode::Competitive, economy),
                    competitive
                );
                assert_eq!(
                    weapon_kill_reward_for(&weapon, &Mode::Casual, economy),
                    casual
                );
            }
        }
    }

    #[test]
    fn planted_bomb_loss_award_differs_only_where_the_games_differ() {
        for (mode, cs2, legacy) in [
            (Mode::Competitive, 2000, 2200),
            (Mode::Wingman, 2600, 2800),
            (Mode::Casual, 2600, 2600),
        ] {
            assert_eq!(
                loss_bonus_for(
                    1,
                    &mode,
                    &TeamClass::T,
                    Some(&BombState::Defused),
                    None,
                    EconomyVersion::Cs2,
                    false,
                ),
                cs2
            );
            assert_eq!(
                loss_bonus_for(
                    1,
                    &mode,
                    &TeamClass::T,
                    Some(&BombState::Defused),
                    None,
                    EconomyVersion::CsgoLegacy,
                    false,
                ),
                legacy
            );
        }
    }

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
            assert_eq!(
                weapon_kill_reward_for(&WeaponName::AK47, &mode, EconomyVersion::Cs2),
                0
            );
            assert_eq!(default_kill_reward_for(&mode, EconomyVersion::Cs2), 0);
            assert_eq!(bomb_objective_reward_for(&mode, EconomyVersion::Cs2), 0);
            assert_eq!(
                loss_bonus_for(
                    5,
                    &mode,
                    &TeamClass::T,
                    None,
                    None,
                    EconomyVersion::Cs2,
                    false,
                ),
                0
            );
            assert_eq!(
                round_win_bonus_for(
                    &TeamClass::CT,
                    None,
                    &mode,
                    "de_dust2",
                    Some("ct_win_elimination"),
                    EconomyVersion::Cs2,
                ),
                0
            );
        }
    }

    #[test]
    fn wingman_uses_its_separate_round_economy() {
        assert_eq!(
            loss_bonus_for(
                1,
                &Mode::Wingman,
                &TeamClass::CT,
                None,
                None,
                EconomyVersion::Cs2,
                false,
            ),
            2000
        );
        assert_eq!(
            loss_bonus_for(
                5,
                &Mode::Wingman,
                &TeamClass::CT,
                None,
                None,
                EconomyVersion::Cs2,
                false,
            ),
            3200
        );
        assert_eq!(
            loss_bonus_for(
                1,
                &Mode::Wingman,
                &TeamClass::T,
                None,
                Some("ct_win_defuse"),
                EconomyVersion::Cs2,
                false,
            ),
            2600
        );
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::CT,
                None,
                &Mode::Wingman,
                "de_inferno",
                Some("ct_win_elimination"),
                EconomyVersion::Cs2,
            ),
            2750
        );
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::T,
                None,
                &Mode::Wingman,
                "de_inferno",
                Some("t_win_bomb"),
                EconomyVersion::Cs2,
            ),
            3000
        );
    }

    #[test]
    fn round_outcome_preserves_objective_win_reward_when_bomb_state_is_absent() {
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::CT,
                None,
                &Mode::Competitive,
                "de_nuke",
                Some("ct_win_defuse"),
                EconomyVersion::Cs2,
            ),
            3500
        );
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::T,
                None,
                &Mode::Competitive,
                "de_nuke",
                Some("t_win_bomb"),
                EconomyVersion::Cs2,
            ),
            3500
        );
    }

    #[test]
    fn casual_bomb_rounds_use_the_fixed_2700_win_award() {
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::CT,
                Some(&BombState::Defused),
                &Mode::Casual,
                "de_mirage",
                Some("ct_win_defuse"),
                EconomyVersion::Cs2,
            ),
            2700
        );
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::T,
                None,
                &Mode::Casual,
                "de_dust2",
                Some("t_win_elimination"),
                EconomyVersion::Cs2,
            ),
            2700
        );
    }

    #[test]
    fn casual_loss_award_is_always_2400() {
        assert_eq!(
            loss_bonus_for(
                1,
                &Mode::Casual,
                &TeamClass::CT,
                None,
                None,
                EconomyVersion::Cs2,
                false,
            ),
            2400
        );
        assert_eq!(
            loss_bonus_for(
                5,
                &Mode::Casual,
                &TeamClass::T,
                None,
                None,
                EconomyVersion::Cs2,
                false,
            ),
            2400
        );
    }

    #[test]
    fn a_losing_t_team_gets_the_planted_bomb_reward_after_a_defuse() {
        assert_eq!(
            loss_bonus_for(
                1,
                &Mode::Casual,
                &TeamClass::T,
                Some(&BombState::Defused),
                None,
                EconomyVersion::Cs2,
                false,
            ),
            2600
        );
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
            2000
        );
    }

    #[test]
    fn hostage_rewards_accept_personal_team_and_combined_payments() {
        assert_eq!(
            hostage_objective_kind_for(800, &Mode::Casual, false, EconomyVersion::Cs2),
            Some("hostage_interact")
        );
        assert_eq!(
            hostage_objective_kind_for(900, &Mode::Competitive, false, EconomyVersion::CsgoLegacy,),
            Some("hostage_interact")
        );
        assert_eq!(
            hostage_objective_kind_for(1600, &Mode::Competitive, true, EconomyVersion::Cs2,),
            Some("hostage_rescue")
        );
        assert_eq!(
            hostage_objective_kind_for(700, &Mode::Competitive, false, EconomyVersion::Cs2,),
            None
        );
    }

    #[test]
    fn casual_hostage_rounds_follow_their_separate_awards() {
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::CT,
                None,
                &Mode::Casual,
                "cs_office",
                Some("ct_win_elimination"),
                EconomyVersion::Cs2,
            ),
            2300
        );
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::CT,
                None,
                &Mode::Casual,
                "cs_office",
                Some("ct_win_rescue"),
                EconomyVersion::Cs2,
            ),
            3000
        );
        assert_eq!(
            round_win_bonus_for(
                &TeamClass::T,
                None,
                &Mode::Casual,
                "cs_office",
                Some("t_win_time"),
                EconomyVersion::Cs2,
            ),
            2000
        );
    }

    #[test]
    fn surviving_terrorist_gets_no_timeout_loss_income_in_either_game() {
        for economy in [EconomyVersion::Cs2, EconomyVersion::CsgoLegacy] {
            assert_eq!(
                loss_bonus_for(
                    5,
                    &Mode::Competitive,
                    &TeamClass::T,
                    None,
                    Some("ct_win_time"),
                    economy,
                    true,
                ),
                0
            );
            assert_eq!(
                loss_bonus_for(
                    5,
                    &Mode::Competitive,
                    &TeamClass::T,
                    None,
                    Some("ct_win_time"),
                    economy,
                    false,
                ),
                3400
            );
        }
    }

    #[test]
    fn complete_loss_ladders_match_both_game_configs() {
        for economy in [EconomyVersion::Cs2, EconomyVersion::CsgoLegacy] {
            for (mode, expected) in [
                (Mode::Competitive, [1400, 1900, 2400, 2900, 3400]),
                (Mode::Wingman, [2000, 2300, 2600, 2900, 3200]),
                (Mode::Casual, [2400, 2400, 2400, 2400, 2400]),
            ] {
                for (losses, expected_reward) in (1..=5).zip(expected) {
                    assert_eq!(
                        loss_bonus_for(losses, &mode, &TeamClass::CT, None, None, economy, false,),
                        expected_reward
                    );
                }
            }
        }
    }

    #[test]
    fn complete_round_win_matrix_matches_both_game_configs() {
        for economy in [EconomyVersion::Cs2, EconomyVersion::CsgoLegacy] {
            let cases = [
                (
                    Mode::Competitive,
                    "de_nuke",
                    "ct_win_elimination",
                    TeamClass::CT,
                    3250,
                ),
                (
                    Mode::Competitive,
                    "de_nuke",
                    "ct_win_time",
                    TeamClass::CT,
                    3250,
                ),
                (
                    Mode::Competitive,
                    "de_nuke",
                    "ct_win_defuse",
                    TeamClass::CT,
                    3500,
                ),
                (
                    Mode::Competitive,
                    "de_nuke",
                    "t_win_bomb",
                    TeamClass::T,
                    3500,
                ),
                (
                    Mode::Competitive,
                    "cs_office",
                    "ct_win_elimination",
                    TeamClass::CT,
                    3000,
                ),
                (
                    Mode::Competitive,
                    "cs_office",
                    "t_win_elimination",
                    TeamClass::T,
                    3000,
                ),
                (
                    Mode::Competitive,
                    "cs_office",
                    "ct_win_rescue",
                    TeamClass::CT,
                    2900,
                ),
                (
                    Mode::Competitive,
                    "cs_office",
                    "t_win_time",
                    TeamClass::T,
                    3250,
                ),
                (
                    Mode::Wingman,
                    "de_inferno",
                    "ct_win_elimination",
                    TeamClass::CT,
                    2750,
                ),
                (
                    Mode::Wingman,
                    "de_inferno",
                    "ct_win_defuse",
                    TeamClass::CT,
                    3000,
                ),
                (
                    Mode::Wingman,
                    "cs_agency",
                    "ct_win_elimination",
                    TeamClass::CT,
                    2500,
                ),
                (
                    Mode::Wingman,
                    "cs_agency",
                    "ct_win_rescue",
                    TeamClass::CT,
                    2900,
                ),
                (Mode::Wingman, "cs_agency", "t_win_time", TeamClass::T, 2750),
                (
                    Mode::Casual,
                    "de_dust2",
                    "ct_win_elimination",
                    TeamClass::CT,
                    2700,
                ),
                (Mode::Casual, "de_dust2", "t_win_bomb", TeamClass::T, 2700),
                (
                    Mode::Casual,
                    "cs_office",
                    "ct_win_elimination",
                    TeamClass::CT,
                    2300,
                ),
                (
                    Mode::Casual,
                    "cs_office",
                    "t_win_elimination",
                    TeamClass::T,
                    2000,
                ),
                (
                    Mode::Casual,
                    "cs_office",
                    "ct_win_rescue",
                    TeamClass::CT,
                    3000,
                ),
                (Mode::Casual, "cs_office", "t_win_time", TeamClass::T, 2000),
            ];
            for (mode, map, outcome, team, expected) in cases {
                assert_eq!(
                    round_win_bonus_for(&team, None, &mode, map, Some(outcome), economy),
                    expected,
                    "{economy:?} {mode:?} {map} {outcome}"
                );
            }
        }
    }
}
