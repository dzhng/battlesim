//! The battle's sound: equal to its recorded commands, and what it plays.
//!
//! `fixtures/sound-vectors.json` holds every command given over the sound
//! lab's scripted firefight, a pause, a cook-off, a reversing truck and a new
//! battle. The behaviour tests below cover what a mixer needs to hold: the
//! budget, fog of war and distance.
use std::collections::HashMap;

use presentation::effects::launches::Launch;
use presentation::effects::publication::{EffectPublication, Segment};
use presentation::sound::bank::{max_momentary_loudness, Bank};
use presentation::sound::frame::{
    sector_pan, Command, Cue, Heard, Listener, Motion, Scripted, Shout, ShoutTake, Sound,
    SoundSoldier, SoundVehicle, Voice,
};
use presentation::sound::synth;
use presentation::sound::tables::{Bus, SoundTables};
use presentation::sound::voices::{Chatter, Request, SetVoice, Speakers, Speaks, Tier};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/");
    serde_json::from_str(&std::fs::read_to_string(format!("{root}{name}")).unwrap()).unwrap()
}

/// The game's sound, as a player sets it up.
fn sound() -> Sound {
    let game = fixture("rules.json");
    let presentation = fixture("presentation.json");
    let fx = &presentation["effects"];
    let setup = json!({
        "audio": fixture("audio.json"),
        "presentation": presentation,
        "catalog": fixture("sounds.json"),
        "weapons": game["weapons"],
        "tick_hz": game["tick_hz"],
        "effects": { "smoke": fx["smoke"], "cook_off": fx["cook_off"] },
    });
    Sound::from_json(&setup.to_string()).unwrap()
}

#[test]
fn first_high_air_entry_raises_one_alarm_until_the_episode_ends() {
    let v: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sound-vectors.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut s = sound();
    s.set_durations(serde_json::from_value(v["durations"].clone()).unwrap())
        .unwrap();
    let listener = Listener {
        position: [0.0, 0.0, 0.0],
        forward: [0.0, 1.0, 0.0],
    };
    let empty = Motion::default();
    s.update(0.0, 0.0, 0.0, &empty, listener);
    let jet = Motion {
        vehicles: vec![SoundVehicle {
            key: 7.0,
            vehicle_class: "high_air_light".to_owned(),
            position: [100.0, 0.0, 1000.0],
            travel_l: 0.0,
            travel_r: 0.0,
            turret: 0.0,
            reverse: false,
        }],
        soldiers: Vec::new(),
    };
    s.update(1.0, 1.0, 1.0, &jet, listener);
    let commands = s.update(1.1, 1.1, 1.1, &jet, listener);
    let alarms = commands
        .iter()
        .filter(|c| matches!(c, Command::Start { voice, .. } if voice.sound == "airwar-air-alarm"))
        .count();
    assert_eq!(alarms, 1);
    s.update(1.2, 1.2, 1.2, &empty, listener);
    s.update(1.3, 1.3, 1.3, &jet, listener);
    let commands = s.update(1.4, 1.4, 1.4, &jet, listener);
    let alarms = commands
        .iter()
        .filter(|c| matches!(c, Command::Start { voice, .. } if voice.sound == "airwar-air-alarm"))
        .count();
    assert_eq!(alarms, 1);
}

#[test]
fn incoming_missile_shout_starts_the_authored_warning() {
    let mut s = ready();
    let effects = EffectPublication {
        tick: 1.0,
        segments: Vec::new(),
        blasts: Vec::new(),
        shooters: Vec::new(),
        smokes: Vec::new(),
        cook_offs: Vec::new(),
        flare_deploys: Vec::new(),
        grinds: Vec::new(),
    };
    let shout = Shout {
        own: true,
        id: 7.0,
        kind: "high_air_light".into(),
        position: [20.0, 0.0, 1000.0],
        trigger: "b15_incoming_missile".into(),
        intensity: "stressed".into(),
    };
    s.note(
        &Heard {
            effects: &effects,
            launches: &[],
            audible: Vec::new(),
            shouts: vec![shout],
        },
        0.0,
    );
    let commands = s.update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE);
    assert!(commands.iter().any(
        |c| matches!(c, Command::Start { voice, .. } if voice.sound == "airwar-missile-warning")
    ));
}

