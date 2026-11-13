//! The smoke cloud law and its depth along a sight line
//! (`specs/smoke-screen`, slice 01). Every number comes from the spec built
//! here, never from authored content.
use contract::smoke::{
    circle_chord, concealment_at, piece_depth, segment_depth, state, CloudState, SmokeCloud,
    SmokeCloudSpec,
};

const HZ: u32 = 30;

fn spec() -> SmokeCloudSpec {
    SmokeCloudSpec {
        radius_m: 15.0,
        height_m: 6.0,
        life_s: 40.0,
        bloom_s: 1.0,
        fade_s: 10.0,
        bloom_steps: 4,
        fade_steps: 5,
        depth_per_m: 0.2,
        concealment: 0.1,
        look: "discharger".into(),
    }
}

fn cloud(at: [f64; 2]) -> SmokeCloud {
    SmokeCloud {
        id: 1,
        centre: [at[0], at[1], 0.0],
        popped: 100,
        spec: spec(),
    }
}

fn full(at: [f64; 2]) -> CloudState {
    let c = cloud(at);
    let mid = c.popped + (c.spec.life_s / 2.0 * HZ as f64) as u64;
    state(&c, mid, HZ).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_chord_is_the_length_of_the_line_inside_the_circle() {
    let (c, r) = ([0.0, 0.0], 10.0);
    // Through the centre: the whole diameter.
    assert!(close(
        circle_chord(c, r, [-20.0, 0.0], [20.0, 0.0]),
        2.0 * r
    ));
    // A line that misses, and one that only touches.
    assert_eq!(circle_chord(c, r, [-20.0, 11.0], [20.0, 11.0]), 0.0);
    assert!(circle_chord(c, r, [-20.0, r], [20.0, r]) < 1e-6);
    // Both ends inside: the whole segment.
    assert!(close(circle_chord(c, r, [-3.0, 1.0], [4.0, 1.0]), 7.0));
    // One end inside: from that end to the rim.
    assert!(close(circle_chord(c, r, [0.0, 0.0], [20.0, 0.0]), r));
    // A point has no length.
    assert_eq!(circle_chord(c, r, [1.0, 1.0], [1.0, 1.0]), 0.0);
    // Off-centre: 2·sqrt(r² − d²).
    let d: f64 = 6.0;
    assert!(close(
        circle_chord(c, r, [-20.0, d], [20.0, d]),
        2.0 * (r * r - d * d).sqrt()
    ));
}

#[test]
fn a_cloud_blooms_holds_and_fades_in_steps_then_is_gone() {
    let c = cloud([0.0, 0.0]);
    let s = &c.spec;
    let end = c.popped + (s.life_s * HZ as f64) as u64;
    assert!(
        state(&c, c.popped - 1, HZ).is_none(),
        "nothing before the pop"
    );
    let mut seen = Vec::new();
    let mut last: Option<CloudState> = None;
    for tick in c.popped..end {
        let now = state(&c, tick, HZ).expect("alive for its whole life");
        if let Some(prev) = &last {
            assert!(now.phase >= prev.phase, "phases only move forward");
            if now.phase == prev.phase {
                assert_eq!(&now, prev, "a phase is one state");
            }
        }
        if !seen.contains(&now.phase) {
            seen.push(now.phase);
        }
        last = Some(now);
    }
    assert!(state(&c, end, HZ).is_none(), "gone after its life");
    assert!(seen.len() <= (s.bloom_steps + s.fade_steps + 1) as usize);

    let first = state(&c, c.popped, HZ).unwrap();
    assert!(
        first.radius_m > 0.0 && first.radius_m < s.radius_m,
        "it blooms"
    );
    let bloomed = c.popped + (s.bloom_s * HZ as f64) as u64;
    let held = state(&c, bloomed, HZ).unwrap();
    assert!(close(held.radius_m, s.radius_m));
    assert!(close(held.depth_per_m, s.depth_per_m));
    assert!(close(held.concealment, s.concealment));
    let fading = state(&c, end - 1, HZ).unwrap();
    assert!(
        close(fading.radius_m, s.radius_m),
        "fading thins, it doesn't shrink"
    );
    assert!(fading.depth_per_m < held.depth_per_m && fading.depth_per_m > 0.0);
    assert!(fading.concealment > held.concealment && fading.concealment < 1.0);
}

#[test]
fn smoke_counts_only_below_the_cloud_top() {
    let s = full([0.0, 0.0]);
    let (a, b) = ([-20.0, 0.0], [20.0, 0.0]);
    let below = piece_depth(std::slice::from_ref(&s), a, b, s.height_m - 0.5, 0.0);
    assert!(close(below, 2.0 * s.radius_m * s.depth_per_m));
    assert_eq!(
        piece_depth(std::slice::from_ref(&s), a, b, s.height_m + 0.5, 0.0),
        0.0
    );
    // The top is measured from the ground under the piece.
    assert_eq!(piece_depth(std::slice::from_ref(&s), a, b, 7.0, 0.0), 0.0);
    assert!(piece_depth(&[s], a, b, 7.0, 3.0) > 0.0);
}

#[test]
fn overlapping_clouds_add() {
    let one = full([0.0, 0.0]);
    let two = full([5.0, 0.0]);
    let (a, b) = ([-30.0, 0.0], [30.0, 0.0]);
    let each = piece_depth(std::slice::from_ref(&one), a, b, 1.0, 0.0)
        + piece_depth(std::slice::from_ref(&two), a, b, 1.0, 0.0);
    assert!(close(piece_depth(&[one, two], a, b, 1.0, 0.0), each));
}

#[test]
fn a_sight_line_sums_its_pieces_over_the_ground() {
    let s = full([0.0, 0.0]);
    let flat = |_: f64, _: f64| Some(0.0);
    // Level at eye height through the middle: the exact diameter.
    let through = segment_depth(
        std::slice::from_ref(&s),
        [-40.0, 0.0, 2.0],
        [40.0, 0.0, 2.0],
        flat,
    );
    assert!(close(through, 2.0 * s.radius_m * s.depth_per_m));
    // A line that climbs out of the top counts only its low part.
    let climbing = segment_depth(
        std::slice::from_ref(&s),
        [-40.0, 0.0, 2.0],
        [40.0, 0.0, 12.0],
        flat,
    );
    assert!(climbing > 0.0 && climbing < through);
    // No cloud, no depth; a line beside the cloud, no depth.
    assert_eq!(
        segment_depth(&[], [-40.0, 0.0, 2.0], [40.0, 0.0, 2.0], flat),
        0.0
    );
    assert_eq!(
        segment_depth(&[s], [-40.0, 30.0, 2.0], [40.0, 30.0, 2.0], flat),
        0.0
    );
}

#[test]
fn standing_in_smoke_conceals_and_the_strongest_cloud_wins() {
    let a = full([0.0, 0.0]);
    let mut thin = full([4.0, 0.0]);
    thin.concealment = 0.6;
    assert_eq!(concealment_at(&[a.clone(), thin.clone()], 50.0, 0.0), 1.0);
    assert!(close(concealment_at(&[thin.clone()], 4.0, 0.0), 0.6));
    assert!(close(
        concealment_at(&[a.clone(), thin], 4.0, 0.0),
        a.concealment
    ));
}

#[test]
fn a_spec_refuses_what_cannot_be_a_cloud() {
    assert!(spec().validate().is_ok());
    let mut s = spec();
    s.radius_m = 0.0;
    assert!(s.validate().is_err());
    let mut s = spec();
    s.life_s = s.bloom_s + s.fade_s - 1.0;
    assert!(s.validate().is_err(), "bloom and fade must fit in its life");
    let mut s = spec();
    s.concealment = 1.0;
    assert!(s.validate().is_err(), "smoke that conceals nothing");
    let mut s = spec();
    s.fade_steps = 0;
    assert!(s.validate().is_err());
}
