use super::felled::fell_pose;
use super::{FelledRules, Pop, Record, Size, FLOATS};
use crate::view::{Camera, View};
use std::collections::HashMap;

/// A row of trees along +x, twelve in the first chunk and one in each
/// chunk past it, and every list's records by print over cameras that
/// walk along it: one print must always mean one list.
#[test]
fn a_list_named_by_the_same_print_holds_the_same_records() {
    let mut placed = Vec::new();
    let mut tree = |x: f32| {
        let mut p = [0.0f32; FLOATS];
        (p[0], p[3], p[4], p[5]) = (x, 1.0, 1.0, 1.0);
        placed.extend_from_slice(&p);
    };
    for i in 0..12 {
        tree(4.0 + 10.0 * i as f32);
    }
    for c in 1..12 {
        tree(128.0 * c as f32 + 64.0);
    }
    let sizes = [Size {
        height: 10.0,
        radius: 2.0,
    }];
    let mut pop = Pop::new(&placed, &sizes, 1, false, 0);
    let mut seen: HashMap<(u32, u32, u64), Vec<Record>> = HashMap::new();
    for step in 0..400 {
        let x = -200.0 + step as f64 * 4.0;
        let view = View::new(&Camera {
            eye: [x, -30.0, 6.0],
            target: [x + 100.0, 0.0, 0.0],
            up: [0.0, 0.0, 1.0],
            fov_y: 0.8,
            near: 0.5,
            size: [1280, 720],
        });
        pop.select(&view, None, &[150.0, 60.0, 24.0]);
        pop.each_list(&[0], &mut |l| {
            if l.records.is_empty() {
                return; // an empty list is never sent
            }
            let key = (l.tier, l.pass, l.print);
            let earlier = seen.entry(key).or_insert(l.records.to_vec());
            assert_eq!(
                earlier, l.records,
                "tier {} at eye x {x}: one print, two lists",
                l.tier
            );
        });
    }
}

fn rules() -> FelledRules {
    serde_json::from_value(serde_json::json!({
        "fall_s": 2, "settle_s": 0.5, "settle_deg": 5, "rest_height_m": 1,
        "rest_spread": 0.6, "stump_height_m": 0.4, "cut": [0.4, 0.3, 0.2]
    }))
    .unwrap()
}

/// A tree 11 m tall, its crown 4.2 m round from 4 m up, its trunk 0.45 m.
fn pose(age: f64) -> [f64; 7] {
    fell_pose(&rules(), age, 11.0, 4.2, 0.45, 4.0)
}

#[test]
fn a_tree_tips_over_gathering_speed_strikes_at_fall_s_rocks_and_lies() {
    let r = rules();
    let rest = pose(1000.0)[0];
    assert!(rest > std::f64::consts::FRAC_PI_2);
    assert_eq!([pose(-1.0)[0], pose(0.0)[0]], [0.0, 0.0]);
    let (mut last, mut step) = (0.0, 0.0);
    for k in 1..=20 {
        let angle = pose(k as f64 / 20.0 * r.fall_s)[0];
        assert!(angle - last > step - 1e-12, "it never slows while it falls");
        (step, last) = (angle - last, angle);
    }
    assert!((pose(r.fall_s)[0] - rest).abs() < 1e-12);
    let rock = r.settle_deg.to_radians();
    for k in 0..50 {
        let angle = pose(r.fall_s + k as f64 / 50.0 * r.settle_s)[0];
        assert!(
            angle <= rest && angle > rest - rock - 1e-9,
            "it rocks back no more than settle_deg"
        );
    }
    assert_eq!(pose(r.fall_s + r.settle_s)[0], rest);
}

#[test]
fn a_tree_lands_in_its_second_half_smoothly_and_lies_pressed_and_narrowed() {
    let r = rules();
    let landed = |age: f64| pose(age)[5];
    assert_eq!(landed(0.0), 0.0);
    // Half way over it has not begun to land.
    assert_eq!(landed(r.fall_s * 0.5f64.sqrt() * 0.99), 0.0);
    let mut last = 0.0;
    for k in 0..=40 {
        let l = landed(k as f64 / 40.0 * r.fall_s);
        assert!(l >= last && l - last < 0.2, "it lands smoothly");
        last = l;
    }
    let [_, hinge, squash, spread, trunk, landed, crown] = pose(1000.0);
    assert_eq!(
        [hinge, trunk, landed, crown],
        [r.stump_height_m, 0.45, 1.0, 4.0]
    );
    // Lying, its crown narrows to rest_spread, and is pressed so that its
    // farthest leaf above its axis keeps under rest_height_m.
    assert!((spread - r.rest_spread).abs() < 1e-12);
    let axis = trunk * (1.0 - (crown - hinge) / (11.0 - hinge));
    assert!(axis + squash * 4.2 <= r.rest_height_m + 1e-9);
    // Upright, nothing is pressed.
    let upright = pose(0.0);
    assert_eq!([upright[2], upright[3]], [1.0, 1.0]);
}
