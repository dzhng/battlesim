//! Run: cargo run -p sim --release --example concealment_shots
//! Writes deterministic GIFs, PNG frames, contact-sheet.png and report.txt to
//! throwaway/concealment-shots/. Requires ffmpeg; failed encoding is an error.
//!
//! Replays the authored comparison in tests/concealment.rs with actual approach
//! orders. Equal all-round sight and zero crown attenuation isolate concealment;
//! these are diagnostic battles, not roster balance or WebGPU appearance checks.
#[path = "common/canvas.rs"]
mod canvas;

use canvas::{encode_gif, Canvas, Rgb};
use contract::ids::{Side, UnitId};
use contract::scenario::ScenarioDefinition;
use serde_json::json;
use sim::battle::Battle;
use sim::math::{v2, Obb2, V2};
use std::fs::{create_dir_all, remove_dir_all};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const W: u32 = 960;
const H: u32 = 280;
const PAPER: Rgb = [233, 231, 225];
const INK: Rgb = [40, 40, 40];
const FOREST: Rgb = [206, 224, 198];
const IDENTIFIED: Rgb = [20, 110, 40];
const UNIDENTIFIED: Rgb = [155, 65, 40];
const OBSERVER: Rgb = [58, 62, 72];

#[derive(Clone, Copy)]
struct Lane {
    name: &'static str,
    observer: &'static str,
    label: &'static str,
    forest: bool,
    driving: bool,
}

/// Synthetic values match the system comparison, rather than depending on
/// today's roster tuning. The AT team's body stands in for a sniper team.
fn setup(lane: Lane, ordinary: bool) -> ScenarioDefinition {
    let mut rules = sim::fixtures::test_game();
    for (id, concealment, spotting) in [
        ("test_rifle", 1.0, 1.0),
        ("test_at", 0.5, 2.0),
        ("test_recon", 0.8, 2.0),
        ("test_tank", 2.8, 0.8),
    ] {
        sim::fixtures::patch_catalog(
            &mut rules,
            "units",
            id,
            json!({"concealment": concealment,
                "sensors": {"ground_m": 600, "low_air_m": 600, "high_air_m": 600,
                    "spotting": spotting, "sight_shape": {"front": 1, "side": 1, "rear": 1}}}),
        );
    }
    rules["sensors"]["moving_concealment"] = json!(1.5);
    rules["forests"]["rule"]["concealment"] = json!(0.25);
    rules["forests"]["rule"]["attenuation_per_m"] = json!(0);
    let actor = if lane.driving { 1 } else { 0 };
    serde_json::from_value(json!({
        "map": {"size": [2000,240], "fog_cell_m": 8, "height_grid_m": 4,
            "slope_cutoff_deg": 35, "forests": if lane.forest {json!([{
                "shape": {"kind": "polygon", "ring": [[900,0],[1100,0],[1100,240],[900,240]]}
            }])} else {json!([])}},
        "rules": rules,
        "units": [
            {"side": "blue", "kind": lane.observer, "position": [1800,120],
                "yaw": std::f64::consts::PI, "engagement": "return_fire_only"},
            {"side": "red", "kind": if lane.driving {"test_tank"} else if ordinary {"test_rifle"} else {"test_at"},
                "position": [1000,120], "engagement": "return_fire_only"}
        ], "events": [], "scripts": [{"tick": 31,
            "side": if lane.driving {"red"} else {"blue"},
            "order": {"kind": "move", "units": [actor], "gesture": 1,
                "goal": if lane.driving {[1040,120]} else {[1015,120]}, "route": "shortest"}}]
    })).unwrap()
}

/// Identifying centre-to-centre distance, sampled every tick through the public
/// sensing owner (no knowledge grace or stagger delay). HIDDEN is published.
struct Sample {
    tick: u64,
    distance: f64,
    identified: bool,
    hidden: bool,
    moving: bool,
    units: Vec<sim::units::Unit>,
    cleared: Vec<V2>,
    cell_m: f64,
    target_ground: f64,
}

