//! The wave field's clock: it steps at a fixed rate on the presentation
//! clock whatever the frame rate, drops what a jump would pile up, opens a
//! new epoch (a cleared field) on a reset, a load or a side switch, follows
//! the camera in whole texels, and splats each wake impulse once, at the
//! step that covers it.
use presentation::effects::frame::EffectFrame;
use presentation::effects::publication::EffectPublication;
use presentation::effects::tables::EffectPresentation;
use presentation::view::Camera;
use presentation::wave::{area, WaveClock, WAVE};
use serde_json::{json, Value};

const HZ: f64 = 30.0;

/// A camera looking down at (`x`, `y`) from `distance` m.
fn looking_at(x: f64, y: f64, distance: f64) -> Camera {
    Camera {
        eye: [x, y - distance * 0.6, distance * 0.8],
        target: [x, y, 0.0],
        up: [0.0, 0.0, 1.0],
        fov_y: 0.8,
        near: 0.5,
        size: [1280, 720],
    }
}

fn effects() -> EffectFrame {
    let at = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/effect-vectors.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(at).unwrap()).unwrap();
    EffectFrame::new(
        HZ,
        EffectPresentation::from_value(&v["presentation"]).unwrap(),
    )
}

/// Tick `tick`'s publication: one squad of two standing in water at (40, 20).
fn wading(tick: f64) -> EffectPublication {
    serde_json::from_value(
        json!({"tick": tick, "segments": [], "blasts": [], "smokes": [],
        "shooters": [{"key": 5.0, "position": [40.0, 20.0, 0.0], "half": null, "yaw": 0.0,
            "members": [7.0, 8.0], "mounts": [],
            "wading": [{ "at": [40.0, 19.0, 0.0], "depth_m": 0.6 },
                       { "at": [40.0, 21.0, 0.0], "depth_m": 0.6 }]}]}),
    )
    .unwrap()
}

#[test]
fn the_field_steps_the_same_number_of_times_at_any_frame_rate() {
    let steps = |fps: f64| {
        let mut clock = WaveClock::new();
        let camera = looking_at(100.0, 100.0, 60.0);
        (0..=(3.0 * fps) as u32)
            .map(|k| clock.advance(f64::from(k) / fps, &camera).steps)
            .sum::<u32>()
    };
    let want = (3.0 * WAVE.hz).round() as u32;
    for fps in [24.0, 60.0, 144.0, 7.0] {
        assert_eq!(steps(fps), want, "at {fps} frames a second");
    }
    // A jump of the clock steps no more than the cap: the excess is dropped.
    let mut clock = WaveClock::new();
    let camera = looking_at(0.0, 0.0, 60.0);
    clock.advance(1.0, &camera);
    assert_eq!(clock.advance(11.0, &camera).steps, WAVE.max_steps);
    assert_eq!(clock.advance(11.0, &camera).steps, 0, "paused, it holds");
}

#[test]
fn a_reset_a_side_switch_or_a_load_changes_the_epoch() {
    let camera = looking_at(40.0, 20.0, 60.0);
    let mut f = effects();
    f.note(&wading(1.0));
    let first = f.waves(0.0, &camera).step.epoch;
    assert_eq!(
        f.waves(0.5, &camera).step.epoch,
        first,
        "an epoch holds while it runs"
    );
    // A side switch or a load is a new presentation of the world.
    assert_ne!(effects().waves(0.5, &camera).step.epoch, first);
    // An earlier tick is a new battle; so is the clock running backwards.
    f.note(&wading(9.0));
    let held = f.waves(1.0, &camera).step.epoch;
    f.note(&wading(2.0));
    let reset = f.waves(1.1, &camera).step.epoch;
    assert_ne!(reset, held, "a reset");
    assert_ne!(
        f.waves(0.2, &camera).step.epoch,
        reset,
        "the clock ran back"
    );
}

