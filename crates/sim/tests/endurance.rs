//! The endurance load (slice 16): the synthetic stress battle is the size the
//! scale verdict names, and remains are kept, never pruned, as it runs.
use contract::ids::Side;
use sim::battle::Battle;
use sim::endurance::{scenario, LATE_CORPSES, LATE_WRECKS};

use crate::common;

#[test]
fn each_side_fields_100_units_half_of_them_rifle_squads() {
    let s = scenario(&common::village(), 1, false).unwrap();
    for side in Side::ALL {
        let mine: Vec<_> = s.units.iter().filter(|u| u.side == side).collect();
        assert_eq!(mine.len(), 100);
        assert_eq!(mine.iter().filter(|u| u.kind == "rifle").count(), 50);
    }
    assert!(!s.scripts.is_empty(), "seeded waves drive the battle");
    // Same seed, same battle.
    let again = scenario(&common::village(), 1, false).unwrap();
    assert_eq!(
        serde_json::to_string(&s).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn the_late_state_starts_with_its_remains_and_keeps_every_one() {
    let s = scenario(&common::village(), 1, true).unwrap();
    let mut battle = Battle::new(&s, 1);
    let start = battle.load();
    assert!(start.corpses >= LATE_CORPSES);
    assert!(start.wrecks >= LATE_WRECKS);
    assert_eq!(start.living_units, 200);
    let mut last = start;
    for _ in 0..common::tick_hz() * 20 {
        battle.step();
        let now = battle.load();
        assert!(now.corpses >= last.corpses && now.wrecks >= last.wrecks);
        last = now;
    }
}
