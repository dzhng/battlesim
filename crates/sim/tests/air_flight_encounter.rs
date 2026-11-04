//! The first helicopter checkpoint's fire, the air map's `flight` encounter
//! as its lab (`/lab/air`) plays it: red's rifle squad, beside the route,
//! strikes the helicopter's hull without hurting it; the gun jeep, freed
//! once the helicopter hovers at its goal and held again after its first
//! burst, hurts it and leaves it flying. Each shooter stands inside its
//! weapon's reach at the helicopter's height.

use contract::ids::UnitId;
use contract::scenario::Rules;
use sim::battle::Battle;
use sim::fixtures::{self, CatalogSet};
use sim::flight::{FlightEvent, Struck};
use sim::weapons::VEHICLE_BODY_BASE;

const HELI: UnitId = UnitId(0);
const RIFLES: UnitId = UnitId(1);
const JEEP: UnitId = UnitId(2);
/// The lab's seed (`apps/battle-lab/src/routes/air.tsx`).
const SEED: u64 = 7;

/// What struck the helicopter's hull, and its hit points, through the
/// encounter's ticks.
struct Beat {
    /// Hull hits by red's rifle squad, and by the gun jeep.
    rifle_hits: u32,
    jeep_hits: u32,
    /// The helicopter's hit points before the jeep is freed, and at the end.
    hp_before_jeep: f64,
    hp_after: f64,
    full: f64,
}

fn play() -> Beat {
    let map = sim::maps::load("air").unwrap().definition;
    let encounter = sim::maps::encounter("air", "flight").unwrap();
    let freed = encounter.scripts[1].tick;
    let held = encounter.scripts[2].tick;
    let mut game = fixtures::rules();
    game["catalog"] = serde_json::Value::Array(fixtures::catalog_documents(CatalogSet::Test));
    let rules: Rules = serde_json::from_value(game).unwrap();
    let mut b = Battle::new(&encounter.on(map, rules).unwrap(), SEED);
    let full = b
        .rules()
        .catalog
        .get(b.unit(HELI).unwrap().kind)
        .hull()
        .unwrap()
        .hp;
    let mut shooters = std::collections::BTreeMap::new();
    let (mut rifle_hits, mut jeep_hits) = (0, 0);
    let mut hp_before_jeep = full;
    // The lab's last look: ten seconds after the hold.
    while b.tick() < held + 10 * u64::from(b.rules().tick_hz) {
        b.step();
        for e in b.flight_events() {
            let FlightEvent::Impact(i) = e else { continue };
            if i.struck != Struck::Body(sim::flight::BodyId(VEHICLE_BODY_BASE + HELI.0)) {
                continue;
            }
            match shooters.get(&i.projectile) {
                Some(&u) if u == RIFLES.0 => rifle_hits += 1,
                Some(&u) if u == JEEP.0 => jeep_hits += 1,
                _ => {}
            }
        }
        for (p, r) in b.rounds() {
            shooters.entry(p.id).or_insert(r.unit.0);
        }
        if b.tick() + 1 == freed {
            hp_before_jeep = b.unit(HELI).unwrap().hp;
        }
    }
    let heli = b.unit(HELI).unwrap();
    Beat {
        rifle_hits,
        jeep_hits,
        hp_before_jeep,
        hp_after: if heli.alive() { heli.hp } else { 0.0 },
        full,
    }
}

#[test]
fn the_rifle_squad_strikes_the_helicopter_without_hurting_it() {
    let beat = play();
    assert!(
        beat.rifle_hits >= 5,
        "{} rifle hits on its hull",
        beat.rifle_hits
    );
    assert_eq!(
        beat.hp_before_jeep, beat.full,
        "the rifles hurt the helicopter"
    );
}

#[test]
fn the_freed_gun_jeep_hurts_the_helicopter_and_leaves_it_flying() {
    let beat = play();
    assert!(beat.jeep_hits > 0, "the autocannon never struck it");
    assert!(
        beat.hp_after > 0.0 && beat.hp_after < beat.full,
        "hit points {} of {}",
        beat.hp_after,
        beat.full
    );
}
