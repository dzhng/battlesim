//! What a player sees jets' engines and wings leave behind: a glow in every
//! running nozzle, a flame only from an engine that has an afterburner and
//! only once the jet flies past its cruise, wingtip vapour only while the
//! wings pull hard (a tight turn, a pull-up; never level flight, however
//! fast), forming without flicker and thinning over its release, and trails
//! laid where the publications put the jet: the same
//! whatever the frame rate, whole in a replay framed mid-flight, held on a
//! paused clock, cleared on a rewind, broken where a jet jumps, fading out
//! after a jet is gone or downed, and never holding more than their life.
//! Driven by synthetic publications and drawn jets over a test-owned style.
mod jets;
use jets::*;

use presentation::effects::exhaust::Emitter;
use presentation::effects::frame::EffectFrame;

#[test]
fn a_trail_is_the_same_whatever_the_frame_rate() {
    let slow = fly(&mut frame(), 24.0, 1.5, circling, circling);
    let fast = fly(&mut frame(), 144.0, 1.5, circling, circling);
    for albedo in [ENGINE_ALBEDO, TIP_ALBEDO] {
        let (a, b) = (ribbons(&slow, albedo), ribbons(&fast, albedo));
        assert!(a.len() > 20, "a trail of samples behind the jet");
        assert_eq!(a.len(), b.len(), "as many stretches at 24 fps as at 144");
        for (x, y) in a.iter().zip(&b) {
            for k in 0..8 {
                assert!(
                    (x[k] - y[k]).abs() < 1e-3,
                    "same place and width: {x:?} vs {y:?}"
                );
            }
            assert!((x[11] - y[11]).abs() < 1e-4, "same opacity");
        }
    }
}

#[test]
fn a_replay_framed_mid_flight_shows_the_whole_trail() {
    let mut f = frame();
    let flight = circling;
    for tick in 0..=45 {
        f.note(&publ(tick, &flight(f64::from(tick) / TICK_HZ)));
    }
    let seek = draw(&mut f, 1.5, &flight(1.5));
    let flown = fly(&mut frame(), 60.0, 1.5, flight, flight);
    for albedo in [ENGINE_ALBEDO, TIP_ALBEDO] {
        assert!(
            ribbons(&seek, albedo).len() > 20,
            "the trail is there on the first frame"
        );
        assert_eq!(ribbons(&seek, albedo).len(), ribbons(&flown, albedo).len());
    }
}

#[test]
fn a_paused_clock_holds_the_trails_and_a_rewind_clears_them() {
    let mut f = frame();
    let flight = circling;
    let before = fly(&mut f, 60.0, 1.0, flight, flight);
    assert!(
        !ribbons(&before, TIP_ALBEDO).is_empty(),
        "turning, it vapours"
    );
    let held = draw(&mut f, 1.0, &flight(1.0));
    assert_eq!(before, held, "a held clock draws the same frame");
    f.note(&publ(6, &flight(0.2)));
    let rewound = draw(&mut f, 0.2, &flight(0.2));
    assert!(
        ribbons(&rewound, ENGINE_ALBEDO).is_empty(),
        "a rewind starts every trail over"
    );
    assert!(ribbons(&rewound, TIP_ALBEDO).is_empty());
    assert_eq!(glows(&rewound).0, 2, "its nozzles still glow");
}

#[test]
fn a_jet_that_jumps_breaks_its_trail_instead_of_streaking_across() {
    let jumping = |t: f64| {
        let x = CRUISE * t + if t >= 1.0 { 5000.0 } else { 0.0 };
        vec![craft(2.0, "burner", [x, 0.0, 900.0], 0.0)]
    };
    let d = fly(&mut frame(), 60.0, 1.5, jumping, jumping);
    let longest = [ENGINE_ALBEDO, TIP_ALBEDO]
        .iter()
        .flat_map(|&a| ribbons(&d, a))
        .map(|r| span(r))
        .fold(0.0, f64::max);
    assert!(
        longest < 2.0 * CRUISE / TICK_HZ,
        "no stretch spans the jump: {longest} m"
    );
}