#[test]
fn the_area_moves_in_whole_texels() {
    let mut clock = WaveClock::new();
    let start = area(&looking_at(100.0, 50.0, 60.0))[0];
    let mut corner: Option<[f32; 2]> = None;
    for k in 0..40 {
        let x = 100.0 + 0.37 * f64::from(k);
        let camera = looking_at(x, 50.0, 60.0);
        let step = clock.advance(f64::from(k) / 30.0, &camera);
        let [cx, cy, side, _] = area(&camera);
        let texel = WAVE.texel_m as f32;
        // Its corner lies on the texel grid, and the field holds the target.
        assert_eq!((cx / texel).fract(), 0.0, "corner x {cx}");
        assert_eq!((cy / texel).fract(), 0.0, "corner y {cy}");
        assert!(cx < x as f32 && x as f32 <= cx + side);
        if let Some([px, py]) = corner {
            assert_eq!(
                step.shift,
                [((cx - px) / texel) as i32, ((cy - py) / texel) as i32]
            );
        }
        corner = Some([cx, cy]);
    }
    let moved = corner.unwrap()[0] - start;
    assert!(
        (14.0..15.0).contains(&moved),
        "it followed the camera {moved} m"
    );
}

#[test]
fn each_wake_impulse_splats_once_at_the_step_that_covers_it_whatever_the_frame_rate() {
    let camera = looking_at(40.0, 20.0, 60.0);
    let splats = |fps: f64| {
        let mut f = effects();
        let mut made = vec![];
        let mut tick = 0.0;
        for k in 0..=(2.0 * fps) as u32 {
            let clock = f64::from(k) / fps;
            while tick / HZ <= clock {
                tick += 1.0;
                f.note(&wading(tick));
            }
            let w = f.waves(clock, &camera);
            assert!(w.splats.iter().all(|s| (s.step as u32) < w.step.steps));
            made.extend(w.splats.iter().map(|s| s.splat));
        }
        made
    };
    let slow = splats(24.0);
    assert!(!slow.is_empty(), "the still squad rings the water");
    assert_eq!(slow, splats(144.0));
}

#[test]
fn moving_wakes_follow_travel_instead_of_the_units_facing() {
    let mut f = effects();
    let camera = looking_at(40.0, 20.0, 60.0);
    f.waves(0.0, &camera);
    let mut splats = vec![];
    for tick in 1..=60 {
        let mut p = wading(f64::from(tick));
        let d = [0.06 * f64::from(tick), 0.08 * f64::from(tick)];
        let s = &mut p.shooters[0];
        s.position[0] += d[0];
        s.position[1] += d[1];
        for w in &mut s.wading {
            w.at[0] += d[0];
            w.at[1] += d[1];
        }
        // Its yaw stays east while its displacement travels north-east.
        f.note(&p);
        splats.extend(f.waves(f64::from(tick) / HZ, &camera).splats);
    }
    let moving: Vec<_> = splats.iter().filter(|s| s.kind == 1.0).collect();
    assert!(!moving.is_empty(), "the moving squad disturbed the water");
    for s in moving {
        assert!((s.heading[0] - 0.6).abs() < 1e-5, "{s:?}");
        assert!((s.heading[1] - 0.8).abs() < 1e-5, "{s:?}");
    }
}

/// A side switch (`Presentation::invalidate`, which starts the field over)
/// discards the wakes the old side saw made and not yet splatted: none
/// reaches the new side's field.
#[test]
fn a_side_switch_splats_none_of_the_old_sides_wakes() {
    let camera = looking_at(40.0, 20.0, 60.0);
    let mut f = effects();
    f.waves(0.0, &camera);
    for tick in 1..=16 {
        f.note(&wading(f64::from(tick)));
    }
    // Drawn up to 0.49 s: the squad's 0.5 s rings are queued, not yet splatted.
    assert!(f.wakes(0.49, 0.6).count() > 0, "a wake waits");
    f.waves(0.49, &camera);
    f.restart_waves();
    let cleared = f.waves(0.5, &camera);
    let next = f.waves(0.7, &camera);
    assert_eq!(next.step.epoch, cleared.step.epoch);
    assert!(next.step.steps > 0);
    assert!(
        next.splats.is_empty(),
        "the old side's wakes: {:?}",
        next.splats
    );
}