fn sample(b: &Battle) -> Sample {
    let units: Vec<_> = (0..2).map(|i| b.unit(UnitId(i)).unwrap().clone()).collect();
    Sample {
        tick: b.tick(),
        distance: (units[0].position - units[1].position).length(),
        identified: sim::sensing::evaluate(b.world(), &units, b.rules(), Side::Blue, |_| true)
            .iter()
            .any(|s| s.observer == UnitId(0) && s.target == UnitId(1)),
        hidden: b
            .observe(Side::Red)
            .own
            .iter()
            .find(|u| u.id == UnitId(1))
            .unwrap()
            .concealed,
        moving: units[1].moving(),
        target_ground: b
            .world()
            .forest_concealment(units[1].position.x, units[1].position.y),
        units,
        cleared: b.world().cleared_since(0).collect(),
        cell_m: b.world().cleared_cell_m(),
    }
}

/// Same metre-to-pixel scale for all approach lanes; north points up.
fn px(p: V2) -> V2 {
    v2(30.0 + 1800.0 - p.x, 170.0 + 120.0 - p.y)
}

fn frame(lane: Lane, s: &Sample, first: Option<f64>, ordinary: f64) -> Canvas {
    let mut cv = Canvas::new(W, H, PAPER);
    if lane.forest {
        for y in 108..234 {
            for x in 730..930 {
                cv.px(x, y, FOREST);
            }
        }
        for &at in &s.cleared {
            cv.obb(
                &Obb2 {
                    center: px(at),
                    half: v2(s.cell_m, s.cell_m) * 0.5,
                    yaw: 0.0,
                },
                [180, 150, 100],
            );
        }
        cv.text(742, 218, "FOREST", 2, INK);
    }
    cv.text(
        14,
        12,
        &format!(
            "{} / {}",
            lane.label,
            if lane.driving {
                "FOREST DRIVE"
            } else if lane.forest {
                "FOREST EDGE"
            } else {
                "OPEN FIELD"
            }
        ),
        3,
        INK,
    );
    cv.text(
        14,
        38,
        &format!(
            "TICK={}  GAP={:.1}M  {}",
            s.tick,
            s.distance,
            if s.identified {
                "IDENTIFIED"
            } else {
                "UNIDENTIFIED"
            }
        ),
        3,
        if s.identified {
            IDENTIFIED
        } else {
            UNIDENTIFIED
        },
    );
    let caption = if lane.driving {
        format!("MBT 2.8 / {}", if s.moving { "DRIVING" } else { "PARKED" })
    } else {
        "SNIPER 0.5".into()
    };
    cv.text(14, 66, &caption, 3, INK);
    // A filled/outlined badge makes the published concealment transition read
    // independently of the observer's identification colour.
    let badge = Obb2 {
        center: v2(830.0, 72.0),
        half: v2(116.0, 17.0),
        yaw: 0.0,
    };
    cv.obb(&badge, INK);
    if !s.hidden {
        cv.obb(
            &Obb2 {
                half: badge.half - v2(2.0, 2.0),
                ..badge
            },
            PAPER,
        );
    }
    let badge_ink = if s.hidden { PAPER } else { INK };
    cv.text(728, 58, "OWN PUBLICATION", 2, badge_ink);
    cv.text(
        728,
        72,
        if s.hidden { "HIDDEN" } else { "NOT HIDDEN" },
        3,
        badge_ink,
    );
    let a = px(s.units[0].position.xy());
    let t = px(s.units[1].position.xy());
    let colour = if s.identified {
        IDENTIFIED
    } else {
        UNIDENTIFIED
    };
    let dir = (t - a).normalized();
    cv.line(
        a + dir * 11.0,
        t - dir * 16.0,
        2.0,
        colour,
        if s.identified { None } else { Some((4.0, 4.0)) },
        0.0,
    );
    cv.ring(a.x, a.y, 11.0, 1.5, OBSERVER);
    cv.ring(t.x, t.y, 16.0, if s.identified { 3.0 } else { 1.5 }, colour);
    for id in 0..2 {
        let u = &s.units[id];
        let c = if id == 0 { OBSERVER } else { INK };
        if let Some(hull) = u.ground_footprint() {
            cv.obb(
                &Obb2 {
                    center: px(hull.center),
                    half: hull.half * 2.0,
                    yaw: std::f64::consts::PI + hull.yaw,
                },
                c,
            );
        } else {
            for soldier in u.members.iter().filter(|m| m.alive()) {
                let p = px(soldier.position.xy());
                cv.disc(p.x, p.y, 3.0, c);
            }
        }
    }
    actor_label(
        &mut cv,
        v2((a.x - 24.0).max(14.0), a.y - 30.0),
        "OBSERVER",
        s.units[0].yaw,
    );
    actor_label(
        &mut cv,
        v2(t.x - 24.0, t.y + 26.0),
        "TARGET",
        s.units[1].yaw,
    );
    let identification = if lane.driving {
        "ID NONE".into()
    } else {
        first.map_or("ID PENDING".into(), |d| format!("ID {d:.1}M"))
    };
    cv.text(t.x as i32 - 24, t.y as i32 + 44, &identification, 2, colour);
    cv.text(
        16,
        244,
        &format!("600M EYES / NO CROWN LOSS / ORDINARY RIFLE ID {ordinary:.1}M"),
        2,
        INK,
    );
    cv.text(
        16,
        262,
        "DASHED=UNIDENTIFIED  SOLID=IDENTIFIED / HIDDEN FROM PUBLICATION / TAN=CLEARED / ARROW=FACING",
        2,
        INK,
    );
    cv
}

