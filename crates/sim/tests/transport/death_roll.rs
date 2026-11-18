//! Wreck rolling under passengers who remain aboard an overhead carrier.
use super::*;

fn battle_soldier_radius() -> f64 {
    test_rules()["physics"]["soldier_radius_m"]
        .as_f64()
        .expect("a soldier's radius")
}

fn killed_driving_with_riders_overhead(hover: [f64; 2]) -> Rolled {
    let mut battle = Battle::new(
        &setup(json!([
            {"side":"blue","kind":"test_transport_heli","position":[300,150],"engagement":"return_fire_only"},
            {"side":"blue","kind":"test_rifle","position":[300,150]},
            {"side":"blue","kind":"test_tank","position":[100,300],"condition":{"hp":1},"engagement":"return_fire_only"},
            {"side":"red","kind":"test_tank","position":[300,420],"yaw":-1.2,"engagement":"return_fire_only"}
        ])),
        1,
    );
    let board = Order::Board {
        units: vec![UnitId(1)],
        carrier: UnitId(0),
        gesture: 1,
    };
    assert_eq!(battle.accept(command(1, board)).error, None);
    let loaded = (0..battle.rules().tick_hz * 20).any(|_| {
        battle.step();
        !battle.unit(UnitId(1)).unwrap().on_field()
    });
    assert!(loaded, "the squad never boarded the helicopter");
    assert_eq!(battle.accept(heli_move(2, hover)).error, None);
    for _ in 0..battle.rules().tick_hz * 20 {
        battle.step();
    }
    let drive = Order::Move {
        units: vec![UnitId(2)],
        gesture: 4,
        goal: [600.0, 300.0],
        route: RoutePolicy::Shortest,
        direction: MoveDirection::Forward,
        facing: None,
    };
    assert_eq!(battle.accept(command(3, drive)).error, None);
    for _ in 0..battle.rules().tick_hz * 4 {
        battle.step();
    }
    let tank_kind = battle.unit(UnitId(2)).unwrap().kind;
    let target = battle
        .observe(Side::Red)
        .identified
        .iter()
        .find(|u| u.kind == tank_kind)
        .map(|u| u.id)
        .expect("red sees blue's tank");
    let attack = CommandEnvelope {
        side: Side::Red,
        player: None,
        seq: 1,
        order: Order::Attack {
            units: vec![UnitId(3)],
            target: contract::command::TargetRef::Identified { id: target },
        },
        queued: false,
    };
    assert_eq!(battle.accept(attack).error, None);
    let mut last = None;
    for _ in 0..battle.rules().tick_hz * 30 {
        let tank = battle.unit(UnitId(2)).unwrap();
        if !tank.alive() {
            break;
        }
        last = Some(([tank.position.x, tank.position.y], tank.drive_speed_mps));
        battle.step();
    }
    assert!(
        !battle.unit(UnitId(2)).unwrap().alive(),
        "red never killed the tank"
    );
    let (at, speed) = last.expect("it drove");
    let tank = battle.unit(UnitId(2)).unwrap();
    let braking = tank.ground(battle.rules()).road_mps / battle.rules().movement.drive.wreck_stop_s;
    let wreck = battle
        .observe(Side::Blue)
        .known_props
        .iter()
        .find(|p| p.wreck_of == Some(tank_kind))
        .expect("blue knows its tank's wreck")
        .center;
    let heli = battle.unit(UnitId(0)).unwrap().position;
    assert!(
        !battle.unit(UnitId(1)).unwrap().on_field(),
        "the squad still rides"
    );
    Rolled {
        at,
        speed,
        braking,
        wreck,
        front: tank.hull.expect("a tank's hull").x,
        heli: [heli.x, heli.y],
    }
}

struct Rolled {
    at: [f64; 2],
    speed: f64,
    braking: f64,
    wreck: [f64; 2],
    /// From the tank's middle to its front.
    front: f64,
    heli: [f64; 2],
}

#[test]
fn a_wreck_rolls_on_under_soldiers_riding_a_helicopter() {
    // Find the stretch the wreck rolls with the helicopter well away, then
    // hover over the ground its front rolls onto.
    let away = killed_driving_with_riders_overhead([300.0, 150.0]);
    let onto = away.front + (away.wreck[0] - away.at[0]) / 2.0;
    let Rolled {
        at,
        speed,
        braking,
        wreck,
        front,
        heli,
    } = killed_driving_with_riders_overhead([away.at[0] + onto, away.at[1]]);
    assert!(speed > 3.0, "it was driving: {speed} m/s");
    let rolled = speed * speed / (2.0 * braking);
    let ahead = wreck[0] - at[0];
    let radius = battle_soldier_radius();
    assert!(
        heli[0] - at[0] > front + radius
            && heli[0] - at[0] < front + rolled
            && (heli[1] - at[1]).abs() < 0.5,
        "the helicopter {heli:?} is not over the ground the tank's front rolls onto from {at:?} to {wreck:?}"
    );
    assert!(
        (ahead - rolled).abs() < speed / 30.0 + 0.05,
        "rolled {ahead} m under the riders, locked tracks stop it in {rolled} m"
    );
}