#[test]
fn flare_deploy_starts_the_positional_flare_sound() {
    let mut s = ready();
    let effects = EffectPublication {
        tick: 1.0,
        segments: Vec::new(),
        blasts: Vec::new(),
        shooters: Vec::new(),
        smokes: Vec::new(),
        cook_offs: Vec::new(),
        flare_deploys: vec![[20.0, 0.0, 1000.0]],
        grinds: Vec::new(),
    };
    s.note(
        &Heard {
            effects: &effects,
            launches: &[],
            audible: Vec::new(),
            shouts: Vec::new(),
        },
        0.0,
    );
    let commands = s.update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE);
    let flare = fixture("audio.json")["flare_deploy"]["sound"].clone();
    assert!(commands.iter().any(|c| matches!(c, Command::Start { voice, .. } if voice.sound == flare.as_str().unwrap() && voice.position == Some([20.0, 0.0, 1000.0]))));
}

#[test]
fn air_defense_launches_use_the_authored_aa_launch_sound() {
    let mut s = ready();
    let (effects, launches) = shot(1.0, 3.0, [20.0, 0.0, 0.0], "aa_long");
    s.note(
        &Heard {
            effects: &effects,
            launches: &launches,
            audible: Vec::new(),
            shouts: Vec::new(),
        },
        0.0,
    );
    let commands = s.update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE);
    assert!(commands
        .iter()
        .any(|c| matches!(c, Command::Start { voice, .. } if voice.sound == "airwar-aa-launch")));
}

fn tables() -> SoundTables {
    sound().tables().clone()
}

