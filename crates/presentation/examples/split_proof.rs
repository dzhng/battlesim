//! Exact, transitional split-game-json controls; remove with slice 16.
//! cargo run -p presentation --example split_proof -- <output directory>
//!
//! JSON numbers use serde's round-trip encoding, never the vector tests'
//! rounding/tolerance. Poses use Rust's lossless float Debug representation
//! to include every pose field without a parallel pose schema. Effect and
//! light floats also carry their exact IEEE-754 bits.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use contract::command::{Engagement, Order, TargetRef};
use contract::ids::{Side, UnitId};
use presentation::assets::Runtime;
use presentation::effects::frame::{EffectBatch, EffectFrame, EFFECT_FLOATS};
use presentation::effects::lights::CAST_LIGHT_FLOATS;
use presentation::effects::publication::EffectFeed;
use presentation::effects::tables::EffectPresentation;
use presentation::fielded::Fielding;
use presentation::seed::side_key;
use presentation::sound::frame::{Heard, Listener, Motion, Sound, SoundSoldier, SoundVehicle};
use presentation::units::drawn::DrawnModels;
use presentation::units::driver::PoseDriver;
use presentation::units::feed::ObservationFeed;
use presentation::units::interpolate::TickInterpolator;
use serde_json::{json, Value};
use sim::battle::Battle;
use sim::publication::{self, reader::Reader, Publisher};

#[path = "split_proof/cases.rs"]
mod cases;

struct Dumps {
    sound: BufWriter<File>,
    effects: BufWriter<File>,
    poses: BufWriter<File>,
    digests: BufWriter<File>,
    hull: BufWriter<File>,
    events: BufWriter<File>,
}

fn line(out: &mut impl Write, value: Value) {
    serde_json::to_writer(&mut *out, &value).unwrap();
    writeln!(out).unwrap();
}