/// Heading belongs beside the actor label, leaving the sensing connector free.
fn actor_label(cv: &mut Canvas, at: V2, label: &str, yaw: f64) {
    cv.text(at.x as i32, at.y as i32, label, 2, INK);
    let c = at + v2(label.len() as f64 * 8.0 + 10.0, 5.0);
    let forward = v2(1.0, 0.0).rotated(std::f64::consts::PI + yaw);
    let tip = c + forward * 5.0;
    cv.line(c - forward * 5.0, tip, 2.0, INK, None, 0.0);
    let cross = v2(-forward.y, forward.x) * 4.0;
    for wing in [cross, -cross] {
        cv.line(c + wing, tip, 2.0, INK, None, 0.0);
    }
}

fn write_frame(cv: &Canvas, dir: &Path, count: &mut usize) {
    cv.write(&dir.join(format!("t{count:03}.png")));
    *count += 1;
}

struct ResultRow {
    lane: Lane,
    distance: f64,
    picks: Vec<Canvas>,
}

fn shoot(lane: Lane, root: &Path, ordinary: f64) -> ResultRow {
    let dir = root.join(lane.name);
    create_dir_all(&dir).unwrap();
    let setup = setup(lane, false);
    let hz = setup.rules.tick_hz as u64;
    let mut b = Battle::new(&setup, 7);
    let (mut count, mut first, mut picks) = (0, None, Vec::new());
    let mut before: Option<Sample> = None;
    // Bound the real approach: a stopped or misrouted observer fails, never loops.
    for _ in 0..600 * hz {
        b.step();
        let s = sample(&b);
        let event = if lane.driving {
            s.moving && !s.hidden
        } else {
            s.identified
        };
        if event {
            if lane.driving {
                assert!(
                    s.target_ground < 1.0,
                    "moving exposes the MBT before its ground is cleared"
                );
            }
            first = Some(s.distance);
            if let Some(before) = before {
                let cv = frame(lane, &before, None, ordinary);
                cv.write(&dir.join("before.png"));
                for _ in 0..8 {
                    write_frame(&cv, &dir, &mut count);
                }
                picks.push(cv);
            }
            let cv = frame(lane, &s, first, ordinary);
            cv.write(&dir.join("moment.png"));
            picks.push(cv.clone());
            // Pause at the transition, then let the forest tank visibly travel.
            for _ in 0..15 {
                write_frame(&cv, &dir, &mut count);
            }
            if !lane.driving {
                // The published HIDDEN reads the battle's own sightings, which
                // can trail this direct check by a tick: show it clearing.
                for _ in 0..3 * hz {
                    if !sample(&b).hidden {
                        break;
                    }
                    b.step();
                }
                let after = frame(lane, &sample(&b), first, ordinary);
                for _ in 0..8 {
                    write_frame(&after, &dir, &mut count);
                }
                break;
            }
            for _ in 0..6 {
                for _ in 0..hz {
                    b.step();
                }
                let s = sample(&b);
                assert!(
                    s.moving && !s.hidden,
                    "driving tank remains exposed in forest"
                );
                write_frame(&frame(lane, &s, first, ordinary), &dir, &mut count);
            }
            break;
        }
        if b.tick() % (6 * hz) == 1 || b.tick() == 30 {
            write_frame(&frame(lane, &s, first, ordinary), &dir, &mut count);
        }
        // Keep the exact tick immediately before identification, not a distant
        // periodic frame that conceals how the threshold was crossed.
        assert_eq!(
            s.hidden, lane.forest,
            "stationary target HIDDEN requires forest"
        );
        before = Some(s);
    }
    let distance = first.expect("identification or HIDDEN transition within 600 seconds");
    assert!(encode_gif(
        &dir,
        &root.join(format!("{}.gif", lane.name)),
        10
    ));
    println!("{}: {distance:.1}m, {count} frames", lane.name);
    ResultRow {
        lane,
        distance,
        picks,
    }
}