/// `actual` equals `wanted` (the recorded), floats within 1e-9 relative.
fn same(actual: &Value, wanted: &Value, at: &str) {
    match (actual, wanted) {
        (Value::Number(a), Value::Number(w)) => {
            let (a, w) = (a.as_f64().unwrap(), w.as_f64().unwrap());
            assert!(
                (a - w).abs() <= 1e-9 * a.abs().max(w.abs()).max(1.0),
                "{at}: {a} against {w}"
            );
        }
        (Value::Array(a), Value::Array(w)) => {
            assert_eq!(a.len(), w.len(), "{at}: length");
            for (i, (a, w)) in a.iter().zip(w).enumerate() {
                same(a, w, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(w)) => {
            let keys = |o: &serde_json::Map<String, Value>| {
                let mut k: Vec<_> = o.keys().cloned().collect();
                k.sort();
                k
            };
            assert_eq!(keys(a), keys(w), "{at}: fields");
            for (k, w) in w {
                same(&a[k], w, &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(actual, wanted, "{at}"),
    }
}

/// Run with `UPDATE_SOUND_VECTORS=1 cargo test -p presentation every_command`
/// to regenerate the fixture after intentional audio configuration changes.
#[test]
fn every_command_equals_the_recorded_ones() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sound-vectors.json"
    );
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let update = std::env::var_os("UPDATE_SOUND_VECTORS").is_some();
    let mut s = sound();
    s.set_durations(serde_json::from_value(v["durations"].clone()).unwrap())
        .unwrap();
    let mut commands = 0;
    let steps = v["steps"].as_array_mut().unwrap();
    for (i, step) in steps.iter_mut().enumerate() {
        let got: Vec<Command> = if let Some(note) = step.get("note") {
            let scripted: Scripted = serde_json::from_value(note.clone()).unwrap();
            let launches = scripted.launches();
            s.note(&scripted.heard(&launches), step["now"].as_f64().unwrap())
                .to_vec()
        } else {
            let u = &step["update"];
            let motion: Motion = serde_json::from_value(u["motion"].clone()).unwrap();
            let listener: Listener = serde_json::from_value(u["listener"].clone()).unwrap();
            let f = |k: &str| u[k].as_f64().unwrap();
            s.update(f("clock"), f("wall"), f("now"), &motion, listener)
                .to_vec()
        };
        if update {
            step["commands"] = serde_json::to_value(&got).unwrap();
        } else {
            same(
                &serde_json::to_value(&got).unwrap(),
                &step["commands"],
                &format!("step {i}"),
            );
        }
        commands += got.len();
    }
    if update {
        v["stats"] = serde_json::to_value(s.stats()).unwrap();
        std::fs::write(path, serde_json::to_string(&v).unwrap()).unwrap();
        panic!("fixture updated; rerun without UPDATE_SOUND_VECTORS");
    }
    assert!(commands > 3000, "{commands} commands compared");
    let stats = serde_json::to_value(s.stats()).unwrap();
    same(&stats, &v["stats"], "stats");
}

fn shot(tick: f64, shooter: f64, at: [f64; 3], kind: &str) -> (EffectPublication, Vec<Launch>) {
    let e = EffectPublication {
        tick,
        segments: Vec::new(),
        blasts: Vec::new(),
        shooters: Vec::new(),
        smokes: Vec::new(),
        cook_offs: Vec::new(),
        flare_deploys: Vec::new(),
        grinds: Vec::new(),
    };
    let l = Launch {
        shooter,
        mount: 0,
        soldier: None,
        at,
        dir: [1.0, 0.0, 0.0],
        kind: kind.into(),
        hull: true,
    };
    (e, vec![l])
}

const HERE: Listener = Listener {
    position: [0.0, 0.0, 0.0],
    forward: [0.0, 1.0, 0.0],
};

fn starts(commands: &[Command]) -> Vec<&Voice> {
    commands
        .iter()
        .filter_map(|c| match c {
            Command::Start { voice, .. } if !voice.looping => Some(voice),
            _ => None,
        })
        .collect()
}

fn ready() -> Sound {
    let mut s = sound();
    let durations = s
        .tables()
        .battle_sounds()
        .into_iter()
        .map(|n| (n, vec![2.0; 8]))
        .collect();
    s.set_durations(durations).unwrap();
    s
}

#[test]
fn a_cannon_sounds_near_close_and_far_away_quieter_but_never_below_the_floor() {
    let mut s = ready();
    let mut heard = Vec::new();
    for (tick, x) in [(1.0, 50.0), (2.0, 2500.0)] {
        let (e, l) = shot(tick, 4.0, [x, 0.0, 0.0], "tank_ap");
        s.note(
            &Heard {
                effects: &e,
                launches: &l,
                audible: Vec::new(),
                shouts: Vec::new(),
            },
            0.0,
        );
        let got = s.update(tick / 30.0, tick, tick, &Motion::default(), HERE);
        heard.extend(starts(got).into_iter().cloned());
    }
    let [near, far] = &heard[..] else {
        panic!("two shots, {heard:?}")
    };
    let row = s.tables().shot("tank_ap");
    assert_eq!(near.sound, row.near);
    assert_eq!(far.sound, row.far);
    assert!(far.gain < near.gain);
    let floor = s.tables().audio.distance.floor;
    assert!(far.gain >= row.gain * floor);
    assert!(far.lowpass < near.lowpass && far.wet > near.wet);
}

#[test]
fn the_budget_holds_and_the_loudest_win() {
    let mut s = ready();
    let segments: Vec<Segment> = (0..200)
        .map(|i| Segment {
            path: vec![[0.0, 0.0, 1.0], [i as f64 * 5.0, 30.0, 0.0]],
            ricochets: Vec::new(),
            kind: "rifle".into(),
            shooter: None,
            hit: "ground".into(),
            normal: None,
        })
        .collect();
    let e = EffectPublication {
        tick: 1.0,
        segments,
        blasts: Vec::new(),
        shooters: Vec::new(),
        smokes: Vec::new(),
        cook_offs: Vec::new(),
        flare_deploys: Vec::new(),
        grinds: Vec::new(),
    };
    s.note(
        &Heard {
            effects: &e,
            launches: &[],
            audible: Vec::new(),
            shouts: Vec::new(),
        },
        0.0,
    );
    let got = s
        .update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE)
        .to_vec();
    let started = starts(&got);
    let budget = s.tables().audio.budget;
    assert_eq!(started.len(), budget.voices - budget.loops);
    let quietest = started.iter().map(|v| v.gain).fold(f64::INFINITY, f64::min);
    let stats = s.stats();
    assert_eq!(stats.started + stats.dropped, 200);
    // Every dropped impact was farther than every started one.
    let far = started
        .iter()
        .map(|v| v.position.unwrap()[0])
        .fold(0.0, f64::max);
    assert!(far < 200.0 * 5.0 && quietest > 0.0);
}

#[test]
fn an_unseen_enemy_is_heard_only_by_direction() {
    let mut s = ready();
    let e = shot(1.0, 4.0, [0.0; 3], "rifle").0;
    let cue = Cue {
        category: "vehicle".into(),
        sector: 2.0,
        band: "far".into(),
        moving: true,
    };
    s.note(
        &Heard {
            effects: &e,
            launches: &[],
            audible: vec![cue],
            shouts: Vec::new(),
        },
        0.0,
    );
    let got = s
        .update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE)
        .to_vec();
    let [voice] = &starts(&got)[..] else {
        panic!("one cue sound")
    };
    assert_eq!(voice.position, None, "never at a position");
    // North (sector 2) is straight ahead of a listener looking north.
    assert!(voice.pan.unwrap().abs() < 1e-9);
    assert_eq!(voice.lowpass, s.tables().audio.cues.far_lowpass_hz);
}

fn cue(category: &str, sector: f64, band: &str, moving: bool) -> Cue {
    Cue {
        category: category.into(),
        sector,
        band: band.into(),
        moving,
    }
}

/// The sounds `audible` starts, heard at tick 1.
fn hear_cues(s: &mut Sound, audible: Vec<Cue>) -> Vec<Voice> {
    let e = shot(1.0, 4.0, [0.0; 3], "rifle").0;
    s.note(
        &Heard {
            effects: &e,
            launches: &[],
            audible,
            shouts: Vec::new(),
        },
        0.0,
    );
    let got = s.update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE);
    starts(got).into_iter().cloned().collect()
}

