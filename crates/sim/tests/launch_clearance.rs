//! A launcher that needs a clear launch path (`requires_clear_launch_path`)
//! holds fire while anything stands in its round's first flight from the
//! muzzle: tree crowns over it, a wall at its muzzle, a body its shell could
//! otherwise blow apart. Only the launcher's end is judged: a high arc
//! clears a low wall, and a wood or a hill farther on is still fair. It holds
//! before launching, so it spends nothing, and says why.
use contract::command::{Order, TargetRef};
use contract::ids::{Side, UnitId};
use contract::observation::ActionReason;
use serde_json::{json, Value};
use sim::battle::Battle;

use crate::common;

/// Where the launcher stands, facing east; its target is 600 m east.
const AT: [f64; 2] = [200.0, 300.0];
const TARGET: [f64; 3] = [800.0, 300.0, 0.0];

/// The test launcher: a mortar whose slower shell flies a high arc of about
/// 55° at 600 m, so what stands a few metres before its muzzle is what it
/// must clear.
fn launcher(requires: bool) -> Value {
    json!({
        "extends": "artillery_mortar",
        "speed_mps": 80,
        "range_m": 650,
        "targets": { "ground": { "range_m": 650, "scatter_mrad": 26 } },
        "requires_clear_launch_path": requires,
    })
}

