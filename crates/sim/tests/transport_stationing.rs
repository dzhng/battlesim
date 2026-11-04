//! Dropped-off infantry hold the landing area when they meet a threat.
use contract::command::{CommandEnvelope, Order};
use contract::ids::{Side, UnitId};
use serde_json::json;
use sim::battle::Battle;

fn order(battle: &mut Battle, seq: u64, order: Order) {
    let ack = battle.accept(CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq,
        order,
        queued: false,
    });
    assert_eq!(ack.error, None, "{ack:?}");
}

#[test]
fn dropped_off_squads_hold_the_landing_area_when_they_spot_an_enemy() {
    for carrier in ["test_apc", "test_transport_heli"] {
        let setup = crate::common::scenario_with(
            &json!({"size":[800,600],"fog_cell_m":8,"height_grid_m":4,
                "slope_cutoff_deg":35,"props":[],"buildings":[]})
            .to_string(),
            json!([
                {"side":"blue","kind":carrier,"position":[200,300],"engagement":"return_fire_only"},
                {"side":"blue","kind":"test_rifle","position":[195,300],"engagement":"return_fire_only"},
                {"side":"red","kind":"test_rifle","position":[500,450],"engagement":"return_fire_only"}
            ]),
            json!([]),
            json!([]),
        );
        let mut battle = Battle::new(&setup, 13);
        order(
            &mut battle,
            1,
            Order::Board {
                units: vec![UnitId(1)],
                carrier: UnitId(0),
                gesture: 1,
            },
        );
        let aboard = (0..30 * battle.rules().tick_hz).any(|_| {
            battle.step();
            battle
                .unit(UnitId(1))
                .unwrap()
                .hold
                .as_ref()
                .is_some_and(|h| h.phase == sim::hold::Phase::Inside)
        });
        assert!(aboard, "{carrier}: squad never boarded");
        order(
            &mut battle,
            2,
            Order::Unload {
                units: vec![UnitId(0)],
                goal: Some([500.0, 300.0]),
                gesture: 2,
                route: Default::default(),
            },
        );
        let landed = (0..120 * battle.rules().tick_hz).any(|_| {
            battle.step();
            battle.unit(UnitId(1)).unwrap().hold.is_none()
        });
        assert!(landed, "{carrier}: squad never unloaded");
        let landing = battle.unit(UnitId(1)).unwrap().position.xy();
        assert!((landing - sim::math::v2(500.0, 300.0)).length() < 20.0);
        let radius = sim::cover::area_radius(
            battle.rules(),
            battle
                .rules()
                .catalog
                .get(battle.unit(UnitId(1)).unwrap().kind),
        );
        let mut saw_enemy = false;
        for _ in 0..30 * battle.rules().tick_hz {
            battle.step();
            let frame = battle.observe(Side::Blue);
            saw_enemy |= !frame.identified.is_empty();
            let squad = battle.unit(UnitId(1)).unwrap();
            assert!(
                (squad.position.xy() - landing).length() <= radius + 5.0,
                "{carrier}: idle squad left its landing area: landing={landing:?}, now={:?}",
                squad.position
            );
        }
        assert!(saw_enemy, "{carrier}: no threat triggered taking cover");
        let mut replay = Battle::from_replay(&setup, &battle.replay()).unwrap();
        while replay.tick() < battle.tick() {
            replay.step();
        }
        assert_eq!(
            replay.digest(),
            battle.digest(),
            "{carrier}: replay diverged"
        );
    }
}