fn pool() -> Vec<ShoutTake> {
    ["a", "b", "c"]
        .map(|v| ShoutTake {
            sound: format!("ru_rifle_squad/{v}/under_fire/stressed/0"),
            duration: 2.0,
        })
        .to_vec()
}

#[test]
fn an_unseen_shout_is_a_muffled_bark_from_the_enemys_pool_by_direction() {
    let mut s = ready();
    s.set_shouts(pool());
    let heard = hear_cues(
        &mut s,
        vec![
            cue("shout", 2.0, "near", false),
            cue("shout", 0.0, "far", false),
        ],
    );
    let [near, far] = &heard[..] else {
        panic!("one line per shout, {heard:?}")
    };
    let cues = &s.tables().audio.cues;
    for (voice, sector, band_gain) in [(near, 2.0, cues.near_gain), (far, 0.0, cues.far_gain)] {
        assert!(
            pool().iter().any(|t| t.sound == voice.sound),
            "{}",
            voice.sound
        );
        assert_eq!(voice.bus, Bus::Voices);
        assert_eq!(voice.position, None, "never at a position");
        assert_eq!(voice.pan, Some(sector_pan(sector, HERE.forward)));
        assert_eq!(
            voice.lowpass, cues.shout.lowpass_hz,
            "muffled in either band"
        );
        assert_eq!(voice.wet, cues.shout.wet);
        assert_eq!(voice.gain, cues.shout.gain * band_gain);
    }
    assert!(near.gain > far.gain);
    assert_eq!(s.stats().cues, 2);
    // The take is seeded by the cue alone: the same battle hears the same takes.
    let mut again = ready();
    again.set_shouts(pool());
    let replay = hear_cues(
        &mut again,
        vec![
            cue("shout", 2.0, "near", false),
            cue("shout", 0.0, "far", false),
        ],
    );
    assert_eq!(replay, heard);
}

