#[cfg(test)]
mod gsi_sanitizer_tests {
    use super::{
        GsiGameVersion, parse_gsi_body, parse_gsi_body_with_version,
        sanitize_cs2_numeric_fields,
    };

    fn sample_payload() -> serde_json::Value {
        serde_json::json!({
            "auth": { "token": "t" },
            "map": {
                "mode": "deathmatch",
                "name": "de_dust2",
                "phase": "live",
                "round": 300,
                "round_wins": { "1": "ct_win", "500": "t_win" },
                "team_ct": {
                    "score": 500,
                    "consecutive_round_losses": 0,
                    "timeouts_remaining": 1,
                    "matches_won_this_series": 0
                },
                "team_t": {
                    "score": 12,
                    "consecutive_round_losses": 2,
                    "timeouts_remaining": 1,
                    "matches_won_this_series": 0
                },
                "num_matches_to_win_series": 0
            },
            "player": {
                "steamid": "76561198000000000",
                "state": {
                    "health": 100,
                    "armor": 500,
                    "helmet": true,
                    "flashed": 0,
                    "smoked": 0,
                    "burning": 0,
                    "money": 16000,
                    "round_kills": 0,
                    "round_killhs": 0,
                    "equip_value": 4500,
                    "defusekit": false
                },
                "match_stats": {
                    "kills": 7,
                    "assists": 1,
                    "deaths": 3,
                    "mvps": 2,
                    "score": 40
                }
            }
        })
    }

    #[test]
    fn oversized_counters_are_clamped_not_rejected() {
        let mut value = sample_payload();
        sanitize_cs2_numeric_fields(&mut value);
        let map = value.get("map").unwrap();
        assert_eq!(map.get("round"), Some(&serde_json::json!(255)));
        assert_eq!(map.pointer("/team_ct/score"), Some(&serde_json::json!(255)));
        // Round-win keys beyond u8 are dropped so the map key stays parseable.
        assert!(
            map.get("round_wins")
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("1")
        );
        assert!(
            !map.get("round_wins")
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("500")
        );
        assert_eq!(
            value.pointer("/player/state/armor"),
            Some(&serde_json::json!(255))
        );
        // In-range values pass through untouched.
        assert_eq!(
            value.pointer("/map/team_t/score"),
            Some(&serde_json::json!(12))
        );
    }

    #[test]
    fn oversized_u8_field_no_longer_fails_cs2_parse() {
        // Regression for the field report "invalid value: integer `500`,
        // expected u8" that made the service drop every GSI payload.
        let mut value = sample_payload();
        value["auth"] = serde_json::json!({ "token": "killconfirm" });
        let body = serde_json::to_vec(&value).expect("serialize sample payload");
        let parsed = parse_gsi_body(&body, GsiGameVersion::Cs2);
        assert!(
            parsed.is_ok(),
            "expected payload to parse: {:?}",
            parsed.err()
        );
        let parsed = parsed.unwrap();
        assert_eq!(parsed.player.unwrap().state.unwrap().armor, 255);
    }

    #[test]
    fn legacy_payload_is_auto_detected_while_cs2_is_selected() {
        let value = serde_json::json!({
            "provider": {
                "name": "Counter-Strike: Global Offensive",
                "appid": 730,
                "version": "13804",
                "steamid": "76561198000000000",
                "timestamp": 1700000000
            },
            "map": {
                "mode": "competitive",
                "name": "de_dust2",
                "phase": "live",
                "round": 4,
                "current_spectators": 1,
                "team_ct": { "score": 2 },
                "team_t": { "score": 1 }
            },
            "player": {
                "steamid": "76561198000000001",
                "name": "Observed Player",
                "team": "CT",
                "state": {
                    "health": 100,
                    "armor": 50,
                    "money": 3200,
                    "round_kills": 1,
                    "round_killhs": 0
                },
                "match_stats": { "kills": 4, "assists": 1, "deaths": 2, "mvps": 0, "score": 9 },
                "weapons": {}
            },
            "round": { "phase": "live" },
            "auth": { "token": "killconfirm" }
        });
        let body = serde_json::to_vec(&value).unwrap();
        let (parsed, detected) =
            parse_gsi_body_with_version(&body, GsiGameVersion::Cs2).unwrap();

        assert_eq!(detected, GsiGameVersion::CsgoLegacy);
        assert_eq!(parsed.player.unwrap().state.unwrap().round_kills, 1);
    }

    #[test]
    fn cs2_payload_is_auto_detected_while_legacy_is_selected() {
        let mut value = sample_payload();
        value["provider"] = serde_json::json!({
            "name": "Counter-Strike 2",
            "appid": 730,
            "version": 14000,
            "steamid": "76561198000000000",
            "timestamp": 1700000000
        });
        value["auth"] = serde_json::json!({ "token": "killconfirm" });
        let body = serde_json::to_vec(&value).unwrap();
        let (parsed, detected) =
            parse_gsi_body_with_version(&body, GsiGameVersion::CsgoLegacy).unwrap();

        assert_eq!(detected, GsiGameVersion::Cs2);
        assert_eq!(parsed.player.unwrap().state.unwrap().armor, 255);
    }
}