#[test]
fn a_jet_gone_stops_glowing_at_once_and_its_trail_fades_out() {
    let mut f = frame();
    let flight = straight(2.0, "burner", CRUISE);
    let until_one = |t: f64| if t <= 1.0 { flight(t) } else { vec![] };
    fly(&mut f, 60.0, 1.0, until_one, until_one);
    let gone = fly(&mut f, 60.0, 1.0 + 1.0 / 60.0, until_one, until_one);
    assert_eq!(glows(&gone).0, 0, "no glow without a running jet");
    assert_eq!(flames(&gone), 0);
    assert!(
        !ribbons(&gone, ENGINE_ALBEDO).is_empty(),
        "its trail stays a while"
    );
    f.note(&publ(31 + (LIFE * TICK_HZ) as u32 + 2, &[]));
    assert!(
        draw(&mut f, 1.0 + LIFE + 0.1, &[]).is_empty(),
        "then fades out entirely"
    );
    assert_eq!(f.jet_samples(), 0, "and holds nothing");
}

#[test]
fn a_downed_jet_still_falling_lays_no_more_trail() {
    // Published all the way down, drawn running only until it is hit.
    let mut f = frame();
    let flight = straight(2.0, "burner", CRUISE);
    let running = |t: f64| if t < 1.0 { flight(t) } else { vec![] };
    let late = fly(&mut f, 60.0, 1.0 + MAX_GAP_S + LIFE + 0.1, &flight, running);
    assert!(
        late.is_empty(),
        "nothing laid after its engines went out survives: {}",
        late.len()
    );
}

#[test]
fn trails_hold_no_more_than_their_life_whatever_the_flight_or_the_crowd() {
    let crowd = |t: f64| -> Vec<Craft> {
        (0..40u32)
            .map(|k| {
                let mut c = circling(t + f64::from(k) * 3.0)[0];
                c.key = f64::from(2 * k);
                c.at[2] += f64::from(k) * 50.0;
                c
            })
            .collect()
    };
    let mut f = frame();
    let per_tick_life = (LIFE * TICK_HZ + 2.0) as usize;
    let bound = MAX_AIRCRAFT * 5 * per_tick_life;
    let mut most = 0;
    let mut noted = 0;
    for i in 0..=(30 * 60) {
        let t = f64::from(i) / 60.0;
        while f64::from(noted) / TICK_HZ <= t {
            f.note(&publ(noted, &crowd(f64::from(noted) / TICK_HZ)));
            noted += 1;
        }
        draw(&mut f, t, &crowd(t));
        most = most.max(f.jet_samples());
    }
    assert!(
        most <= bound,
        "at most max_aircraft tracks of a life each: {most} > {bound}"
    );
}

#[test]
fn only_an_afterburning_engine_burns_and_only_past_its_cruise() {
    let flames_at = |engine: &'static str, speed: f64| {
        let flight = straight(2.0, engine, speed);
        flames(&fly(&mut frame(), 60.0, 2.0, &flight, &flight))
    };
    assert_eq!(flames_at("burner", CRUISE), 0, "no flame at cruise");
    assert!(flames_at("burner", 1.5 * CRUISE) > 0, "a flame past it");
    assert_eq!(flames_at("fan", 1.5 * CRUISE), 0, "a turbofan never burns");
    let glow_at = |speed: f64| {
        let flight = straight(2.0, "burner", speed);
        glows(&fly(&mut frame(), 60.0, 2.0, &flight, &flight)).1
    };
    let (slow, cruise, fast) = (
        glow_at(0.5 * CRUISE),
        glow_at(CRUISE),
        glow_at(1.5 * CRUISE),
    );
    assert!(
        slow < cruise && cruise < fast,
        "brighter the faster: {slow} {cruise} {fast}"
    );
}

#[test]
fn level_flight_leaves_no_wingtip_vapour_however_fast_or_hard_it_accelerates() {
    for speed in [0.5 * CRUISE, CRUISE, 1.5 * CRUISE] {
        let flight = straight(2.0, "burner", speed);
        let d = fly(&mut frame(), 60.0, 2.0, &flight, &flight);
        assert!(vapour(&d).is_empty(), "level at {speed} m/s");
        assert!(
            !ribbons(&d, ENGINE_ALBEDO).is_empty(),
            "its engines still trail"
        );
    }
    // Straight on, its afterburner shoving it from cruise to half again.
    let shove = along(|t: f64| [CRUISE * t + 0.25 * CRUISE * t * t, 0.0, 900.0]);
    let d = fly(&mut frame(), 60.0, 2.0, &shove, &shove);
    assert!(flames(&d) > 0, "it burns");
    assert!(
        vapour(&d).is_empty(),
        "but its wings carry no more than its weight"
    );
}