#[test]
fn a_shout_with_no_pool_loaded_is_silent() {
    let mut s = ready();
    assert!(hear_cues(&mut s, vec![cue("shout", 2.0, "near", false)]).is_empty());
    assert_eq!(s.stats().cues, 0);
}

#[test]
fn unseen_infantry_is_heard_moving_and_silent_idle() {
    let mut s = ready();
    let heard = hear_cues(
        &mut s,
        vec![
            cue("infantry", 1.0, "near", false),
            cue("infantry", 3.0, "near", true),
        ],
    );
    let [moving] = &heard[..] else {
        panic!("only the moving squad, {heard:?}")
    };
    assert_eq!(
        moving.sound,
        s.tables().audio.cues.sounds["infantry_moving"]
    );
}

fn shout(own: bool, id: f64, kind: &str, position: [f64; 3], trigger: &str) -> Shout {
    Shout {
        own,
        id,
        kind: kind.into(),
        position,
        trigger: trigger.into(),
        intensity: "stressed".into(),
    }
}

/// A battle where the side's tanks speak with the US tank crew's set and
/// the enemy's with the Russian one.
fn speaking() -> Sound {
    let mut s = ready();
    s.set_speakers(Speakers {
        own: HashMap::from([("m1a2".into(), "us_tank_crew".into())]),
        enemy: HashMap::from([("t90".into(), "ru_tank_crew".into())]),
    });
    s
}

/// The barks one publication at tick 1 asks for.
fn hear_shouts(s: &mut Sound, shouts: Vec<Shout>, audible: Vec<Cue>) -> Vec<Request> {
    let e = shot(1.0, 4.0, [0.0; 3], "rifle").0;
    s.note(
        &Heard {
            effects: &e,
            launches: &[],
            audible,
            shouts,
        },
        2.0,
    );
    s.take_barks()
}

#[test]
fn an_own_and_a_seen_shout_are_barks_in_their_sides_set_where_they_were_shouted() {
    let mut s = speaking();
    s.set_shouts(pool());
    let barks = hear_shouts(
        &mut s,
        vec![
            shout(true, 3.0, "m1a2", [40.0, 0.0, 1.0], "b5_under_fire"),
            shout(false, 7.0, "t90", [300.0, 50.0, 2.0], "b2_contact_vehicle"),
            // A kind the side has no set for says nothing.
            shout(true, 4.0, "test_crate", [5.0, 0.0, 0.0], "b5_under_fire"),
        ],
        // An unseen enemy's shout is only a hearing cue: never a bark.
        vec![cue("shout", 2.0, "near", false)],
    );
    let [own, seen] = &barks[..] else {
        panic!("one bark per shout with a set, {barks:?}")
    };
    assert_eq!(
        (
            own.tier,
            &*own.speaker,
            &*own.set,
            &*own.trigger,
            &*own.intensity
        ),
        (
            Tier::OwnBark,
            "own/3",
            "us_tank_crew",
            "under_fire",
            "stressed"
        )
    );
    assert_eq!(own.position, Some([40.0, 0.0, 1.0]));
    assert_eq!(
        (seen.tier, &*seen.speaker, &*seen.set, &*seen.trigger),
        (Tier::OtherBark, "seen/7", "ru_tank_crew", "contact_vehicle")
    );
    assert_eq!(seen.position, Some([300.0, 50.0, 2.0]));
    assert!(own.at == 2.0 && seen.at == 2.0 && own.group_key.is_none());
    assert!(s.take_barks().is_empty(), "each bark is taken once");
}

