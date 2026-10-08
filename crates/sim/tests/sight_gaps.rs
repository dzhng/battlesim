//! A line of sight squeezing between bodies that hide what lies behind them,
//! one on each side, needs `sensors.min_sight_gap_m` (half a metre) of room
//! on its two sides together: no squad is spotted through the crack between
//! two houses, nor through the sliver between a near house on one side and a
//! far one on the other. A line running along one row of them is not
//! squeezed, and one that starts or ends at a wall or in a crack sees out.
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
    assert!(!houses(0.3).sight_clear(BEFORE, BEYOND));
    assert!(!houses(0.3).sight_clear(BEYOND, BEFORE));
}

#[test]
fn a_gap_wider_than_the_least_is_seen_through() {
    assert!(houses(0.8).sight_clear(BEFORE, BEYOND));
}

#[test]
fn a_soldier_standing_in_the_crack_sees_out_of_it() {
    let inside = v3(45.0, 50.0, 1.6);
    assert!(houses(0.3).sight_clear(inside, BEYOND));
    assert!(houses(0.3).sight_clear(inside, BEFORE));
}

/// The row along y = 80 is two walls end to end, with no gap between them: a
/// soldier pressed against its facade far along is seen from the street.
#[test]
fn a_soldier_against_a_facade_down_the_street_is_still_seen() {
    let observer = v3(22.0, 78.3, 1.6);
    let hugging = v3(55.0, 79.3, 1.0);
    assert!(houses(0.3).sight_clear(observer, hugging));
}

#[test]
fn a_crack_opens_when_either_house_is_gone() {
    let mut world = houses(0.3);
    world.remove_prop(1);
    assert!(world.sight_clear(BEFORE, BEYOND));
}

/// The moment: one house in front and to the left, another far behind and to
/// the right, their corners all but meeting as the eye sees them. The houses
/// stand 30 m apart, but the line between them squeezes past each corner by
/// a hand's width: no one is seen through that sliver.
#[test]
fn no_one_is_seen_through_the_sliver_between_a_near_house_and_a_far_one() {
    let world = common::flat(
        [160.0, 160.0],
        r#","props":[
            {"kind":"wall","center":[39.85,45],"yaw":0,"half_extents":[5,5,3]},
            {"kind":"wall","center":[50.15,80],"yaw":0,"half_extents":[5,5,3]}
        ]"#,
    );
    // Straight up x = 45: 0.15 m past the near house's right face, 0.15 m
    // past the far house's left face.
    assert!(!world.sight_clear(v3(45.0, 20.0, 1.6), v3(45.0, 120.0, 1.0)));
    // Room of 0.3 m on each side is a view.
    let wide = common::flat(
        [160.0, 160.0],
        r#","props":[
            {"kind":"wall","center":[39.7,45],"yaw":0,"half_extents":[5,5,3]},
            {"kind":"wall","center":[50.3,80],"yaw":0,"half_extents":[5,5,3]}
        ]"#,
    );
    assert!(wide.sight_clear(v3(45.0, 20.0, 1.6), v3(45.0, 120.0, 1.0)));
}