fn run(case: cases::Case, runtime: &Runtime, out: &Path, dumps: &mut Dumps) -> Value {
    let mut battle = Battle::new(&case.scenario, 1);
    let bomb = battle.rules().weapons[&case.expected[0]]
        .ballistics
        .delivery
        == contract::ballistics::Delivery::Unpowered;
    if bomb {
        let target = battle.unit(UnitId(1)).unwrap().position;
        let ack = battle.accept(contract::command::CommandEnvelope {
            side: Side::Blue,
            player: None,
            seq: 1,
            queued: false,
            order: Order::Attack {
                units: vec![UnitId(0)],
                target: TargetRef::Ground {
                    point: [target.x, target.y, target.z],
                },
            },
        });
        assert_eq!(ack.error, None);
    }
    let catalog = battle.rules().catalog.view();
    let kinds: Vec<String> = case.scenario.units.iter().map(|u| u.kind.clone()).collect();
    let rules = serde_json::to_value(battle.rules()).unwrap();
    let fielded = Fielding {
        presentation: &serde_json::from_str(include_str!("../../../fixtures/presentation.json"))
            .unwrap(),
        weapons: &case.game["weapons"],
        rules: &rules,
        catalog: &catalog,
        runtime: &runtime.catalog,
        kinds: &kinds,
        factions: None,
    }
    .resolve(&|hash| runtime.read(hash, "bundle.bin"))
    .unwrap()
    .typed()
    .unwrap();
    let units = Arc::new(fielded.units);
    let effects = fielded.effects;
    let tick_hz = f64::from(battle.rules().tick_hz);
    let p = EffectPresentation::from_value(&effects.presentation).unwrap();
    let mut batch = EffectBatch::new(p.capacity);
    let mut frame = EffectFrame::new(tick_hz, p);
    let mut effect_feed = EffectFeed::new("blue");
    let mut pose_feed = ObservationFeed::new("blue");
    let mut driver = PoseDriver::new(&units).unwrap();
    let mut drawn = DrawnModels::new(units.clone(), Arc::new(fielded.tables), "blue");
    let mut interpolator = TickInterpolator::new(tick_hz);
    let mut publisher = Publisher::new();
    let layout = publication::layout_json(&battle);
    if case.hull_trace {
        fs::write(out.join("hull-layout.json"), &layout).unwrap();
    }
    let mut reader = Reader::new(&layout).unwrap();
    // Same command-only setup and deterministic durations as sound.rs;
    // no audio device or recording decoder is part of this control.
    let mut sound = Sound::from_json(&json!({
        "presentation": serde_json::from_str::<Value>(include_str!("../../../fixtures/presentation.json")).unwrap(),
        "audio": serde_json::from_str::<Value>(include_str!("../../../fixtures/audio.json")).unwrap(),
        "catalog": serde_json::from_slice::<Value>(&fs::read(sim::fixtures::dir().join("sounds.json")).unwrap()).unwrap(),
        "weapons": case.game["weapons"], "tick_hz": tick_hz,
        "effects": {"smoke": effects.presentation["smoke"], "cook_off": effects.presentation["cook_off"]}
    }).to_string()).unwrap();
    sound
        .set_durations(
            sound
                .tables()
                .battle_sounds()
                .into_iter()
                .map(|n| (n, vec![2.0; 8]))
                .collect(),
        )
        .unwrap();
    let listener = Listener {
        position: [100.0, 180.0, 2.0],
        forward: [1.0, 0.0, 0.0],
    };
    let mut seen_rounds = BTreeSet::new();
    let mut fired = BTreeMap::<String, usize>::new();
    let mut counts = [0usize; 6]; // commands, instances, lights, poses, segments, blasts
    let mut held = false;
    for tick in 0..=40 * battle.rules().tick_hz as u64 {
        if tick > 0 {
            battle.step();
        }
        let words: Vec<u32> = publisher
            .publish(&battle, Side::Blue)
            .unwrap()
            .iter()
            .map(|v| v.to_bits())
            .collect();
        let observation = Arc::new(reader.decode(&words).unwrap().expect("fresh tick"));
        let pub_ = effect_feed.publication(&observation, &units, &effects, None);
        let launches = frame.note(&pub_);
        let fresh: Vec<_> = battle
            .rounds()
            .filter_map(|(p, r)| {
                if !seen_rounds.insert(p.id) {
                    return None;
                }
                let weapon = &battle.arsenal().weapons[r.weapon].id;
                if r.unit == UnitId(0) {
                    *fired.entry(weapon.clone()).or_default() += 1;
                }
                Some(json!({"projectile": p.id.0, "unit": r.unit.0, "weapon": weapon}))
            })
            .collect();
        if !fresh.is_empty()
            || !launches.is_empty()
            || !pub_.segments.is_empty()
            || !pub_.blasts.is_empty()
        {
            line(
                &mut dumps.events,
                json!({"case": case.name, "tick": tick, "rounds": fresh,
                "publication": pub_, "shooters": format!("{:?}", pub_.shooters), "launches": format!("{launches:?}")}),
            );
        }
        counts[4] += pub_.segments.len();
        counts[5] += pub_.blasts.len();
        line(
            &mut dumps.digests,
            json!({"case": case.name, "tick": tick, "digest": format!("{:016x}", battle.digest())}),
        );
        if case.hull_trace {
            line(
                &mut dumps.hull,
                json!({"tick": tick, "publication": words,
                "mounts": format!("{:?}", observation.own[0].mounts),
                "weapon_poses": format!("{:?}", observation.own[0].weapon_poses)}),
            );
        }
        let now = tick as f64 / tick_hz;
        interpolator.push(observation.clone(), now);
        // Sample at the published tick and halfway to it; short flashes and
        // spring recoil must not disappear between tick-only samples.
        for fraction in [0.5, 1.0] {
            let sample = interpolator.frame(now + fraction / tick_hz).unwrap();
            let clock = sample.time;
            let poses = driver.update(pose_feed.frame(sample, &units, &effects), &units, None);
            drawn.update(poses);
            let rotors = drawn.rotors();
            frame.build(clock, &mut batch, Some(&mut drawn), &rotors, &[], None);
            assert_eq!(
                (batch.dropped, batch.lights.dropped),
                (0, 0),
                "{}: control overflow",
                case.name
            );
            let instances = &batch.data[..batch.count * EFFECT_FLOATS];
            let lights = &batch.lights.data[..batch.lights.count * CAST_LIGHT_FLOATS];
            line(
                &mut dumps.effects,
                json!({"case": case.name, "tick": tick, "clock": clock,
                "instances": instances, "instance_bits": instances.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "lights": lights, "light_bits": lights.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "causes": &batch.lights.causes[..batch.lights.count], "wrap": batch.lights.wrap}),
            );
            writeln!(
                dumps.poses,
                "{} tick={tick} clock={clock:?} {poses:?}",
                case.name
            )
            .unwrap();
            counts[1] += batch.count;
            counts[2] += batch.lights.count;
            counts[3] += poses.soldiers.len()
                + poses.vehicles.len()
                + poses.corpses.len()
                + poses.fading.len();
            let motion = Motion {
                vehicles: poses
                    .vehicles
                    .iter()
                    .filter_map(|v| {
                        Some(SoundVehicle {
                            key: side_key(v.unit, &*v.side == "blue"),
                            vehicle_class: units.types.get(&*v.kind)?.vehicle_class.clone()?,
                            position: v.position,
                            travel_l: v.articulation.travel_l,
                            travel_r: v.articulation.travel_r,
                            turret: v.articulation.turret_yaw,
                            reverse: false,
                        })
                    })
                    .collect(),
                soldiers: poses
                    .soldiers
                    .iter()
                    .map(|s| SoundSoldier {
                        id: s.soldier,
                        position: s.position,
                    })
                    .collect(),
            };
            let commands = if fraction == 0.5 {
                sound
                    .note(
                        &Heard {
                            effects: &pub_,
                            launches: &launches,
                            audible: observation.audible.iter().map(Into::into).collect(),
                            shouts: observation.shouts.iter().map(Into::into).collect(),
                        },
                        now,
                    )
                    .to_vec()
            } else {
                Vec::new()
            };
            counts[0] += commands.len();
            let updated = sound.update(
                clock,
                now + fraction / tick_hz,
                now + fraction / tick_hz,
                &motion,
                listener,
            );
            counts[0] += updated.len();
            line(
                &mut dumps.sound,
                json!({"case": case.name, "tick": tick, "clock": clock, "note": commands, "update": updated}),
            );
        }
        if !case.hull_trace && !held && !fired.is_empty() {
            // Let the first burst finish, then keep its landing/tail without
            // filling the control with continuous fire. Autonomous weapons
            // retain their authored ability to keep firing.
            if tick > 5 * battle.rules().tick_hz as u64 {
                let ack = battle.accept(contract::command::CommandEnvelope {
                    side: Side::Blue,
                    player: None,
                    seq: if bomb { 2 } else { 1 },
                    queued: false,
                    order: Order::SetEngagement {
                        units: vec![UnitId(0)],
                        policy: Engagement::ReturnFireOnly,
                    },
                });
                assert_eq!(ack.error, None);
                held = true;
            }
        }
    }
    for weapon in &case.expected {
        assert!(
            fired.get(weapon).is_some_and(|n| *n > 0),
            "{} never fired {weapon}; observed {fired:?}; shooter {:?}",
            case.name,
            battle.unit(UnitId(0))
        );
    }
    assert!(
        counts[0] > 0 && counts[1] > 0 && counts[3] > 0,
        "{}: empty control",
        case.name
    );
    eprintln!(
        "{}: {fired:?}, {} commands, {} instances, {} lights",
        case.name, counts[0], counts[1], counts[2]
    );
    json!({"case": case.name, "fired": fired, "ticks": battle.tick() + 1,
        "sound_commands": counts[0], "effect_instances": counts[1], "lights": counts[2],
        "poses": counts[3], "segments": counts[4], "blasts": counts[5]})
}