#[test]
fn a_hard_turn_blooms_wingtip_vapour_and_a_gentle_one_does_not() {
    let at = |load: f64| {
        let flight = along(turning(CRUISE, move |_| rate_for(CRUISE, load)));
        thickest(&fly(&mut frame(), 60.0, 2.0, &flight, &flight))
    };
    let (gentle, hard, harder) = (at(1.3), at(2.5), at(3.5));
    assert_eq!(gentle, 0.0, "a gentle turn leaves none");
    assert!(
        hard > 0.0 && harder > hard,
        "thicker the harder: {hard} {harder}"
    );
}

#[test]
fn the_same_turn_flown_faster_pulls_more_vapour() {
    // One rate of turn: the faster jet's wings carry more.
    let rate = rate_for(CRUISE, 2.2);
    let at = |speed: f64| {
        let flight = along(turning(speed, move |_| rate));
        thickest(&fly(&mut frame(), 60.0, 2.0, &flight, &flight))
    };
    let (slow, fast) = (at(0.75 * CRUISE), at(1.3 * CRUISE));
    assert!(fast > slow, "slow {slow}, fast {fast}");
}

#[test]
fn a_pull_up_out_of_level_flight_blooms_vapour_too() {
    // Level, then a 1500 m vertical arc upward at cruise: about 3.7 g.
    let r = 1500.0;
    let pull = along(move |t: f64| {
        let a = (CRUISE * (t - 0.6) / r).max(0.0);
        let level = CRUISE * t.min(0.6);
        [level + r * a.sin(), 0.0, 900.0 + r * (1.0 - a.cos())]
    });
    let level = fly(&mut frame(), 60.0, 0.55, &pull, &pull);
    assert!(vapour(&level).is_empty(), "none before the pull");
    let pulled = fly(&mut frame(), 60.0, 1.6, &pull, &pull);
    assert!(thickest(&pulled) > 0.0, "it forms in the pull");
}

#[test]
fn vapour_does_not_flicker_while_the_load_hovers_round_where_it_forms() {
    // Into a hard turn, then a load wavering slowly between 1.6 and 2.0:
    // above where it stops, and back and forth across where it forms;
    // beside the same turn held at 1.8.
    let laid = |waver: f64| {
        let rate = move |t: f64| {
            let load = if t < 1.0 {
                2.6
            } else {
                1.8 + waver * (t * 3.0).cos()
            };
            rate_for(CRUISE, load)
        };
        let flight = along(turning(CRUISE, rate));
        vapour(&fly(&mut frame(), 60.0, 3.8, &flight, &flight))
    };
    let (wavering, held) = (laid(0.2), laid(0.0));
    assert_eq!(wavering.len(), held.len());
    // Laid while it wavered: each stretch against the same age's held one.
    let n = wavering.len() / 2;
    for k in n - 40..n - 4 {
        let share = wavering[k] / held[k];
        assert!(
            (0.75..1.5).contains(&share),
            "steady, not pulsing with the load: {share} at {k}"
        );
    }
}

#[test]
fn vapour_thins_out_over_its_release_after_the_pull_ends_whatever_the_frame_rate() {
    // A hard turn for a second and a half, then level again.
    let rate = |t: f64| if t < 1.5 { rate_for(CRUISE, 3.0) } else { 0.0 };
    let flight = along(turning(CRUISE, rate));
    // The thickest vapour within 30 m of the jet: what it lays now.
    let head = |fps: f64, until: f64| {
        let d = fly(&mut frame(), fps, until, &flight, &flight);
        let at = flight(until)[0].at;
        ribbons(&d, TIP_ALBEDO)
            .iter()
            .filter(|r| {
                let d: f64 = (0..3).map(|k| (f64::from(r[4 + k]) - at[k]).powi(2)).sum();
                d.sqrt() < 30.0
            })
            .map(|r| r[11])
            .fold(0.0, f32::max)
    };
    let pulling = head(60.0, 1.5);
    let soon = head(60.0, 1.5 + RELEASE_S / 3.0);
    // Its load is read over half a second of flight, so it lets go that late.
    let later = head(60.0, 1.5 + 0.5 + RELEASE_S);
    assert!(pulling > 0.0);
    assert!(
        soon > 0.0 && soon < pulling,
        "it thins, not stops: {pulling} {soon}"
    );
    assert!(
        later < 0.1 * pulling,
        "and is all but gone after its release: {pulling} {later}"
    );
    let slow = fly(&mut frame(), 24.0, 2.5, &flight, &flight);
    let fast = fly(&mut frame(), 144.0, 2.5, &flight, &flight);
    let (a, b) = (vapour(&slow), vapour(&fast));
    assert_eq!(a.len(), b.len(), "as many stretches at 24 fps as at 144");
    for (x, y) in a.iter().zip(&b) {
        assert!((x - y).abs() < 1e-4, "the same release: {x} vs {y}");
    }
}