fn ordinary_reach(lane: Lane) -> f64 {
    let setup = setup(lane, true);
    let mut b = Battle::new(&setup, 7);
    for _ in 0..600 * setup.rules.tick_hz {
        b.step();
        let s = sample(&b);
        if s.identified {
            return s.distance;
        }
    }
    panic!("ordinary target never identified");
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../throwaway/concealment-shots");
    if let Err(e) = remove_dir_all(&root) {
        assert_eq!(e.kind(), ErrorKind::NotFound);
    }
    create_dir_all(&root).unwrap();
    let mut rows = Vec::new();
    let mut ordinary = 0.0;
    for (observer, label) in [
        ("test_rifle", "RIFLE"),
        ("test_recon", "SCOUTS"),
        ("test_tank", "MBT"),
    ] {
        for forest in [false, true] {
            let name = match (observer, forest) {
                ("test_rifle", false) => "rifle-open",
                ("test_rifle", true) => "rifle-forest",
                ("test_recon", false) => "scouts-open",
                ("test_recon", true) => "scouts-forest",
                (_, false) => "mbt-open",
                (_, true) => "mbt-forest",
            };
            let lane = Lane {
                name,
                observer,
                label,
                forest,
                driving: false,
            };
            if rows.is_empty() {
                ordinary = ordinary_reach(lane);
            }
            rows.push(shoot(lane, &root, ordinary));
        }
    }
    assert!(
        rows[0].distance < ordinary,
        "rifle identifies sniper closer than ordinary"
    );
    assert!(
        rows[2].distance > rows[0].distance,
        "scouts identify farther"
    );
    assert!(
        rows[4].distance < rows[0].distance,
        "MBT identifies closest"
    );
    for pair in rows.as_chunks::<2>().0 {
        assert!(
            pair[1].distance < pair[0].distance,
            "forest shortens every distance"
        );
    }
    rows.push(shoot(
        Lane {
            name: "mbt-forest-drive",
            observer: "test_rifle",
            label: "MBT",
            forest: true,
            driving: true,
        },
        &root,
        ordinary,
    ));
    let mut report = format!("Authored diagnostic battles, seed 7. Equal 600m all-round eyes; crown attenuation 0.\nSniper 0.5/spotting 2; scouts 0.8/2; rifle 1/1; MBT 2.8/0.8; forest 0.25; moving 1.5.\nIdentification from sensing::evaluate every tick; centre distances, not theoretical range or knowledge grace.\nA squad identifies from its centre eye to living soldier samples, so centre range can exceed nominal sight.\nHIDDEN from Battle::observe(Red).own.concealed; visible enemies spotting it can also clear the published flag.\nFrames every 6 simulation seconds; pauses at the exact ticks bracketing the transition. Hull symbols enlarged for legibility; same metre scale in every lane.\nOrdinary rifle target: {ordinary:.1}m.\n");
    let mut sheet = Canvas::new(2 * W, rows.len() as u32 * H, PAPER);
    for (i, row) in rows.iter().enumerate() {
        for (col, cv) in row.picks.iter().enumerate() {
            sheet.blit(&cv.shrink(W), col as u32 * W, i as u32 * H);
        }
        report += &format!(
            "{}: {} at {:.1}m\n",
            row.lane.name,
            if row.lane.driving {
                "HIDDEN -> NOT HIDDEN while driving"
            } else {
                "first identification"
            },
            row.distance
        );
    }
    report += "PASS: rifle/sniper < ordinary; scouts > rifle > MBT; every forest distance < open.\nPASS: parked forest MBT HIDDEN; driving MBT NOT HIDDEN before clearing its ground.\nContinued driving clears a lane (tan cells from the world), independently of the movement concealment penalty.\nContact sheet: each row before transition (left), transition (right).\n";
    sheet.write(&root.join("contact-sheet.png"));
    std::fs::write(root.join("report.txt"), &report).unwrap();
    print!("{report}");
}
