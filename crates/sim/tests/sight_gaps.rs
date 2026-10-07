//! Sight passes between two bodies that hide what lies behind them only
//! through a gap at least `sensors.min_sight_gap_m` wide: no squad is spotted
//! through the crack between two houses, and the fog draws no sliver of sight
//! there. A gap is judged between two bodies, so a line running along one
//! row of them never closes, and one that starts or ends in a gap sees out.
use sim::math::{v3, V3};
use sim::world::WorldGeometry;

use crate::common;

/// Two walls 10 m long and 6 m high side by side, `gap` metres apart across
/// x = 45, and a wall row along y = 60 for the facade moments.
fn houses(gap: f64) -> WorldGeometry {
    let wall =
        |x: f64| format!(r#"{{"kind":"wall","center":[{x},50],"yaw":0,"half_extents":[4.7,5,3]}}"#);
    let row = |x: f64| {
        format!(r#"{{"kind":"wall","center":[{x},80],"yaw":0,"half_extents":[10,0.5,3]}}"#)
    };
    common::flat(
        [120.0, 120.0],
        &format!(
            r#","props":[{},{},{},{}]"#,
            wall(45.0 - gap / 2.0 - 4.7),
            wall(45.0 + gap / 2.0 + 4.7),
            row(30.0),
            row(50.0)
        ),
    )
}

const BEFORE: V3 = v3(45.0, 30.0, 1.6);
const BEYOND: V3 = v3(45.0, 70.0, 1.0);

#[test]
fn no_one_is_seen_through_a_crack_between_two_houses() {
    assert!(!houses(0.6).sight_clear(BEFORE, BEYOND));
    assert!(!houses(0.6).sight_clear(BEYOND, BEFORE));
}

#[test]
fn a_gap_wider_than_the_least_is_seen_through() {
    assert!(houses(1.5).sight_clear(BEFORE, BEYOND));
}

#[test]
fn a_soldier_standing_in_the_crack_sees_out_of_it() {
    let inside = v3(45.0, 50.0, 1.6);
    assert!(houses(0.6).sight_clear(inside, BEYOND));
    assert!(houses(0.6).sight_clear(inside, BEFORE));
}

/// The row along y = 80 is two walls end to end, with no gap between them: a
/// soldier pressed against its facade far along is seen from the street.
#[test]
fn a_soldier_against_a_facade_down_the_street_is_still_seen() {
    let observer = v3(22.0, 78.3, 1.6);
    let hugging = v3(55.0, 79.3, 1.0);
    assert!(houses(0.6).sight_clear(observer, hugging));
}

#[test]
fn a_crack_opens_when_either_house_is_gone() {
    let mut world = houses(0.6);
    world.remove_prop(1);
    assert!(world.sight_clear(BEFORE, BEYOND));
}