fn main() {
    let out = std::env::args_os().nth(1).expect("output directory");
    let out = Path::new(&out);
    fs::create_dir_all(out).unwrap();
    let open = |name| BufWriter::new(File::create(out.join(name)).unwrap());
    let mut dumps = Dumps {
        sound: open("sound.jsonl"),
        effects: open("effects.jsonl"),
        poses: open("poses.txt"),
        digests: open("digests.jsonl"),
        hull: open("hull-publication.jsonl"),
        events: open("events.jsonl"),
    };
    let runtime = Runtime::open(&sim::fixtures::dir().join("../assets/runtime")).unwrap();
    let game = sim::fixtures::rules();
    let weapons = game["weapons"].as_object().unwrap();
    let filter = std::env::args().nth(2);
    assert_eq!(
        weapons.len(),
        49,
        "update slice 01's coverage contract deliberately"
    );
    let mut summaries = Vec::new();
    for id in weapons.keys() {
        if filter.as_ref().is_some_and(|name| name != id) {
            continue;
        }
        summaries.push(run(
            cases::weapon(id, false, false),
            &runtime,
            out,
            &mut dumps,
        ));
    }
    for id in ["rifle", "carbine"] {
        if filter
            .as_ref()
            .is_some_and(|name| name != &format!("soldier-{id}"))
        {
            continue;
        }
        summaries.push(run(
            cases::weapon(id, false, true),
            &runtime,
            out,
            &mut dumps,
        ));
    }
    if filter
        .as_ref()
        .is_none_or(|name| name == "soldier-proof_encounter_carbine")
    {
        summaries.push(run(
            cases::weapon("proof_encounter_carbine", true, true),
            &runtime,
            out,
            &mut dumps,
        ));
    }
    if filter.as_ref().is_none_or(|name| name == "bmp-3") {
        summaries.push(run(cases::bmp(), &runtime, out, &mut dumps));
    }
    assert!(!summaries.is_empty(), "unknown case {filter:?}");
    fs::write(out.join("coverage.json"), serde_json::to_string_pretty(&json!({
        "weapon_rows": weapons.keys().collect::<Vec<_>>(), "only_case": filter, "cases": summaries,
        "notes": ["helicopter_crash is mounted on the synthetic hull to fire its unmodified row",
                  "command-only audio uses sound.rs's fixed 2-second take durations",
                  "poses are full, unrounded Rust Debug records; effects/lights include float bits"]
    })).unwrap() + "\n").unwrap();
    for stream in [
        &mut dumps.sound,
        &mut dumps.effects,
        &mut dumps.poses,
        &mut dumps.digests,
        &mut dumps.hull,
        &mut dumps.events,
    ] {
        stream.flush().unwrap();
    }
}