#[test]
fn a_trail_widens_and_fades_toward_its_end() {
    let flight = straight(2.0, "burner", CRUISE);
    let d = fly(&mut frame(), 60.0, 1.5, &flight, &flight);
    let r = ribbons(&d, ENGINE_ALBEDO);
    // One nozzle's trail, oldest first: wider and fainter far behind than
    // near the jet, past the near end's fade-in.
    let half = r.len() / 2;
    let (old, young) = (r[0], r[half - 4]);
    assert!(old[3] > young[3], "wider far behind");
    assert!(old[11] < young[11], "fainter far behind");
}

/// Each frame's wingtip vapour at the jet (the thickest stretch ending
/// within a metre of a wingtip), over a hard turn between level flight,
/// drawn half a tick past each tick so a stretch reaches every wingtip.
fn vapour_at_the_wingtips(mut f: EffectFrame) -> Vec<(f64, f32)> {
    let rate = |t: f64| {
        if (1.0..2.5).contains(&t) {
            rate_for(CRUISE, 3.0)
        } else {
            0.0
        }
    };
    let flight = along(turning(CRUISE, rate));
    let mut noted = 0u32;
    let mut out = Vec::new();
    for k in 0..(5.0 * TICK_HZ) as u32 {
        let t = (f64::from(k) + 0.5) / TICK_HZ;
        while f64::from(noted) / TICK_HZ <= t + 1e-9 {
            f.note(&publ(noted, &flight(f64::from(noted) / TICK_HZ)));
            noted += 1;
        }
        let jets = flight(t);
        let d = draw(&mut f, t, &jets);
        let tips: Vec<[f64; 3]> = drawn_jet(&jets[0])
            .emitters
            .iter()
            .filter(|e| e.0 == Emitter::Wingtip)
            .map(|e| e.1)
            .collect();
        let near = |r: &&Vec<f32>| {
            tips.iter().any(|w| {
                (0..3)
                    .map(|i| (f64::from(r[4 + i]) - w[i]).powi(2))
                    .sum::<f64>()
                    < 1.0
            })
        };
        let head = ribbons(&d, TIP_ALBEDO)
            .into_iter()
            .filter(near)
            .map(|r| r[11])
            .fold(0.0, f32::max);
        out.push((t, head));
    }
    out
}

#[test]
fn how_the_wingtip_vapour_looks_never_moves_when_it_forms_or_stops() {
    // The same flight under the plain look and the short, wispy one.
    let plain = vapour_at_the_wingtips(frame());
    let wispy = vapour_at_the_wingtips(frame_with(wispy()));
    let level = plain.iter().zip(&wispy).filter(|(p, _)| p.0 < 1.0);
    for ((t, a), (_, b)) in level {
        assert!(*a == 0.0 && *b == 0.0, "none before the turn, at {t}");
    }
    // Where it forms it is one envelope under either look: at the jet each
    // frame, the two looks differ by the same factor.
    let shared: Vec<f64> = plain
        .iter()
        .zip(&wispy)
        .filter(|(p, _)| p.1 > 1e-3)
        .map(|(p, w)| f64::from(w.1) / f64::from(p.1))
        .collect();
    assert!(shared.len() > 60, "vapour through the turn and its release");
    for r in &shared {
        assert!(
            (r / shared[0] - 1.0).abs() < 1e-3,
            "the wispy look's vapour forms and fades as the plain one's: {shared:?}"
        );
    }
}