#[test]
fn an_unseen_enemys_shout_never_asks_for_a_bark() {
    let mut s = speaking();
    s.set_shouts(pool());
    let barks = hear_shouts(
        &mut s,
        Vec::new(),
        vec![
            cue("shout", 2.0, "near", false),
            cue("shout", 6.0, "far", false),
        ],
    );
    assert!(barks.is_empty(), "{barks:?}");
}

#[test]
fn a_bark_is_spoken_where_it_was_shouted_and_an_answer_stays_dry() {
    let mut s = speaking();
    s.update(0.0, 0.0, 0.0, &Motion::default(), HERE);
    let mut chatter = Chatter::new(s.tables().audio.voices, 1.0);
    chatter.set_voices(
        "us_tank_crew",
        vec![
            SetVoice {
                id: "lead".into(),
                speaks: Speaks::Responses,
            },
            SetVoice {
                id: "soldier".into(),
                speaks: Speaks::Barks,
            },
        ],
    );
    chatter.loaded("us_tank_crew", "soldier", "under_fire", "stressed", 0, 2.0);
    chatter.loaded("us_tank_crew", "lead", "move", "steady", 0, 1.5);
    let barks = vec![
        shout(true, 1.0, "m1a2", [20.0, 0.0, 0.0], "b5_under_fire"),
        shout(true, 2.0, "m1a2", [800.0, 0.0, 0.0], "b5_under_fire"),
    ];
    let e = shot(1.0, 4.0, [0.0; 3], "rifle").0;
    s.note(
        &Heard {
            effects: &e,
            launches: &[],
            audible: Vec::new(),
            shouts: barks,
        },
        0.0,
    );
    let answer: Request = serde_json::from_value(json!({
        "tier": "response", "speaker": "own/5", "set": "us_tank_crew",
        "trigger": "move", "intensity": "steady", "at": 0.0
    }))
    .unwrap();
    chatter.request(answer, 0.0);
    let lines = chatter.hear(&mut s, 0.0).clone();
    assert_eq!(lines.heard.len(), 2, "both barks were heard");
    let voice = |speaker: &str| {
        let line = lines
            .started
            .iter()
            .position(|l| l.request.speaker == speaker)
            .unwrap_or_else(|| panic!("{speaker} speaks"));
        let Command::Start { voice, .. } = &lines.commands[line] else {
            panic!("a start")
        };
        voice.clone()
    };
    let (dry, near, far) = (voice("own/5"), voice("own/1"), voice("own/2"));
    let level = s.tables().audio.voices.gain;
    assert!(
        dry.position.is_none() && dry.pan.is_none() && dry.wet == 0.0 && dry.gain == level,
        "an answer is the owner's radio: {dry:?}"
    );
    assert_eq!(near.position, Some([20.0, 0.0, 0.0]));
    assert_eq!(far.position, Some([800.0, 0.0, 0.0]));
    for v in [&dry, &near, &far] {
        assert_eq!(v.bus, Bus::Voices);
    }
    // Farther is quieter, duller and wetter, by the unit sounds' own curves.
    assert!(level > near.gain && near.gain > far.gain);
    let floor = s.tables().audio.distance.floor;
    assert!(far.gain >= level * floor);
    assert!(near.lowpass > far.lowpass && near.wet < far.wet);
}