fn rules_with(row: Value, clearance_m: f64) -> Value {
    let mut rules = common::game();
    rules["weapons"]["test_launcher"] = row;
    rules["weapons"]["test_launcher"]["round_cost"] = json!(1);
    rules["launch"]["clearance_m"] = json!(clearance_m);
    sim::fixtures::patch_catalog(
        &mut rules,
        "units",
        "test_jeep",
        json!({
            "mounts": [{ "id": "Launcher", "name": "Launcher", "weapons": ["test_launcher"],
                "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }],
        }),
    );
    rules
}

/// A blue jeep carrying `row` at [`AT`] on a flat field with `cover`
/// (props, forests, relief), ordered to shell [`TARGET`].
fn shelling(row: Value, cover: Value) -> Battle {
    let mut map = json!({ "size": [1200, 600], "fog_cell_m": 8, "height_grid_m": 4,
        "slope_cutoff_deg": 35 });
    for (key, value) in cover.as_object().unwrap() {
        map[key] = value.clone();
    }
    let setup = serde_json::from_value(json!({
        "map": map,
        "rules": rules_with(row, 30.0),
        "units": [{ "side": "blue", "kind": "test_jeep", "position": AT, "yaw": 0.0 }],
        "events": [], "scripts": [],
    }))
    .unwrap();
    let mut b = Battle::new(&setup, 3);
    let ack = b.accept(contract::command::CommandEnvelope {
        side: Side::Blue,
        player: None,
        seq: 1,
        queued: false,
        order: Order::Attack {
            units: vec![UnitId(0)],
            target: TargetRef::Ground { point: TARGET },
        },
    });
    assert_eq!(ack.error, None);
    b
}

/// What the launcher (the jeep's second mount, after its own gun) did over
/// 20 s: whether it launched, and the reason and ammunition it showed last
/// while it stood.
fn outcome(b: &mut Battle) -> (bool, ActionReason, Option<u32>) {
    let mut launched = false;
    let mut shown = None;
    for _ in 0..600 {
        b.step();
        launched |= b.rounds().any(|(_, r)| r.unit == UnitId(0));
        if let Some(own) = b.observe(Side::Blue).own.first() {
            let mount = own.mounts.iter().find(|m| m.mount == 1).unwrap();
            shown = Some((mount.reason, mount.ammo[0]));
        }
    }
    let (reason, ammo) = shown.expect("the launcher stood");
    (launched, reason, ammo)
}

/// The launcher under trees, ordered to fire: it holds.
pub(crate) fn held_under_trees() -> Battle {
    shelling(
        launcher(true),
        json!({ "forests": forest([[150.0, 250.0], [250.0, 250.0], [250.0, 350.0], [150.0, 350.0]]) }),
    )
}

fn forest(ring: [[f64; 2]; 4]) -> Value {
    json!([{ "shape": { "kind": "polygon", "ring": ring } }])
}

/// A box prop of `kind` standing `ahead` metres east of the launcher, `tall`
/// metres high and 10 m wide across its line of fire.
fn wall(kind: &str, ahead: f64, tall: f64) -> Value {
    json!([{ "kind": kind, "center": [AT[0] + ahead, AT[1]], "yaw": 0,
        "half_extents": [0.5, 5.0, tall / 2.0] }])
}

#[test]
fn a_launcher_needing_a_clear_path_holds_under_cover_and_fires_past_it() {
    let full = Some(36);
    let holds = (false, ActionReason::BlockedTrajectory, full);
    for (name, cover, held) in [
        ("in the open", json!({}), false),
        (
            "under trees",
            json!({ "forests": forest([[150.0, 250.0], [250.0, 250.0], [250.0, 350.0], [150.0, 350.0]]) }),
            true,
        ),
        // A wall it cannot arc over, at its muzzle.
        (
            "before a high wall",
            json!({ "props": wall("wall", 4.0, 14.0) }),
            true,
        ),
        // Sandbags its shells could blow apart, stacked high at its muzzle:
        // a gun would fire into them; a launcher needing a clear path holds.
        (
            "behind a tall stack of sandbags",
            json!({ "props": wall("sandbags", 4.0, 8.0) }),
            true,
        ),
        // The same sandbags waist-high: the arc clears them.
        (
            "behind low sandbags",
            json!({ "props": wall("sandbags", 4.0, 1.0) }),
            false,
        ),
        (
            "into a wood",
            json!({ "forests": forest([[740.0, 240.0], [860.0, 240.0], [860.0, 360.0], [740.0, 360.0]]) }),
            false,
        ),
        (
            "over a hill",
            json!({ "relief": [{ "kind": "mesa", "rect": [330, 0, 380, 600], "height_m": 20, "side_degrees": 45 }] }),
            false,
        ),
    ] {
        let (launched, reason, ammo) = outcome(&mut shelling(launcher(true), cover));
        if held {
            assert_eq!(
                (launched, reason, ammo),
                holds,
                "{name}: holds, spending nothing"
            );
        } else {
            assert!(launched, "{name}: fires ({reason:?})");
            assert!(ammo < full, "{name}: spends its shells");
        }
    }
}

/// The hold is the launcher's own: the same mortar not needing a clear path
/// fires through the crowns, and into the sandbags it can break, as every
/// gun does (and its first shell, bursting at its muzzle, may take the
/// launcher with it).
#[test]
fn a_launcher_not_needing_a_clear_path_fires_through_cover_it_can_break() {
    for (name, cover) in [
        (
            "under trees",
            json!({ "forests": forest([[150.0, 250.0], [250.0, 250.0], [250.0, 350.0], [150.0, 350.0]]) }),
        ),
        (
            "behind a tall stack of sandbags",
            json!({ "props": wall("sandbags", 4.0, 8.0) }),
        ),
    ] {
        let (launched, reason, _) = outcome(&mut shelling(launcher(false), cover));
        assert!(launched, "{name}: fires ({reason:?})");
    }
}

/// A long-range SAM and a short-range gun side by side under trees, a jet
/// passing overhead: the gun swings up and fires through the leaves, while
/// the SAM, needing a clear path out of its tubes, holds. In the open, the
/// same SAM fires.
#[test]
fn a_sam_under_trees_holds_while_a_gun_beside_it_fires() {
    let mut rules = common::game();
    rules["launch"]["clearance_m"] = json!(30.0);
    // The SAM needs a clear launch path; the gun does not.
    rules["weapons"]["aa_long"]["requires_clear_launch_path"] = json!(true);
    rules["weapons"]["aa_ballistic"]["requires_clear_launch_path"] = json!(false);
    let jeep = |weapon: &str| {
        json!({
            "mounts": [{ "id": "Launcher", "name": "Launcher", "weapons": [weapon],
                "turret": true, "pivot_m": [0, 0, 1.68], "muzzle_m": [1.43, 0, 0.32] }],
            "sensors": { "high_air_m": 3000 },
        })
    };
    let run = |weapon: &str, covered: bool| {
        let mut rules = rules.clone();
        sim::fixtures::patch_catalog(&mut rules, "units", "test_jeep", jeep(weapon));
        let mut map = json!({ "size": [2400, 800], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35 });
        if covered {
            map["forests"] = forest([
                [1100.0, 300.0],
                [1300.0, 300.0],
                [1300.0, 500.0],
                [1100.0, 500.0],
            ]);
        }
        let setup = serde_json::from_value(json!({
            "map": map,
            "rules": rules,
            "units": [
                { "side": "blue", "kind": "test_jeep", "position": [1200, 400], "yaw": 0.0 },
                { "side": "red", "kind": "test_jet", "position": [2300, 400],
                  "yaw": std::f64::consts::PI, "engagement": "return_fire_only" },
            ],
            "events": [], "scripts": [],
        }))
        .unwrap();
        let mut b = Battle::new(&setup, 3);
        b.step();
        let ack = b.accept(contract::command::CommandEnvelope {
            side: Side::Red,
            ..crate::air::order(&[1], [100.0, 400.0])
        });
        assert_eq!(ack.error, None, "{ack:?}");
        let (mut launched, mut blocked) = (false, false);
        for _ in 0..450 {
            b.step();
            launched |= b.rounds().any(|(_, r)| r.unit == UnitId(0));
            let own = &b.observe(Side::Blue).own[0];
            blocked |= own
                .mounts
                .iter()
                .any(|m| m.reason == ActionReason::BlockedTrajectory);
        }
        (launched, blocked)
    };
    assert_eq!(
        run("aa_long", true),
        (false, true),
        "the SAM under trees holds"
    );
    assert!(
        run("aa_ballistic", true).0,
        "the gun under the same trees fires"
    );
    assert!(run("aa_long", false).0, "the SAM in the open fires");
}