#[test]
fn wisps_hang_where_they_were_laid_without_seams_and_engines_stay_smooth() {
    let rate = |_: f64| rate_for(CRUISE, 3.0);
    let flight = along(turning(CRUISE, rate));
    let mut f = frame_with(wispy());
    let a = fly(&mut f, 60.0, 1.2, &flight, &flight);
    let b = fly(&mut f, 60.0, 1.2 + 1.0 / 60.0, &flight, &flight);
    let tips =
        |d: &[Vec<f32>]| -> Vec<Vec<f32>> { ribbons(d, TIP_ALBEDO).into_iter().cloned().collect() };
    let (a, b) = (tips(&a), tips(&b));
    assert!(a.len() > 6, "a wingtip trail of several stretches");
    // A wisp (misc.w) breaks up by its amount; its ends are fixed where laid.
    for r in &a {
        assert_eq!(r[15], 0.85, "a wingtip stretch is broken into wisps");
    }
    let mut kept = 0;
    for r in &a {
        if let Some(s) = b.iter().find(|s| s[..3] == r[..3] && s[4..7] == r[4..7]) {
            assert_eq!(
                s[13..15],
                r[13..15],
                "a stretch's wisps stay put as it ages"
            );
            kept += 1;
        }
    }
    assert!(kept > 4, "the same stretches a frame later");
    // Along each trail, one stretch's end is the next one's start.
    for w in a.windows(2) {
        if w[0][4..7] == w[1][..3] {
            assert!(
                (w[0][14] - w[1][13]).abs() < 1e-4,
                "no seam between stretches"
            );
        }
    }
    // Engines leave the plain, even trail.
    let flown = fly(&mut frame_with(wispy()), 60.0, 1.2, &flight, &flight);
    for r in ribbons(&flown, ENGINE_ALBEDO) {
        assert_eq!(r[15], 0.0, "an engine's trail is not broken into wisps");
    }
}

/// `c` drawn with two wing-top sheet points besides its nozzles and tips.
fn sheeted(c: &Craft) -> presentation::effects::exhaust::DrawnJet {
    let mut jet = drawn_jet(c);
    let (s, k) = (c.yaw.sin(), c.yaw.cos());
    for y in [3.0, -3.0] {
        let at = [
            c.at[0] - 2.0 * k - y * s,
            c.at[1] - 2.0 * s + y * k,
            c.at[2] + 0.5,
        ];
        jet.emitters.push((Emitter::Sheet, at));
    }
    jet
}

#[test]
fn the_wing_sheet_rises_only_in_the_hardest_pulls_past_where_the_tips_vapour() {
    let mut style = style();
    style["wing_sheet"] = serde_json::json!([{"albedo": [SHEET_ALBEDO, SHEET_ALBEDO, SHEET_ALBEDO],
        "opacity": [0.0, 0.6], "life_s": LIFE, "width_m": [1, 2], "fade_in_s": 0.05}]);
    let fly_at = |flight: &dyn Fn(f64) -> Vec<Craft>| {
        let drawn = |t: f64| flight(t).iter().map(sheeted).collect();
        fly_jets(&mut frame_with(style.clone()), 60.0, 2.0, flight, drawn)
    };
    let level = fly_at(&straight(2.0, "burner", 1.3 * CRUISE));
    assert!(vapour(&level).is_empty(), "level flight: no tip vapour");
    assert!(ribbons(&level, SHEET_ALBEDO).is_empty(), "and no sheet");
    // Its tips' vapour forms below `SHEET_FROM` of its thickest here…
    let moderate = fly_at(&along(turning(CRUISE, |_| rate_for(CRUISE, 2.2))));
    assert!(
        !vapour(&moderate).is_empty(),
        "a firm turn wisps at the tips"
    );
    assert!(
        ribbons(&moderate, SHEET_ALBEDO).is_empty(),
        "but raises no sheet until the vapour passes sheet_from"
    );
    // …and at its thickest here.
    let hard = fly_at(&along(turning(CRUISE, |_| rate_for(CRUISE, 4.5))));
    assert!(!vapour(&hard).is_empty(), "a hard pull vapours at the tips");
    assert!(
        !ribbons(&hard, SHEET_ALBEDO).is_empty(),
        "and raises the sheet over the wings"
    );
}