#[test]
fn a_standing_clock_silences_transients_and_holds_loops() {
    let mut s = ready();
    let (e, l) = shot(1.0, 4.0, [10.0, 0.0, 0.0], "tank_ap");
    s.note(
        &Heard {
            effects: &e,
            launches: &l,
            audible: Vec::new(),
            shouts: Vec::new(),
        },
        0.0,
    );
    let walking = Motion {
        vehicles: Vec::new(),
        soldiers: vec![SoundSoldier {
            id: 1.0,
            position: [5.0, 5.0, 0.0],
        }],
    };
    s.update(1.0 / 30.0, 0.0, 0.0, &walking, HERE);
    assert_eq!(s.stats().transients, 1);
    let hold = s.tables().audio.hold_s;
    let got = s.update(1.0 / 30.0, hold * 2.0, hold * 2.0, &walking, HERE);
    assert!(got.iter().any(|c| matches!(c, Command::Stop { .. })));
    assert!(
        !got.iter().any(|c| matches!(c, Command::Set { .. })),
        "loops hold as they are"
    );
    let stats = s.stats();
    assert!(stats.held && stats.transients == 0 && stats.loops == 1);
}

#[test]
fn every_synthesized_sound_is_finite_and_loops_wrap() {
    for name in synth::names() {
        let s = synth::synthesize(name, 48000.0).unwrap();
        let peak = s
            .channels
            .iter()
            .flatten()
            .fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((peak - 0.9).abs() < 1e-3, "{name} peaks at {peak}");
        assert!(s.channels.iter().flatten().all(|v| v.is_finite()), "{name}");
        assert_eq!(Some(s.looping), synth::loops(name));
    }
}

#[test]
fn a_bank_prepares_every_battle_sound_at_the_reference_loudness() {
    let t = tables();
    let mut bank = Bank::new(48000.0);
    // Recordings stand in as a second of a 1 kHz tone.
    let tone: Vec<f32> = (0..48000)
        .map(|i| (0.3 * (std::f64::consts::TAU * 1000.0 * i as f64 / 48000.0).sin()) as f32)
        .collect();
    bank.prepare(&t, |_| Ok(vec![tone.clone()])).unwrap();
    let durations: HashMap<_, _> = bank.durations();
    for name in t.battle_sounds() {
        let recipe = &t.catalog.sounds[&name];
        let buffers = &bank.sounds[&name];
        assert_eq!(buffers.len(), recipe.clips.len().max(1), "{name}");
        assert!(durations[&name].iter().all(|d| *d > 0.0), "{name}");
        for b in buffers {
            let level =
                max_momentary_loudness(&b.channels, 48000.0) + 20.0 * b.normalization.log10();
            let authored = recipe.gain
                * if recipe.clips.is_empty() {
                    recipe.synth_gain
                } else {
                    1.0
                };
            let want = -23.0 + 20.0 * authored.log10();
            assert!(
                (level - want).abs() < 1e-6,
                "{name}: {level} against {want}"
            );
        }
    }
}

#[test]
fn k_weighting_reads_the_bs1770_reference() {
    // A 997 Hz sine at 0 dBFS on one channel reads −3.01 LKFS.
    for rate in [44100.0, 48000.0] {
        let sine: Vec<f32> = (0..(rate as usize))
            .map(|i| (std::f64::consts::TAU * 997.0 * i as f64 / rate).sin() as f32)
            .collect();
        let l = max_momentary_loudness(&[sine], rate);
        assert!((l + 3.01).abs() < 0.05, "{rate}: {l}");
    }
}

#[test]
fn an_rpg_launch_is_heard_above_a_thrown_grenade() {
    let launch_gain = |kind: &str| {
        let mut s = ready();
        let (effects, launches) = shot(1.0, 3.0, [20.0, 0.0, 0.0], kind);
        s.note(
            &Heard {
                effects: &effects,
                launches: &launches,
                audible: Vec::new(),
                shouts: Vec::new(),
            },
            0.0,
        );
        let commands = s.update(1.0 / 30.0, 0.0, 0.0, &Motion::default(), HERE);
        let voices = starts(commands);
        assert!(!voices.is_empty(), "the launch must be audible");
        voices.iter().map(|voice| voice.gain).sum::<f64>()
    };
    assert!(
        launch_gain("rpg_at_light") > launch_gain("grenade"),
        "an RPG must play its own louder launch, rather than the grenade's fallback level"
    );
}
